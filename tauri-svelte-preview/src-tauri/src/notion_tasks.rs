use std::collections::{BTreeSet, HashMap};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use mcb_core::session_store::{NotionTaskProjection, NotionTaskProjectionPage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent_conversation::manager::AgentRuntimeManager;

const SETTINGS_KEY: &str = "notion.tasks";
const KEYCHAIN_SERVICE: &str = "Assembly Notion";
const KEYCHAIN_ACCOUNT: &str = "integration-token";
const SECURITY_ITEM_NOT_FOUND: i32 = 44;
const NOTION_VERSION: &str = "2026-03-11";
const NOTION_PAGE_SIZE: usize = 100;
const MAX_NOTION_TASKS: usize = 500;
const MAX_NOTION_DETAIL_BLOCKS: usize = 200;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    data_source_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskSettings {
    data_source_id: String,
    has_token: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskRow {
    source_task_id: String,
    title: String,
    project: String,
    status: String,
    priority: Option<String>,
    assignee: Option<String>,
    due_date: Option<String>,
    source_url: String,
    fetched_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskRefreshReceipt {
    count: usize,
    fetched_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskPage {
    tasks: Vec<NotionTaskRow>,
    projects: Vec<String>,
    statuses: Vec<String>,
    has_more: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskDetailBlock {
    kind: String,
    text: String,
    checked: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTaskDetail {
    blocks: Vec<NotionTaskDetailBlock>,
}

struct PendingNotionTask {
    source_task_id: String,
    title: String,
    project_ids: Vec<String>,
    status: String,
    priority: Option<String>,
    assignee: Option<String>,
    due_date: Option<String>,
    source_url: String,
    fetched_at_ms: i64,
}

impl From<NotionTaskProjection> for NotionTaskRow {
    fn from(task: NotionTaskProjection) -> Self {
        Self {
            source_task_id: task.source_task_id,
            title: task.title,
            project: task.project,
            status: task.status,
            priority: task.priority,
            assignee: task.assignee,
            due_date: task.due_date,
            source_url: task.source_url,
            fetched_at_ms: task.fetched_at_ms,
        }
    }
}

impl From<NotionTaskProjectionPage> for NotionTaskPage {
    fn from(page: NotionTaskProjectionPage) -> Self {
        Self {
            tasks: page.tasks.into_iter().map(NotionTaskRow::from).collect(),
            projects: page.projects,
            statuses: page.statuses,
            has_more: page.has_more,
        }
    }
}

fn read_stored_settings(manager: &AgentRuntimeManager) -> Result<StoredSettings, String> {
    let Some(value) = manager.read_app_setting(SETTINGS_KEY)? else {
        return Ok(StoredSettings::default());
    };
    serde_json::from_str(&value).map_err(|_| "Notion task settings are invalid".to_string())
}

fn read_keychain_token() -> Result<Option<String>, String> {
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Could not read the Notion token: {error}"))?;
    if output.status.code() == Some(SECURITY_ITEM_NOT_FOUND) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err("The Notion token could not be read from Keychain".to_string());
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!token.is_empty()).then_some(token))
}

fn write_keychain_token(token: &str) -> Result<(), String> {
    let output = Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-U",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
            token,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Could not store the Notion token: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("The Notion token could not be stored in Keychain".to_string())
    }
}

fn delete_keychain_token() -> Result<(), String> {
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Could not remove the Notion token: {error}"))?;
    if output.status.success() || output.status.code() == Some(SECURITY_ITEM_NOT_FOUND) {
        Ok(())
    } else {
        Err("The Notion token could not be removed from Keychain".to_string())
    }
}

#[tauri::command]
pub async fn read_notion_task_settings(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> Result<NotionTaskSettings, String> {
    let stored = read_stored_settings(&manager)?;
    let has_token =
        tauri::async_runtime::spawn_blocking(|| read_keychain_token().map(|v| v.is_some()))
            .await
            .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
    Ok(NotionTaskSettings {
        data_source_id: stored.data_source_id,
        has_token,
    })
}

#[tauri::command]
pub async fn save_notion_task_settings(
    manager: tauri::State<'_, AgentRuntimeManager>,
    data_source_id: String,
    token: String,
) -> Result<NotionTaskSettings, String> {
    let token = token.trim().to_string();
    let active_token = if token.is_empty() {
        tauri::async_runtime::spawn_blocking(read_keychain_token)
            .await
            .map_err(|error| format!("Notion Keychain task failed: {error}"))??
            .ok_or_else(|| "Enter a Notion personal access token".to_string())?
    } else {
        token.clone()
    };
    let data_source_id = if data_source_id.trim().is_empty() {
        discover_task_data_source(&reqwest::Client::new(), &active_token).await?
    } else {
        data_source_id.trim().to_string()
    };
    if !token.is_empty() {
        tauri::async_runtime::spawn_blocking(move || write_keychain_token(&token))
            .await
            .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
    }
    let stored = StoredSettings {
        data_source_id: data_source_id.clone(),
    };
    manager.write_app_setting(
        SETTINGS_KEY,
        &serde_json::to_string(&stored).map_err(|error| error.to_string())?,
    )?;
    Ok(NotionTaskSettings {
        data_source_id,
        has_token: true,
    })
}

pub(crate) async fn save_oauth_connection(
    manager: &AgentRuntimeManager,
    token: String,
) -> Result<NotionTaskSettings, String> {
    let data_source_id = discover_task_data_source(&reqwest::Client::new(), &token).await?;
    let token_to_store = token.clone();
    tauri::async_runtime::spawn_blocking(move || write_keychain_token(&token_to_store))
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
    let stored = StoredSettings {
        data_source_id: data_source_id.clone(),
    };
    manager.write_app_setting(
        SETTINGS_KEY,
        &serde_json::to_string(&stored).map_err(|error| error.to_string())?,
    )?;
    Ok(NotionTaskSettings {
        data_source_id,
        has_token: true,
    })
}

async fn discover_task_data_source(
    client: &reqwest::Client,
    token: &str,
) -> Result<String, String> {
    let response = notion_request(client.post("https://api.notion.com/v1/search"), token)
        .json(&json!({
            "page_size": NOTION_PAGE_SIZE,
            "filter": { "property": "object", "value": "data_source" }
        }))
        .send()
        .await
        .map_err(|error| format!("Notion could not be reached: {error}"))?;
    task_data_source_id(&response_payload(response).await?)
}

fn task_data_source_id(payload: &Value) -> Result<String, String> {
    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "Notion returned no data sources".to_string())?;
    let candidates = results
        .iter()
        .filter(|source| source.get("object").and_then(Value::as_str) == Some("data_source"))
        .filter(|source| {
            let Some(properties) = source.get("properties").and_then(Value::as_object) else {
                return false;
            };
            properties.contains_key("Status")
                && properties
                    .values()
                    .any(|property| property.get("type").and_then(Value::as_str) == Some("title"))
        })
        .filter_map(|source| {
            Some((
                source.get("id")?.as_str()?.to_string(),
                rich_text(source.get("title")?).unwrap_or_default(),
            ))
        })
        .collect::<Vec<_>>();
    let named_tasks = candidates
        .iter()
        .filter(|(_, title)| title.eq_ignore_ascii_case("tasks"))
        .collect::<Vec<_>>();
    if let [candidate] = named_tasks.as_slice() {
        return Ok(candidate.0.clone());
    }
    if let [candidate] = candidates.as_slice() {
        return Ok(candidate.0.clone());
    }
    if candidates.is_empty() {
        Err("No Notion task database was found. Make sure the token can read a database with a title and Status fields.".to_string())
    } else {
        Err("More than one Notion task database was found. Open Advanced and paste the data source ID to choose one.".to_string())
    }
}

#[tauri::command]
pub async fn clear_notion_task_settings(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(delete_keychain_token)
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
    manager.write_app_setting(
        SETTINGS_KEY,
        &serde_json::to_string(&StoredSettings::default()).map_err(|error| error.to_string())?,
    )
}

#[tauri::command]
pub async fn list_notion_tasks(
    manager: tauri::State<'_, AgentRuntimeManager>,
    offset: u32,
    limit: u32,
    search: String,
    project: String,
    status: String,
    sort_by: String,
    sort_direction: String,
) -> Result<NotionTaskPage, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .query_notion_task_projections(
                offset,
                limit.min(100),
                &search,
                &project,
                &status,
                &sort_by,
                &sort_direction,
            )
            .map(NotionTaskPage::from)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Notion cache task failed: {error}"))?
}

#[tauri::command]
pub async fn read_notion_task_detail(source_task_id: String) -> Result<NotionTaskDetail, String> {
    validate_task_id(&source_task_id)?;
    let token = tauri::async_runtime::spawn_blocking(read_keychain_token)
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??
        .ok_or_else(|| "Connect Notion first".to_string())?;
    read_task_blocks(&reqwest::Client::new(), &token, &source_task_id).await
}

#[tauri::command]
pub async fn update_notion_task_status(
    manager: tauri::State<'_, AgentRuntimeManager>,
    source_task_id: String,
    status: String,
) -> Result<NotionTaskRefreshReceipt, String> {
    validate_task_id(&source_task_id)?;
    let status = status.trim();
    if status.is_empty() || status.len() > 100 {
        return Err("Choose a valid Notion task status".to_string());
    }
    let token = tauri::async_runtime::spawn_blocking(read_keychain_token)
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??
        .ok_or_else(|| "Connect Notion first".to_string())?;
    let client = reqwest::Client::new();
    let response = notion_request(
        client.patch(format!("https://api.notion.com/v1/pages/{source_task_id}")),
        &token,
    )
    .json(&status_update_payload(status))
    .send()
    .await
    .map_err(|error| format!("Notion could not update this task: {error}"))?;
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(
            "Assembly's Notion connection needs the Update content capability. Enable it in Notion Developer tools, then reconnect Notion in Assembly."
                .to_string(),
        );
    }
    response_payload(response).await?;
    refresh_notion_tasks(manager).await
}

#[tauri::command]
pub async fn refresh_notion_tasks(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> Result<NotionTaskRefreshReceipt, String> {
    let settings = read_stored_settings(&manager)?;
    if settings.data_source_id.trim().is_empty() {
        return Err("Configure a Notion data source first".to_string());
    }
    let token = tauri::async_runtime::spawn_blocking(read_keychain_token)
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??
        .ok_or_else(|| "Configure a Notion integration token first".to_string())?;
    let client = reqwest::Client::new();
    let fetched_at_ms = now_ms();
    let pending_tasks =
        query_task_pages(&client, &token, &settings.data_source_id, fetched_at_ms).await?;
    let project_names = load_project_names(&client, &token, &pending_tasks).await?;
    let tasks = pending_tasks
        .into_iter()
        .map(|task| finish_task_projection(task, &project_names))
        .collect::<Vec<_>>();
    let count = tasks.len();
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .replace_notion_task_projections(&tasks)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Notion cache task failed: {error}"))??;
    Ok(NotionTaskRefreshReceipt {
        count,
        fetched_at_ms,
    })
}

async fn query_task_pages(
    client: &reqwest::Client,
    token: &str,
    data_source_id: &str,
    fetched_at_ms: i64,
) -> Result<Vec<PendingNotionTask>, String> {
    let mut tasks = Vec::new();
    let mut results_seen = 0;
    let mut cursor: Option<String> = None;
    while results_seen < MAX_NOTION_TASKS {
        let mut body = json!({ "page_size": NOTION_PAGE_SIZE });
        if let Some(value) = &cursor {
            body["start_cursor"] = Value::String(value.clone());
        }
        let response = notion_request(
            client.post(format!(
                "https://api.notion.com/v1/data_sources/{data_source_id}/query"
            )),
            token,
        )
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("Notion could not be reached: {error}"))?;
        let payload = response_payload(response).await?;
        let results = payload
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| "Notion returned no task results".to_string())?;
        if results.is_empty() {
            break;
        }
        let result_count = results.len().min(MAX_NOTION_TASKS - results_seen);
        results_seen += result_count;
        tasks.extend(
            results
                .iter()
                .take(result_count)
                .filter_map(|page| pending_task_projection(page, fetched_at_ms)),
        );
        if payload.get("has_more").and_then(Value::as_bool) != Some(true) {
            break;
        }
        cursor = payload
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }
    Ok(tasks)
}

