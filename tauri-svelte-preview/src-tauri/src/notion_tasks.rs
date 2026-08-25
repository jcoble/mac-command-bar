use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use mcb_core::session_store::NotionTaskProjection;
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
    let data_source_id = data_source_id.trim().to_string();
    if data_source_id.is_empty() {
        return Err("Enter a Notion data source ID".to_string());
    }
    let token = token.trim().to_string();
    if !token.is_empty() {
        tauri::async_runtime::spawn_blocking(move || write_keychain_token(&token))
            .await
            .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
    } else {
        let has_token = tauri::async_runtime::spawn_blocking(|| {
            read_keychain_token().map(|value| value.is_some())
        })
        .await
        .map_err(|error| format!("Notion Keychain task failed: {error}"))??;
        if !has_token {
            return Err("Enter a Notion integration token".to_string());
        }
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
) -> Result<Vec<NotionTaskRow>, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .list_notion_task_projections(offset, limit.min(100))
            .map(|rows| rows.into_iter().map(NotionTaskRow::from).collect())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Notion cache task failed: {error}"))?
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
    let pages = query_task_pages(&client, &token, &settings.data_source_id).await?;
    let project_names = load_project_names(&client, &token, &pages).await?;
    let tasks = pages
        .iter()
        .filter_map(|page| project_task(page, &project_names, fetched_at_ms))
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
) -> Result<Vec<Value>, String> {
    let mut pages = Vec::new();
    let mut cursor: Option<String> = None;
    while pages.len() < MAX_NOTION_TASKS {
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
        pages.extend(results.iter().take(MAX_NOTION_TASKS - pages.len()).cloned());
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
    Ok(pages)
}

async fn load_project_names(
    client: &reqwest::Client,
    token: &str,
    pages: &[Value],
) -> Result<HashMap<String, String>, String> {
    let ids = pages
        .iter()
        .flat_map(project_relation_ids)
        .collect::<HashSet<_>>();
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

fn project_task(
    page: &Value,
    project_names: &HashMap<String, String>,
    fetched_at_ms: i64,
) -> Option<NotionTaskProjection> {
    let properties = page.get("properties")?;
    let source_task_id = page.get("id")?.as_str()?.to_string();
    let title = property_text(properties, "Name", "title")
        .or_else(|| page_title(page))
        .unwrap_or_else(|| "Untitled task".to_string());
    let project = project_relation_ids(page)
        .into_iter()
        .filter_map(|id| project_names.get(&id).cloned())
        .collect::<Vec<_>>()
        .join(", ");
    Some(NotionTaskProjection {
        source_task_id,
        title,
        project: if project.is_empty() {
            "Unassigned".to_string()
        } else {
            project
        },
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
        let task = project_task(
            &page,
            &HashMap::from([("project-1".to_string(), "MacCommandBar".to_string())]),
            42,
        )
        .unwrap();

        assert_eq!(task.title, "[TSK-808] Workbench");
        assert_eq!(task.project, "MacCommandBar");
        assert_eq!(task.status, "Doing");
        assert_eq!(task.assignee.as_deref(), Some("Codex"));
        assert_eq!(task.fetched_at_ms, 42);
    }
}
