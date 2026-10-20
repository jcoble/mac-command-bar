use super::{MessageKind, Receipt};
use crate::session_store::SessionStore;

fn empty_group(store: &SessionStore) -> String {
    store
        .create_group("build", None, &[])
        .expect("create group")
}

fn append(store: &SessionStore, group: &str, to: &str, body: &str) -> super::Envelope {
    store
        .append_message(group, "orchestrator", to, MessageKind::Message, body)
        .expect("append message")
}

#[test]
fn broker_delivers_to_session_outside_any_group() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute("INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra)
        VALUES ('target','codex','/work','ready',0,1,1,'{}')", []).unwrap();
    let message = store.append_message("", "sender", "target", MessageKind::Message, "hello").unwrap();
    assert_eq!(store.pending_for("target").unwrap(), Some(message.clone()));
    store.set_receipt(&message.id, Receipt::Delivered).unwrap();
    assert!(store.pending_for("target").unwrap().is_none());
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra)
        VALUES ('worker','claude','/work','waiting-input',0,1,1,'{}');
        INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json)
        VALUES ('e','run-1','workflow.node.started','1',1,
        '{"ownedId":"worker","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}');"#).unwrap();
    assert_eq!(store.resolve_session_name("tsk-1416/build").unwrap(), "worker");
    let message = store.append_message("", "worker", "target", MessageKind::Message, "reply to me").unwrap();
    assert!(message.sender_name.ends_with(" · tsk-1416/build"), "{}", message.sender_name);
    assert_eq!(message.run_id.as_deref(), Some("run-1"));
    // A reply to the worker belongs to the same run, so the run's Messages view shows it.
    let reply = store.append_message("", "target", "worker", MessageKind::Message, "done").unwrap();
    assert_eq!(reply.run_id.as_deref(), Some("run-1"));
    // A workflow's own notice to the session that started it belongs to that run.
    let notice = store.append_message("", "workflow:run-9", "target", MessageKind::Message, "waiting").unwrap();
    assert_eq!(notice.run_id.as_deref(), Some("run-9"));
}

#[test]
fn session_list_says_where_each_session_came_from() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra) VALUES
        ('parent','codex','/work','ready',0,1,1,'{}'),
        ('helper','claude','/work','ready',0,1,1,'{"helperOf":"parent"}'),
        ('worker','codex','/work','ready',0,1,1,'{}');
        INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json) VALUES
        ('e','run-1','workflow.node.started','1',1,
         '{"ownedId":"worker","nodeId":"build","workflowPayload":{"run":{"definition":{"name":"Fix and review"},"inputSnapshot":{"task":{"key":"TSK-9"},"started_by":"parent"}}}}');"#).unwrap();
    let listed = store.list_sessions_with_handles().unwrap();
    let lineage = |id: &str| listed.iter().find(|entry| entry.row.owned_id == id)
        .map(|entry| (entry.helper_of.clone(), entry.run_id.clone(), entry.run_label.clone())).unwrap();
    assert_eq!(lineage("parent"), (None, None, None));
    assert_eq!(lineage("helper"), (Some("parent".into()), None, None));
    assert_eq!(lineage("worker"), (None, Some("run-1".into()), Some("TSK-9 · Fix and review".into())));
    // A worker names the session that started its run; any other row names none.
    let lead = |id: &str| listed.iter().find(|entry| entry.row.owned_id == id).unwrap().run_lead.clone();
    assert_eq!(lead("worker").as_deref(), Some("parent"));
    assert_eq!((lead("parent"), lead("helper")), (None, None));
}

