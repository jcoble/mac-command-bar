//! Regression pins for the Tasks panel's cached Notion task list (TSK-1167).
//! The in-module tests in `session_store.rs` cover paging, sorting and the
//! pill filters; these cover the search and project rules around them.

use mcb_core::session_store::{NotionTaskProjection, SessionStore};

fn task(id: &str, project: &str, status: &str, title: &str) -> NotionTaskProjection {
    NotionTaskProjection {
        source_task_id: id.to_owned(),
        title: title.to_owned(),
        project: project.to_owned(),
        status: status.to_owned(),
        priority: None,
        assignee: None,
        due_date: None,
        source_url: format!("https://notion.local/{id}"),
        fetched_at_ms: 1,
    }
}

fn ids(store: &SessionStore, search: &str, project: &str) -> Vec<String> {
    store
        .query_notion_task_projections(0, 50, search, project, &[], &[], "title", "asc")
        .expect("query cached Notion tasks")
        .tasks
        .into_iter()
        .map(|task| task.source_task_id)
        .collect()
}

/// Pins 824a6457c: the search box finds a task by its project, status,
/// priority or assignee as well as its title, whatever the letter case.
#[test]
fn search_matches_every_listed_field_ignoring_case() {
    let store = SessionStore::open_in_memory().expect("open store");
    let mut by_priority = task("by-priority", "Assembly", "Todo", "Alpha");
    by_priority.priority = Some("Urgent".to_owned());
    let mut by_assignee = task("by-assignee", "Assembly", "Todo", "Beta");
    by_assignee.assignee = Some("Sam Lee".to_owned());
    store
        .replace_notion_task_projections(&[
            task("by-title", "Assembly", "Todo", "Fix the LOGIN page"),
            task("by-project", "Rental Command", "Todo", "Gamma"),
            task("by-status", "Assembly", "Blocked", "Delta"),
            by_priority,
            by_assignee,
        ])
        .expect("write snapshot");

    assert_eq!(ids(&store, "login", ""), ["by-title"]);
    assert_eq!(ids(&store, "RENTAL", ""), ["by-project"]);
    assert_eq!(ids(&store, "blocked", ""), ["by-status"]);
    assert_eq!(ids(&store, "urgent", ""), ["by-priority"]);
    assert_eq!(ids(&store, "sam", ""), ["by-assignee"]);
    assert_eq!(ids(&store, "  sam  ", ""), ["by-assignee"]);
}

/// Pins 824a6457c and e4f34d198 (tasks scoped to the session project): the project filter
/// shows that project only, not another project whose name contains it.
#[test]
fn project_filter_matches_the_whole_project_name() {
    let store = SessionStore::open_in_memory().expect("open store");
    store
        .replace_notion_task_projections(&[
            task("assembly", "Assembly", "Todo", "Alpha"),
            task("assembly-mobile", "Assembly Mobile", "Todo", "Beta"),
            task("none", "", "Todo", "Gamma"),
        ])
        .expect("write snapshot");

    assert_eq!(ids(&store, "", "Assembly"), ["assembly"]);
    assert_eq!(ids(&store, "", " Assembly "), ["assembly"]);
    assert_eq!(ids(&store, "", "").len(), 3);
}

/// Pins 95046f423: Future tasks stay hidden by default whatever the letter
/// case of the status, and still count as a status the user can pick.
#[test]
fn future_tasks_stay_hidden_by_default_in_any_case() {
    let store = SessionStore::open_in_memory().expect("open store");
    store
        .replace_notion_task_projections(&[
            task("active", "Assembly", "Todo", "Alpha"),
            task("future-upper", "Assembly", "FUTURE", "Beta"),
            task("future-lower", "Assembly", "future", "Gamma"),
        ])
        .expect("write snapshot");

    let page = store
        .query_notion_task_projections(0, 50, "", "", &[], &[], "title", "asc")
        .expect("query cached Notion tasks");
    assert_eq!(
        page.tasks.iter().map(|task| task.source_task_id.as_str()).collect::<Vec<_>>(),
        ["active"]
    );
    assert!(page.statuses.iter().any(|status| status == "FUTURE"));
}
