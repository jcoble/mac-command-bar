use mcb_core::session_store::{SessionRow, SessionStore};

#[test]
fn phone_sessions_put_pinned_first_and_say_which_are_done() {
    let store = SessionStore::open_in_memory().unwrap();
    for (owned_id, activity, extra) in [
        ("old", 100, r#"{"railMeta":{"pinnedAt":"2026-10-20T10:00:00.000Z"}}"#),
        ("mid", 200, r#"{"railMeta":{"completedAt":"2026-10-20T11:00:00.000Z"}}"#),
        ("new", 300, "{}"),
    ] {
        store.upsert_session(&SessionRow { owned_id: owned_id.into(), native_session_id: None,
            provider: "codex".into(), model: None, effort: None, cwd: "/work/assembly".into(), worktree: None, branch: None,
            title: Some(owned_id.into()), title_source: None, project: Some("assembly".into()), project_id: None,
            state: "ready".into(), suspended: false, created_at_ms: 1, last_activity_at_ms: activity, extra_json: extra.into() }).unwrap();
    }

    let sessions = store.list_phone_sessions().unwrap();
    let ids: Vec<&str> = sessions.iter().map(|(entry, _, _, _)| entry.owned_id.as_str()).collect();
    assert_eq!(ids, ["old", "new", "mid"], "pinned first, then newest activity");
    assert_eq!(sessions.iter().map(|(_, _, pinned, _)| *pinned).collect::<Vec<_>>(), [true, false, false]);
    assert_eq!(sessions.iter().map(|(_, _, _, done)| *done).collect::<Vec<_>>(), [false, false, true]);
}