#[test]
fn every_session_keeps_a_handle_that_outlives_renames() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch("INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title) VALUES
        ('a','codex','/work','ready',0,1,1,'{}','Lane A'), ('b','claude','/work','ready',0,2,2,'{}','Lane B');").unwrap();
    let entries = store.list_directory().unwrap();
    let handle = entries.iter().find(|entry| entry.owned_id == "a").unwrap().handle.clone();
    assert!(handle.split('-').count() >= 2 && handle != entries[1].handle, "{handle}");
    store.lock_write().unwrap().execute("UPDATE sessions SET title='Renamed' WHERE owned_id='a'", []).unwrap();
    assert_eq!(store.resolve_session_name(&handle).unwrap(), "a");
    assert_eq!(store.resolve_session_name("a").unwrap(), "a");
    let message = store.append_message("", "a", "b", MessageKind::Message, "hi").unwrap();
    assert_eq!(message.sender_name, format!("{handle} · Renamed"));
    // A reply addressed to the whole label reaches the sender.
    assert_eq!(store.resolve_session_name(&message.sender_name).unwrap(), "a");
}

#[test]
fn a_message_from_another_machine_keeps_its_sender_label() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch("INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title) VALUES
        ('here','codex','/work','ready',0,1,1,'{}','Here');").unwrap();
    let message = store.append_message_as("", "elsewhere", "here", MessageKind::Message, "hi",
        Some("brisk-heron@workbox-test · There")).unwrap();
    assert_eq!(message.sender_name, "brisk-heron@workbox-test · There");
    assert_eq!(store.message_status(&message.id, "elsewhere").unwrap(), "queued");
}

#[test]
fn directory_shows_the_session_project_title() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch("INSERT INTO projects
        (id,machine,root_path,title,repo_key,created_at) VALUES ('p1','local','/work','demo-repo','k',1);
        INSERT INTO sessions (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,project_id)
        VALUES ('s1','codex','/work','ready',0,1,1,'{}','p1');").unwrap();
    assert_eq!(store.list_directory().unwrap()[0].project.as_deref(), Some("demo-repo"));
}

#[test]
fn retried_worker_name_points_at_the_newest_session() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra) VALUES
        ('first-try','codex','/work','closed',0,1,1,'{}'), ('second-try','codex','/work','working',0,2,2,'{}');
        INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json) VALUES
        ('e1','run-1','workflow.node.started','1',1,'{"ownedId":"first-try","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}'),
        ('e2','run-1','workflow.node.started','2',2,'{"ownedId":"second-try","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}');"#).unwrap();
    assert_eq!(store.resolve_session_name("tsk-1416/build").unwrap(), "second-try");
    assert_eq!(store.resolve_session_name("tsk-1416/build [first-try]").unwrap(), "first-try");
    let reply = store.append_message("", "second-try", "first-try", MessageKind::Message, "hi").unwrap();
    assert!(reply.sender_name.ends_with(" · tsk-1416/build"), "{}", reply.sender_name);
    assert_eq!(store.resolve_session_name(&reply.sender_name).unwrap(), "second-try");
}

#[test]
fn orchestrator_comes_from_setting_not_title() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
        (owned_id,provider,cwd,title,state,suspended,created_at,last_activity_at,extra) VALUES
        ('named','codex','/work','orchestrator','ready',0,1,1,'{}'),
        ('marked','codex','/work','Controller','ready',0,2,2,'{}');"#).unwrap();
    let marked = |store: &SessionStore| store.list_directory().unwrap().into_iter()
        .filter(|entry| entry.orchestrator).map(|entry| entry.owned_id).collect::<Vec<_>>();
    assert!(marked(&store).is_empty(), "a title does not make an orchestrator");
    store.set_main_orchestrator(Some("marked")).unwrap();
    assert_eq!(marked(&store), ["marked"]);
    assert_eq!(store.main_orchestrator().unwrap().map(|(id, _)| id).as_deref(), Some("marked"));
    store.set_main_orchestrator(None).unwrap();
    assert!(marked(&store).is_empty());
    assert_eq!(store.main_orchestrator().unwrap(), None);
}