async fn read_task_blocks(
    client: &reqwest::Client,
    token: &str,
    source_task_id: &str,
) -> Result<NotionTaskDetail, String> {
    let mut blocks = Vec::new();
    let mut cursor: Option<String> = None;
    while blocks.len() < MAX_NOTION_DETAIL_BLOCKS {
        let mut request = notion_request(
            client.get(format!(
                "https://api.notion.com/v1/blocks/{source_task_id}/children"
            )),
            token,
        )
        .query(&[("page_size", NOTION_PAGE_SIZE.to_string())]);
        if let Some(value) = &cursor {
            request = request.query(&[("start_cursor", value)]);
        }
        let payload = response_payload(
            request
                .send()
                .await
                .map_err(|error| format!("Notion could not read this task: {error}"))?,
        )
        .await?;
        let results = payload
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| "Notion returned no task content".to_string())?;
        blocks.extend(
            results
                .iter()
                .take(MAX_NOTION_DETAIL_BLOCKS - blocks.len())
                .filter_map(detail_block),
        );
        if payload.get("has_more").and_then(Value::as_bool) != Some(true) {
            break;
        }
        cursor = payload
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }
    Ok(NotionTaskDetail { blocks })
}

fn detail_block(block: &Value) -> Option<NotionTaskDetailBlock> {
    let kind = block.get("type")?.as_str()?;
    if kind == "divider" {
        return Some(NotionTaskDetailBlock {
            kind: kind.to_string(),
            text: String::new(),
            checked: None,
        });
    }
    let value = block.get(kind)?;
    let text = rich_text(value.get("rich_text")?).unwrap_or_default();
    if text.is_empty() && kind != "to_do" {
        return None;
    }
    Some(NotionTaskDetailBlock {
        kind: kind.to_string(),
        text,
        checked: (kind == "to_do")
            .then(|| value.get("checked").and_then(Value::as_bool).unwrap_or(false)),
    })
}

