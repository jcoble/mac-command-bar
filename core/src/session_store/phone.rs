use super::{PhoneDevice, Result, SessionStore, StoreError};
use rusqlite::params;

impl SessionStore {
    /// Names this machine for paired phones, creating its id from `new_id` the
    /// first time. Returns the machine's `(id, name)`.
    pub fn name_phone_machine(&self, new_id: &str, name: &str) -> Result<(String, String)> {
        let connection = self.lock_write()?;
        connection.execute(
            "INSERT INTO phone_machine (id, name) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 FROM phone_machine)",
            params![new_id, name],
        ).map_err(|error| StoreError::sqlite("could not create the phone machine id", error))?;
        connection.query_row("UPDATE phone_machine SET name = ? RETURNING id, name", [name],
            |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|error| StoreError::sqlite("could not name the phone machine", error))
    }

    /// This machine's `(id, name)` for phones, once it has been named.
    pub fn phone_machine(&self) -> Result<Option<(String, String)>> {
        use rusqlite::OptionalExtension;
        self.lock()?.query_row("SELECT id, name FROM phone_machine", [], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional().map_err(|error| StoreError::sqlite("could not read the phone machine", error))
    }

    pub fn add_phone_device(&self, id: &str, name: &str, token_hash: &str, created_at: i64) -> Result<()> {
        self.lock_write()?.execute(
            "INSERT INTO phone_devices (id, name, token_hash, created_at) VALUES (?, ?, ?, ?)",
            params![id, name, token_hash, created_at],
        ).map_err(|error| StoreError::sqlite("could not save the paired phone", error))?;
        Ok(())
    }

    /// The id of the non-revoked phone holding `token_hash`, marked as seen at `now`. An unknown or
    /// revoked token is a read only; only a real phone's request writes.
    pub fn touch_phone_device(&self, token_hash: &str, now: i64) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        let found: Option<String> = self.lock()?.query_row(
            "SELECT id FROM phone_devices WHERE token_hash = ?1 AND revoked_at IS NULL", [token_hash], |row| row.get(0),
        ).optional().map_err(|error| StoreError::sqlite("could not check the phone token", error))?;
        let Some(id) = found else { return Ok(None) };
        self.lock_write()?.query_row(
            "UPDATE phone_devices SET last_seen_at = ?2 WHERE id = ?1 AND revoked_at IS NULL RETURNING id",
            params![id, now], |row| row.get(0),
        ).optional().map_err(|error| StoreError::sqlite("could not mark the phone seen", error))
    }

    /// Paired phones that have not been removed, newest first.
    pub fn phone_devices(&self) -> Result<Vec<PhoneDevice>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, name, created_at, last_seen_at FROM phone_devices
             WHERE revoked_at IS NULL ORDER BY created_at DESC, id",
        ).map_err(|error| StoreError::sqlite("could not list paired phones", error))?;
        let rows = statement.query_map([], |row| Ok(PhoneDevice {
            id: row.get(0)?, name: row.get(1)?, created_at: row.get(2)?, last_seen_at: row.get(3)?,
        })).map_err(|error| StoreError::sqlite("could not list paired phones", error))?;
        rows.collect::<rusqlite::Result<_>>().map_err(|error| StoreError::sqlite("could not read a paired phone", error))
    }

    /// Gives `fcm_token` to this phone alone: the one statement also clears it from every other row.
    pub fn set_phone_push_token(&self, device_id: &str, fcm_token: &str) -> Result<()> {
        self.lock_write()?.execute(
            "UPDATE phone_devices SET fcm_token = CASE WHEN id = ?1 THEN ?2 ELSE NULL END WHERE id = ?1 OR fcm_token = ?2",
            params![device_id, fcm_token],
        ).map_err(|error| StoreError::sqlite("could not save the phone's push token", error))?;
        Ok(())
    }

    /// The push tokens of phones that have not been removed.
    pub fn phone_push_tokens(&self) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare("SELECT fcm_token FROM phone_devices WHERE revoked_at IS NULL AND fcm_token IS NOT NULL")
            .map_err(|error| StoreError::sqlite("could not list push tokens", error))?;
        let rows = statement.query_map([], |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not list push tokens", error))?;
        rows.collect::<rusqlite::Result<_>>().map_err(|error| StoreError::sqlite("could not read a push token", error))
    }

    /// Forgets a push token that Google no longer accepts.
    pub fn clear_phone_push_token(&self, fcm_token: &str) -> Result<()> {
        self.lock_write()?.execute("UPDATE phone_devices SET fcm_token = NULL WHERE fcm_token = ?1", [fcm_token])
            .map_err(|error| StoreError::sqlite("could not clear the push token", error))?;
        Ok(())
    }

    /// Removes a paired phone: its token stops working and it gets no more pushes.
    pub fn revoke_phone_device(&self, id: &str, now: i64) -> Result<bool> {
        let changed = self.lock_write()?.execute(
            "UPDATE phone_devices SET revoked_at = ?2, fcm_token = NULL WHERE id = ?1 AND revoked_at IS NULL",
            params![id, now],
        ).map_err(|error| StoreError::sqlite("could not remove the paired phone", error))?;
        Ok(changed > 0)
    }
}
