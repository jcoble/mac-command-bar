use mcb_core::{broker::{MessageKind, TrafficQuery}, session_store::SessionStore};

#[test]
fn a_source_qualified_sender_stays_in_its_message_list_and_count() {
    let store = SessionStore::open_in_memory().unwrap();
    store.append_message("", "box-a:sender", "receiver", MessageKind::Message, "from A").unwrap();
    store.append_message("", "box-b:sender", "receiver", MessageKind::Message, "from B").unwrap();
    let messages = store.list_messages(&TrafficQuery { session: Some("sender"), qualified_sender: Some("box-a:sender"), limit: 10, ..Default::default() }).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].from_owned_id, "box-a:sender");
    assert!(store.list_messages(&TrafficQuery { session: Some("sender"), qualified_sender: Some("box-c:sender"), limit: 10, ..Default::default() }).unwrap().is_empty());
    assert_eq!(store.count_messages("sender", 0, Some("box-c:sender")).unwrap(), 0);
    assert_eq!(store.count_messages("receiver", 0, None).unwrap(), 2);
    assert_eq!(store.count_messages("sender", 0, Some("box-a:sender")).unwrap(), 1);
}