async fn load_project_names(
    client: &reqwest::Client,
    token: &str,
    tasks: &[PendingNotionTask],
) -> Result<HashMap<String, String>, String> {
    let ids = tasks
        .iter()
        .flat_map(|task| task.project_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut names = HashMap::new();
    for id in ids.into_iter().take(50) {
        let response = notion_request(
            client.get(format!("https://api.notion.com/v1/pages/{id}")),
            token,
        )
        .send()
        .await
        .map_err(|error| format!("Notion could not read project {id}: {error}"))?;
        let payload = response_payload(response).await?;
        names.insert(
            id,
            page_title(&payload).unwrap_or_else(|| "Project".to_string()),
        );
    }
    Ok(names)
}

fn notion_request(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder
        .bearer_auth(token)
        .header("Notion-Version", NOTION_VERSION)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
}

async fn response_payload(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| format!("Notion returned invalid JSON: {error}"))?;
    if status.is_success() {
        Ok(payload)
    } else {
        Err(payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Notion rejected the request")
            .to_string())
    }
}

fn pending_task_projection(page: &Value, fetched_at_ms: i64) -> Option<PendingNotionTask> {
    let properties = page.get("properties")?;
    Some(PendingNotionTask {
        source_task_id: page.get("id")?.as_str()?.to_string(),
        title: property_text(properties, "Name", "title")
            .or_else(|| page_title(page))
            .unwrap_or_else(|| "Untitled task".to_string()),
        project_ids: project_relation_ids(page),
        status: property_choice(properties, "Status").unwrap_or_else(|| "Unspecified".to_string()),
        priority: property_choice(properties, "Priority"),
        assignee: property_people(properties, "Assignee"),
        due_date: property_date(properties, "Due"),
        source_url: page
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        fetched_at_ms,
    })
}