#[test]
fn directory_lookup_uses_owned_event_index() {
    let store = SessionStore::open_in_memory().unwrap();
    let connection = store.lock().unwrap();
    let mut statement = connection.prepare("EXPLAIN QUERY PLAN SELECT name FROM session_directory WHERE owned_id=?").unwrap();
    let details = statement.query_map(["target"], |row| row.get::<_, String>(3)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
    assert!(details.contains("orchestration_events_owned_idx"), "{details}");
}

#[test]
fn list_messages_filters_and_pages_in_sql() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch("INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title,handle) VALUES
        ('b','codex','/work','ready',0,1,1,'{}','Lane B','calm-otter');
        INSERT INTO workflow_messages (id,from_owned_id,to_owned_id,sender_name,run_id,kind,body,receipt,created_at_ms,updated_at_ms) VALUES
        ('m1','a','b','brisk-heron · A','run-1','message','one','delivered',10,10),
        ('m2','a','b','brisk-heron · A',NULL,'message','two','queued',20,20),
        ('m3','c','gone','x · C','run-2','message','three','failed',20,20),
        ('m4','b','a','calm-otter · Lane B','run-1','message','four','delivered',30,30),
        ('m5','b','attempt-1','calm-otter · Lane B','run-1','review','{}','delivered',40,40),
        ('m6','a','b','brisk-heron · A',NULL,'system','joined','delivered',40,40);").unwrap();
    // Review reports and group bookkeeping are not agent messages, so the owner's view leaves them out.
    use super::{TrafficMessage, TrafficQuery, traffic_sql};
    use rusqlite::params;
    let ids = |page: Vec<TrafficMessage>| page.into_iter().map(|message| message.id).collect::<Vec<_>>();
    assert_eq!(ids(store.list_messages(&TrafficQuery { limit: 10, ..Default::default() }).unwrap()), ["m4", "m3", "m2", "m1"]);
    assert_eq!(ids(store.list_messages(&TrafficQuery { run_id: Some("run-1"), limit: 10, ..Default::default() }).unwrap()), ["m4", "m1"]);
    assert_eq!(ids(store.list_messages(&TrafficQuery { session: Some("b"), limit: 10, ..Default::default() }).unwrap()), ["m4", "m2", "m1"]);
    // Paging resumes after the last row shown, even between two rows of the same time.
    let first = store.list_messages(&TrafficQuery { limit: 2, ..Default::default() }).unwrap();
    assert_eq!(ids(first.clone()), ["m4", "m3"]);
    let last = first.last().unwrap();
    assert_eq!(ids(store.list_messages(&TrafficQuery { before: Some((last.created_at_ms, &last.id)), limit: 10, ..Default::default() }).unwrap()), ["m2", "m1"]);
    let messages = store.list_messages(&TrafficQuery { limit: 10, ..Default::default() }).unwrap();
    assert_eq!(messages[2].receiver, "calm-otter · Lane B");
    assert_eq!(messages[1].receiver, "gone");
    let connection = store.lock().unwrap();
    let mut statement = connection.prepare(&format!("EXPLAIN QUERY PLAN {}", traffic_sql("time"))).unwrap();
    let details = statement.query_map(params![None::<String>, None::<String>, None::<String>, None::<String>, None::<String>, None::<i64>, None::<String>, None::<String>, 10], |row| row.get::<_, String>(3)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
    assert!(details.contains("idx_workflow_messages_created"), "{details}");
    assert!(!details.contains("TEMP B-TREE"), "{details}");
}

#[test]
fn list_messages_session_scope_includes_runs_it_started() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO orchestration_events
        (id,run_id,kind,timestamp,payload_json) VALUES ('e','run-b','run','now',
        '{"workflowPayload":{"run":{"inputSnapshot":{"started_by":"owner"}}}}');
        INSERT INTO workflow_messages (id,from_owned_id,to_owned_id,sender_name,run_id,kind,body,receipt,created_at_ms,updated_at_ms)
        VALUES ('m','worker-a','worker-b','Worker','run-b','message','hello','queued',10,10);"#).unwrap();
    assert_eq!(store.list_messages(&super::TrafficQuery { session: Some("owner"), limit: 10, ..Default::default() }).unwrap().len(), 1);
    assert!(store.list_messages(&super::TrafficQuery { session: Some("unrelated"), limit: 10, ..Default::default() }).unwrap().is_empty());
    let connection = store.lock().unwrap();
    let plan: String = connection.query_row("EXPLAIN QUERY PLAN SELECT run_id FROM orchestration_events WHERE json_extract(payload_json,'$.workflowPayload.run.inputSnapshot.started_by')='owner'", [], |r| r.get(3)).unwrap();
    assert!(plan.contains("orchestration_events_started_by_idx"), "{plan}");
}

#[test]
fn list_messages_search_sort_status_in_sql() {
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch("INSERT INTO workflow_messages
        (id,from_owned_id,to_owned_id,sender_name,kind,body,receipt,created_at_ms,updated_at_ms) VALUES
        ('a','a','Receiver','Zebra','message','100% done','queued',30,30),
        ('b','b','Other','Alpha','message','ordinary','delivered',20,20),
        ('c','c','Other','Alpha','message','1000 done','queued',10,10);").unwrap();
    let query = |search| super::TrafficQuery { search, limit: 10, ..Default::default() };
    for search in ["Zebra", "Receiver", "100%"] {
        assert_eq!(store.list_messages(&query(Some(search))).unwrap()[0].id, "a");
        assert_eq!(store.list_messages(&query(Some(search))).unwrap().len(), 1);
    }
    let rows = store.list_messages(&super::TrafficQuery { sort: "sender", limit: 10, ..Default::default() }).unwrap();
    assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["b", "c", "a"]);
    for (sort, key) in [("sender", "Alpha"), ("receiver", "Other")] {
        let rows = store.list_messages(&super::TrafficQuery { sort, before_key: Some(key), before: Some((20,"b")), status: Some("queued"), limit: 10, ..Default::default() }).unwrap();
        assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["c", "a"]);
    }
}

