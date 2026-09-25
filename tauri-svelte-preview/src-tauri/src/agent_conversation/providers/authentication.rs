use super::{process::SidecarEnvironment, AcpClient};
use crate::agent_conversation::protocol::{AgentConversationProvider, AgentProviderManifest};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;

type Attempts = Arc<Mutex<HashMap<(String, u64), watch::Sender<bool>>>>;

#[derive(Clone, Default)]
pub(crate) struct Authentications(Attempts);

pub(crate) struct AuthenticationAttempt {
    attempts: Attempts,
    key: (String, u64),
    cancelled: watch::Receiver<bool>,
}

impl Authentications {
    pub fn begin(&self, owned_id: &str, generation: u64) -> Result<AuthenticationAttempt, String> {
        let mut attempts = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !attempts.is_empty() {
            return Err("Antigravity sign-in is already in progress".into());
        }
        let key = (owned_id.to_string(), generation);
        let (sender, cancelled) = watch::channel(false);
        attempts.insert(key.clone(), sender);
        Ok(AuthenticationAttempt {
            attempts: self.0.clone(),
            key,
            cancelled,
        })
    }

    pub fn cancel(&self, owned_id: &str, generation: u64) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(owned_id.to_string(), generation))
            .is_some_and(|sender| sender.send(true).is_ok())
    }

    pub fn pending(&self) -> bool {
        !self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    }
}

impl AuthenticationAttempt {
    pub fn was_cancelled(&self) -> bool {
        *self.cancelled.borrow()
    }

    pub async fn authenticate(
        &mut self,
        manifest: &AgentProviderManifest,
        cwd: &Path,
    ) -> Result<(), String> {
        if self.was_cancelled() {
            return Err("Antigravity sign-in cancelled".into());
        }
        let mut client = AcpClient::spawn_with_environment(
            manifest,
            cwd,
            &self.key.0,
            &SidecarEnvironment::default(),
        )
        .map_err(|error| error.to_string())?;
        let result = tokio::select! {
            biased;
            _ = self.cancelled.changed() => Err("Antigravity sign-in cancelled".to_string()),
            result = tokio::time::timeout(Duration::from_secs(300), client.authenticate_personal()) => {
                match result {
                    Ok(result) => result.map_err(|error| error.to_string()),
                    Err(_) => Err("Antigravity sign-in timed out. Send again to retry.".into()),
                }
            }
        };
        // No pool membership or session context is retained by an auth attempt.
        let _ = client.detach().await;
        result
    }
}

impl Drop for AuthenticationAttempt {
    fn drop(&mut self) {
        self.attempts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.key);
    }
}

pub(crate) fn required(provider: AgentConversationProvider, error: &str) -> bool {
    provider == AgentConversationProvider::Antigravity
        && error
            .split_once("acp-error: ")
            .and_then(|(_, json)| serde_json::from_str::<serde_json::Value>(json).ok())
            .and_then(|error| error.get("code").and_then(serde_json::Value::as_i64))
            == Some(-32000)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_generation_scoped_and_attempts_are_released() {
        let authentications = Authentications::default();
        let attempt = authentications.begin("session", 7).unwrap();
        assert!(authentications.pending());
        assert!(authentications.begin("other", 1).is_err());
        assert!(!authentications.cancel("session", 6));
        assert!(!attempt.was_cancelled());
        assert!(authentications.cancel("session", 7));
        assert!(attempt.was_cancelled());
        drop(attempt);
        assert!(!authentications.pending());
        assert!(authentications.begin("other", 1).is_ok());
    }
    #[tokio::test]
    async fn authentication_uses_advertised_personal_method_and_cleans_process() {
        let root = std::env::temp_dir().join(format!("mcb-auth-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let log = root.join("frames");
        let manifest = super::super::acp_client::tests::fixture_manifest_named(&log, "auth");
        let authentications = Authentications::default();
        let mut attempt = authentications.begin("session", 1).unwrap();
        attempt.authenticate(&manifest, &root).await.unwrap();
        let frames = std::fs::read_to_string(&log).unwrap();
        assert!(frames.contains("authenticate"));
        assert!(frames.contains("oauth-personal"));
        assert!(!frames.contains("session/new"));
        drop(attempt);
        assert!(!authentications.pending());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn stop_cancels_waiting_authentication_without_retaining_attempt() {
        let root = std::env::temp_dir().join(format!("mcb-auth-cancel-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let log = root.join("frames");
        let manifest = super::super::acp_client::tests::fixture_manifest_named(&log, "auth_wait");
        let authentications = Authentications::default();
        let mut attempt = authentications.begin("session", 1).unwrap();
        let task_root = root.clone();
        let task = tokio::spawn(async move { attempt.authenticate(&manifest, &task_root).await });
        tokio::time::timeout(Duration::from_secs(5), async {
            while !std::fs::read_to_string(&log)
                .unwrap_or_default()
                .contains("authenticate")
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(authentications.cancel("session", 1));
        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(!authentications.pending());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_official_auth_required_triggers_personal_sign_in() {
        let error = "acp-error: {\"code\":-32000,\"message\":\"Authentication required\"}";
        assert!(required(AgentConversationProvider::Antigravity, error));
        assert!(!required(AgentConversationProvider::Claude, error));
        assert!(!required(
            AgentConversationProvider::Antigravity,
            "acp-error: permission denied"
        ));
        assert!(!required(
            AgentConversationProvider::Antigravity,
            "acp-error: {\"code\":-32603}"
        ));
    }
}