fn finish_task_projection(
    task: PendingNotionTask,
    project_names: &HashMap<String, String>,
) -> NotionTaskProjection {
    let project = task
        .project_ids
        .iter()
        .filter_map(|id| project_names.get(id))
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    NotionTaskProjection {
        source_task_id: task.source_task_id,
        title: task.title,
        project,
        status: task.status,
        priority: task.priority,
        assignee: task.assignee,
        due_date: task.due_date,
        source_url: task.source_url,
        fetched_at_ms: task.fetched_at_ms,
    }
}

fn project_relation_ids(page: &Value) -> Vec<String> {
    page.get("properties")
        .and_then(|properties| properties.get("Project"))
        .and_then(|property| property.get("relation"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn page_title(page: &Value) -> Option<String> {
    let properties = page.get("properties")?.as_object()?;
    properties
        .values()
        .find_map(|property| rich_text(property.get("title")?))
}

fn property_text(properties: &Value, name: &str, kind: &str) -> Option<String> {
    rich_text(properties.get(name)?.get(kind)?)
}

fn rich_text(value: &Value) -> Option<String> {
    let text = value
        .as_array()?
        .iter()
        .filter_map(|part| part.get("plain_text").and_then(Value::as_str))
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

fn property_choice(properties: &Value, name: &str) -> Option<String> {
    let property = properties.get(name)?;
    property
        .get("status")
        .or_else(|| property.get("select"))?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

fn property_people(properties: &Value, name: &str) -> Option<String> {
    let names = properties
        .get(name)?
        .get("people")?
        .as_array()?
        .iter()
        .filter_map(|person| person.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(", ");
    (!names.is_empty()).then_some(names)
}

fn property_date(properties: &Value, name: &str) -> Option<String> {
    properties
        .get(name)?
        .get("date")?
        .get("start")?
        .as_str()
        .map(str::to_string)
}

fn validate_task_id(source_task_id: &str) -> Result<(), String> {
    if source_task_id.len() > 64
        || !source_task_id
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
    {
        return Err("The Notion task ID is invalid".to_string());
    }
    Ok(())
}

fn status_update_payload(status: &str) -> Value {
    json!({ "properties": { "Status": { "select": { "name": status } } } })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_the_named_tasks_data_source() {
        let payload = json!({
            "results": [
                {
                    "object": "data_source",
                    "id": "notes-id",
                    "title": [{ "plain_text": "Notes" }],
                    "properties": { "Name": { "type": "title" } }
                },
                {
                    "object": "data_source",
                    "id": "tasks-id",
                    "title": [{ "plain_text": "Tasks" }],
                    "properties": {
                        "Name": { "type": "title" },
                        "Status": { "type": "status" }
                    }
                }
            ]
        });
        assert_eq!(task_data_source_id(&payload).unwrap(), "tasks-id");
    }

    #[test]
    fn requires_a_choice_when_multiple_task_sources_match() {
        let results = ["Alpha", "Beta"]
            .into_iter()
            .map(|title| {
                json!({
                "object": "data_source",
                "id": format!("{title}-id"),
                "title": [{ "plain_text": title }],
                "properties": {
                    "Name": { "type": "title" },
                    "Status": { "type": "status" }
                }
                })
            })
            .collect::<Vec<_>>();
        let payload = json!({ "results": results });
        assert!(task_data_source_id(&payload)
            .unwrap_err()
            .contains("More than one"));
    }

    #[test]
    fn task_page_projects_only_the_fields_the_panel_uses() {
        let page = json!({
            "id": "task-1",
            "url": "https://www.notion.so/task-1",
            "properties": {
                "Name": { "title": [{ "plain_text": "[TSK-808] Workbench" }] },
                "Project": { "relation": [{ "id": "project-1" }] },
                "Status": { "select": { "name": "Doing" } },
                "Priority": { "select": { "name": "High" } },
                "Assignee": { "people": [{ "name": "Codex" }] },
                "Due": { "date": { "start": "2026-08-25" } }
            }
        });
        let task = finish_task_projection(
            pending_task_projection(&page, 42).unwrap(),
            &HashMap::from([("project-1".to_string(), "MacCommandBar".to_string())]),
        );

        assert_eq!(task.title, "[TSK-808] Workbench");
        assert_eq!(task.project, "MacCommandBar");
        assert_eq!(task.status, "Doing");
        assert_eq!(task.assignee.as_deref(), Some("Codex"));
        assert_eq!(task.fetched_at_ms, 42);
    }

    #[test]
    fn task_without_project_does_not_invent_an_assignment() {
        let page = json!({
            "id": "task-1",
            "url": "https://www.notion.so/task-1",
            "properties": {
                "Name": { "title": [{ "plain_text": "[TSK-808] Workbench" }] },
                "Status": { "select": { "name": "Doing" } }
            }
        });
        let task = finish_task_projection(
            pending_task_projection(&page, 42).unwrap(),
            &HashMap::new(),
        );

        assert!(task.project.is_empty());
    }

    #[test]
    fn task_detail_projects_text_and_checkbox_state() {
        let paragraph = detail_block(&json!({
            "type": "paragraph",
            "paragraph": {
                "rich_text": [
                    { "plain_text": "First " },
                    { "plain_text": "note" }
                ]
            }
        }))
        .unwrap();
        let checkbox = detail_block(&json!({
            "type": "to_do",
            "to_do": {
                "rich_text": [{ "plain_text": "Verify it" }],
                "checked": true
            }
        }))
        .unwrap();

        assert_eq!(paragraph.kind, "paragraph");
        assert_eq!(paragraph.text, "First note");
        assert_eq!(paragraph.checked, None);
        assert_eq!(checkbox.text, "Verify it");
        assert_eq!(checkbox.checked, Some(true));
    }

    #[test]
    fn task_status_update_changes_only_the_status_property() {
        assert_eq!(
            status_update_payload("Doing"),
            json!({ "properties": { "Status": { "select": { "name": "Doing" } } } })
        );
    }
}
