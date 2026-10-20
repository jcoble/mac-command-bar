//! Regression pins for the session list the rail reads (TSK-1167).
use mcb_core::session_store::{SessionRow, SessionStore};

fn session(owned_id: &str, activity_ms: i64) -> SessionRow {
    SessionRow {
        owned_id: owned_id.to_owned(),
        native_session_id: None,
        provider: "codex".to_owned(),
        model: None,
        effort: None,
        cwd: "/work/app".to_owned(),
        worktree: None,
        branch: None,
        title: None,
        title_source: None,
        project: None,
        project_id: None,
        state: "idle".to_owned(),
        suspended: false,
        created_at_ms: 1,
        last_activity_at_ms: activity_ms,
        extra_json: "{}".to_owned(),
    }
}

/// Sessions with the same last activity list in session id order, whatever
/// order they were written in, so the rail keeps them in place across a
/// restart. Protects 38711d55c (the rail's own tie-break) at its source.
#[test]
fn the_session_list_breaks_activity_ties_by_session_id() {
    let listed = |writes: &[(&str, i64)]| {
        let store = SessionStore::open_in_memory().unwrap();
        for (owned_id, activity) in writes {
            store.upsert_session(&session(owned_id, *activity)).unwrap();
        }
        store.list_sessions().unwrap().into_iter().map(|row| row.owned_id).collect::<Vec<_>>()
    };
    let expected = ["newest", "a-tied", "b-tied", "c-tied", "oldest"];
    assert_eq!(listed(&[("c-tied", 5), ("oldest", 1), ("a-tied", 5), ("newest", 9), ("b-tied", 5)]), expected);
    assert_eq!(listed(&[("b-tied", 5), ("newest", 9), ("a-tied", 5), ("oldest", 1), ("c-tied", 5)]), expected);
}