#[test]
fn count_messages_after() {
    let store = SessionStore::open_in_memory().unwrap();
    let message = store.append_message("", "sender", "target", MessageKind::Message, "hello").unwrap();
    assert_eq!(store.count_messages("target", message.created_at_ms - 1, None).unwrap(), 1);
    assert_eq!(store.count_messages("sender", message.created_at_ms, None).unwrap(), 0);
    assert_eq!(store.count_messages("other", 0, None).unwrap(), 0);
}

#[test]
fn message_listener_fires_on_append_and_receipt() {
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    let store = Arc::new(SessionStore::open_in_memory().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let (observed, count) = (store.clone(), calls.clone());
    let owner = std::thread::current().id();
    super::on_messages_changed(move || {
        // Taking the same database lock proves notification happens after release.
        if std::thread::current().id() == owner && observed.count_messages("listener-target", 0, None).unwrap() > 0 { count.fetch_add(1, Ordering::SeqCst); }
    });
    let message = store.append_message("", "sender", "listener-target", MessageKind::Message, "hello").unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    store.set_receipt(&message.id, Receipt::Queued).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    store.set_receipt(&message.id, Receipt::Delivered).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(store.set_receipt(&message.id, Receipt::Failed).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn broker_queue_queries_use_target_index() {
    let store = SessionStore::open_in_memory().unwrap();
    let connection = store.lock().unwrap();
    let mut statement = connection.prepare("EXPLAIN QUERY PLAN SELECT id FROM workflow_messages
        WHERE to_owned_id=? AND receipt='queued' ORDER BY created_at_ms,id LIMIT 1").unwrap();
    let details = statement.query_map(["target"], |row| row.get::<_, String>(3)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
    assert!(details.contains("idx_workflow_messages_target"), "{details}");
    assert!(!details.contains("TEMP B-TREE"), "{details}");
}

#[test]
fn append_starts_queued_and_events_return_in_order() {
    let store = SessionStore::open_in_memory().expect("open store");
    let members = vec!["worker-a".to_owned(), "worker-b".to_owned()];
    let group = store
        .create_group("build", Some("orchestrator"), &members)
        .expect("create group");
    let first = append(&store, &group, "worker-a", "first");
    let second = store
        .append_message(
            &group,
            "worker-a",
            "orchestrator",
            MessageKind::Status,
            "second",
        )
        .expect("append second message");

    assert_eq!(
        (first.receipt, second.receipt),
        (Receipt::Queued, Receipt::Queued)
    );
    let bodies: Vec<_> = store
        .events_for(&group, None)
        .expect("list events")
        .into_iter()
        .map(|event| event.body)
        .collect();
    assert_eq!(
        bodies,
        vec!["member:worker-a", "member:worker-b", "first", "second"]
    );
    assert_eq!(store.list_open_groups().expect("list groups")[0].3, members);
}

#[test]
fn terminal_receipts_refuse_change() {
    let store = SessionStore::open_in_memory().expect("open store");
    let message = append(&store, &empty_group(&store), "worker-a", "choose");
    store
        .set_receipt(&message.id, Receipt::Delivered)
        .expect("deliver message");
    assert!(store.set_receipt(&message.id, Receipt::Failed).is_err());
}

#[test]
fn pending_returns_only_queued_for_recipient() {
    let store = SessionStore::open_in_memory().expect("open store");
    let group = empty_group(&store);
    let first = append(&store, &group, "worker-a", "first");
    let delivered = append(&store, &group, "worker-a", "delivered");
    append(&store, &group, "worker-b", "other recipient");
    store
        .set_receipt(&delivered.id, Receipt::Expired)
        .expect("expire message");
    assert_eq!(
        store.pending_for("worker-a").expect("list pending"),
        Some(first)
    );
}

#[test]
fn messages_list_includes_helper_and_sibling_messages_of_every_status() {
    use super::TrafficQuery;
    let store = SessionStore::open_in_memory().unwrap();
    store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
        (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title) VALUES
        ('parent','codex','/work','ready',0,1,1,'{}','Parent'),
        ('helper-a','claude','/work','ready',0,1,1,'{"helperOf":"parent"}','Helper A'),
        ('helper-b','codex','/work','ready',0,1,1,'{"helperOf":"parent"}','Helper B'),
        ('worker','codex','/work','ready',0,1,1,'{}','Worker');
        INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json) VALUES
        ('e','run-1','workflow.node.started','1',1,
        '{"ownedId":"worker","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"started_by":"parent"}}}}');"#).unwrap();
    assert_eq!(store.session_role("worker").unwrap(), "worker");
    let mut expected = std::collections::BTreeMap::new();
    for (from, to) in [("parent", "helper-a"), ("helper-a", "parent"),
        ("helper-a", "helper-b"), ("worker", "parent")] {
        for receipt in [Receipt::Queued, Receipt::Delivered, Receipt::Failed] {
            let message = store.append_message("", from, to, MessageKind::Message, "visible to owner").unwrap();
            if receipt != Receipt::Queued { store.set_receipt(&message.id, receipt).unwrap(); }
            expected.insert(message.id, (from, to, receipt.as_str()));
        }
    }
    let mut cursor: Option<(i64, String)> = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let page = store.list_messages(&TrafficQuery { limit: 2,
            before: cursor.as_ref().map(|(at, id)| (*at, id.as_str())), ..Default::default() }).unwrap();
        assert!(page.len() <= 2);
        let Some(last) = page.last() else { break };
        cursor = Some((last.created_at_ms, last.id.clone()));
        for row in page {
            assert!(seen.insert(row.id.clone()), "a message appears on exactly one page");
            if let Some((from, to, receipt)) = expected.remove(&row.id) {
                assert_eq!((row.from_owned_id.as_str(), row.to_owned_id.as_str(), row.receipt.as_str()), (from, to, receipt));
            } else {
                assert!(row.body.starts_with("[copy: "));
                assert_eq!(row.to_owned_id, "parent");
            }
        }
    }
    assert!(expected.is_empty(), "all directions and statuses reach the owner's SQL pages");
    assert_eq!(seen.len(), 15, "twelve originals and three sibling copies");
}
