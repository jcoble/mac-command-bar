use super::{Result, SessionStore, StoreError};
use rusqlite::OptionalExtension;

impl SessionStore {
    /// The root folders of this Mac's projects.
    pub fn local_project_roots(&self) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT root_path FROM projects WHERE machine = 'local' ORDER BY created_at, id")
            .map_err(|error| StoreError::sqlite("could not prepare the project roots", error))?;
        let rows = statement
            .query_map([], |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not list the project roots", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not list the project roots", error))
    }

    /// The root folder of the project added on this machine whose folder is `named`, or else
    /// the only one titled `named`. A title shared by several projects finds none.
    pub fn local_project_root(&self, named: &str) -> Result<Option<String>> {
        self.lock()?.query_row(
            "SELECT root_path FROM projects WHERE machine = 'local' AND root_path = ?1
             UNION ALL
             SELECT MAX(root_path) FROM projects WHERE machine = 'local' AND title = ?1 HAVING COUNT(*) = 1
             LIMIT 1",
            [named], |row| row.get(0),
        ).optional().map_err(|error| StoreError::sqlite("could not find the project", error))
    }
}

#[cfg(test)]
mod tests {
    use super::super::ProjectRow;
    use super::*;

    fn add(store: &SessionStore, id: &str, machine: &str, root: &str, title: &str) {
        store.insert_or_get_project(&ProjectRow {
            id: id.into(), machine: machine.into(), root_path: root.into(), title: title.into(), repo_key: String::new(),
            created_at_ms: 0, group_key: String::new(), pinned_at_ms: None, last_used_ms: None,
        }).unwrap();
    }

    #[test]
    fn a_local_project_is_found_by_its_folder_or_its_only_title() {
        let store = SessionStore::open_in_memory().unwrap();
        add(&store, "a", "local", "/work/app", "app");
        add(&store, "b", "local", "/work/twin-1", "twin");
        add(&store, "c", "local", "/work/twin-2", "twin");
        add(&store, "d", "workbox", "/work/remote", "remote");
        let found = |named| store.local_project_root(named).unwrap();
        assert_eq!(found("app").as_deref(), Some("/work/app"));
        assert_eq!(found("/work/app").as_deref(), Some("/work/app"));
        assert_eq!(found("/work/twin-2").as_deref(), Some("/work/twin-2"));
        // A shared title, another machine's project and an unknown name find nothing.
        for named in ["twin", "remote", "/work/remote", "/work", "missing"] {
            assert_eq!(found(named), None, "{named}");
        }
    }
}
