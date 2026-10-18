//! The phone-only listener: pairing codes and device tokens (phone spec §5.2).
//!
//! It runs beside the desktop listener when `ASSEMBLY_PHONE_BIND` is set and
//! serves only `/phone/v1`; the desktop's full-control `/assembly` socket and
//! its server token are never reachable through it.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::ws::{rejection::WebSocketUpgradeRejection, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Extension, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::protocol::{
    AgentConversationEvent, AgentConversationPayload as Payload, AgentNativeSessionMode, EnsureAgentConversationRequest,
    ExecutionEnvironment, RespondAgentConversationPermissionRequest, SendAgentConversationMessageRequest, ToolState, TurnState,
};
use super::phone_push::PushSender;
use super::remote::{execute_phone_command, RemoteCommand, RemoteResponse, ServerState};
use crate::workflow::{WorkflowError, WorkflowNodeState, WorkflowOutputContract, WorkflowRunRecord, WorkflowRunState};

/// How long a pairing code works: 10 minutes.
pub(super) const PAIRING_CODE_LIFETIME_MS: i64 = 10 * 60 * 1000;
const WRONG_TRIES_ALLOWED: u32 = 5;
const NOT_SET_UP: &str =
    "Phone access is not set up on this machine: set ASSEMBLY_PHONE_URL in ~/.config/assembly/phone.env";
const REMOVED: &str = "This phone was removed — pair again";
const NO_ORCHESTRATOR: &str = "No orchestrator is marked on this machine";
const NO_SESSION: &str = "No such session on this machine";
const NO_TOOL_CALL: &str = "No such tool call in this session";
/// §5.3 sets no length for chat text, so a plain cap: 10,000 characters, in a body of at most 64 KiB.
const MAX_CHAT_CHARS: usize = 10_000;
const MAX_BODY_BYTES: usize = 64 * 1024;
/// `GET /chat` reads one desktop-sized page of events, and with no `after` keeps its newest 100 items.
const CHAT_PAGE_BYTES: u32 = 1024 * 1024;
const LATEST_CHAT_ITEMS: usize = 100;
const MAX_PUSH_TOKEN_CHARS: usize = 4096;
/// A tool call opened on the phone sends at most this much output and diff.
const MAX_TOOL_OUTPUT_BYTES: usize = 32 * 1024;
const MAX_TOOL_DIFF_BYTES: usize = 64 * 1024;
/// A session's changes send at most this much diff; the file list is always whole.
const MAX_CHANGES_DIFF_BYTES: usize = 256 * 1024;

/// Bumped whenever a phone is removed, so each open live socket checks its own token and closes if it went.
static DEVICE_REMOVED: LazyLock<tokio::sync::watch::Sender<u64>> = LazyLock::new(|| tokio::sync::watch::channel(0).0);

type Reply = Result<Response, Response>;

/// The one pairing code that works right now, kept in memory only.
pub(super) struct PairingCode {
    code: String,
    expires_at_ms: i64,
    wrong_tries: u32,
}

/// The phone listener's routes. It never serves the desktop's `/assembly` socket. Every route but
/// pairing checks the device token before its handler runs, so no body is read for a stranger.
pub(super) fn router(state: ServerState) -> Router {
    let device_routes = Router::new()
        .route("/phone/v1/me", get(me).delete(forget_me))
        .route("/phone/v1/chat", get(chat).post(send_chat))
        .route("/phone/v1/sessions", get(sessions))
        .route("/phone/v1/sessions/{id}/chat", get(session_chat).post(send_session_chat))
        .route("/phone/v1/sessions/{id}/items/{item}", get(tool_item))
        .route("/phone/v1/sessions/{id}/changes", get(changes))
        .route("/phone/v1/runs", get(runs))
        .route("/phone/v1/runs/{id}", get(run_detail))
        .route("/phone/v1/runs/{id}/{action}", post(control_run))
        .route("/phone/v1/inbox", get(inbox))
        .route("/phone/v1/inbox/{id}/answer", post(answer))
        .route("/phone/v1/live", get(live))
        .route("/phone/v1/push-token", post(push_token))
        .route_layer(from_fn_with_state(state.clone(), require_device));
    Router::new()
        .route("/phone/v1/pair", post(pair))
        .merge(device_routes)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// The paired phone a request came from, put on the request by `require_device`.
#[derive(Clone)]
struct Device {
    id: String,
    token_hash: String,
}

async fn require_device(State(state): State<ServerState>, mut request: Request, next: Next) -> Response {
    match authorized_device(&state, request.headers()) {
        Ok(device) => {
            request.extensions_mut().insert(device);
            next.run(request).await
        }
        Err(refused) => refused,
    }
}

/// Starts the phone listener on `bind` when it is set. A bad or busy address is
/// logged and the server carries on without it: the optional phone API must
/// never stop the desktop listener. Returns whether it started.
pub(super) async fn start_listener(bind: Option<&str>, state: ServerState) -> bool {
    let Some(bind) = bind.filter(|bind| !bind.is_empty()) else { return false };
    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Phone listener not started on {bind}: {error}");
            return false;
        }
    };
    println!("Assembly phone listener on {bind}");
    match std::env::var("ASSEMBLY_FCM_SERVICE_ACCOUNT").ok().filter(|path| !path.is_empty()) {
        Some(path) => if let Some(sender) = PushSender::from_key_file(std::path::Path::new(&path)) { spawn_push(state.clone(), sender); },
        None => println!("Phone push is off: ASSEMBLY_FCM_SERVICE_ACCOUNT is not set"),
    }
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router(state)).await {
            eprintln!("Phone listener stopped: {error}");
        }
    });
    true
}

/// Rings the phones once for each inbox item that appears. It wakes on any permission request or run
/// change, recomputes the inbox, and sends for each `(id, version)` it has not seen. It starts from
/// the items already waiting, so a restart does not ring for them again.
fn spawn_push(state: ServerState, sender: PushSender) {
    use tokio::sync::broadcast::error::RecvError;
    tokio::spawn(async move {
        let mut events = state.events.subscribe();
        let mut runs = state.workflow.run_changes();
        let mut seen: HashSet<(String, i64)> = inbox_items(&state).map(|items| item_keys(&items)).unwrap_or_default();
        loop {
            let closed = tokio::select! {
                event = events.recv() => match event {
                    Ok(event) if !matches!(event.payload, Payload::Approval { .. }) => continue,
                    Err(RecvError::Closed) => true,
                    _ => false,
                },
                change = runs.recv() => matches!(change, Err(RecvError::Closed)),
            };
            if closed { break; }
            let Ok(items) = inbox_items(&state) else { continue };
            let store = state.manager.store();
            let machine = store.phone_machine().unwrap_or(None);
            for (_, item) in &items {
                let key = (item["id"].as_str().unwrap_or_default().to_string(), item["version"].as_i64().unwrap_or(0));
                if let (false, Some(machine)) = (seen.contains(&key), &machine) {
                    sender.send_inbox_push(store, machine, item).await;
                }
            }
            seen = item_keys(&items);
        }
    });
}

fn item_keys(items: &[(Option<String>, Value)]) -> HashSet<(String, i64)> {
    items.iter().map(|(_, item)| (item["id"].as_str().unwrap_or_default().to_string(), item["version"].as_i64().unwrap_or(0))).collect()
}

/// Answers the desktop's phone operations, sent as workflow operations over its
/// socket; `None` for every other operation, which goes to the workflow engine.
pub(super) fn operation(state: &ServerState, operation: &str, args: &Value) -> Option<Result<Value, String>> {
    let store = state.manager.store();
    Some(match operation {
        "phonePairing" => start_pairing(state, args),
        "phoneDevices" => store.phone_devices().map(|devices| json!({ "devices": devices })).map_err(|error| error.to_string()),
        "phoneRevoke" => match args["deviceId"].as_str() {
            Some(device_id) => store.revoke_phone_device(device_id, now_ms())
                .map(|revoked| {
                    DEVICE_REMOVED.send_modify(|count| *count = count.wrapping_add(1));
                    json!({ "revoked": revoked })
                }).map_err(|error| error.to_string()),
            None => Err("phoneRevoke needs a deviceId".into()),
        },
        _ => return None,
    })
}

/// Names this machine for phones and replaces the pairing code with a new one.
fn start_pairing(state: &ServerState, args: &Value) -> Result<Value, String> {
    let url = std::env::var("ASSEMBLY_PHONE_URL").ok().filter(|url| !url.is_empty()).ok_or(NOT_SET_UP)?;
    let name = args["machineName"].as_str().map(str::trim).filter(|name| !name.is_empty())
        .ok_or("phonePairing needs a machineName")?;
    let (_, machine_name) = state.manager.store()
        .name_phone_machine(&uuid::Uuid::new_v4().to_string(), name).map_err(|error| error.to_string())?;
    let (code, expires_at) = new_pairing_code(&state.pairing, now_ms())?;
    Ok(json!({ "url": url, "machineName": machine_name, "code": code, "expiresAt": expires_at }))
}

/// Makes a 10-character base32 code that replaces any earlier one. Returns it and its expiry (ms).
pub(super) fn new_pairing_code(slot: &Mutex<Option<PairingCode>>, now_ms: i64) -> Result<(String, i64), String> {
    const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let code: String = random_bytes::<10>()?.iter().map(|byte| char::from(BASE32[usize::from(byte & 31)])).collect();
    let expires_at_ms = now_ms + PAIRING_CODE_LIFETIME_MS;
    *slot.lock().unwrap_or_else(PoisonError::into_inner) =
        Some(PairingCode { code: code.clone(), expires_at_ms, wrong_tries: 0 });
    Ok((code, expires_at_ms))
}

/// Uses up the code when `code` matches it in time. A wrong guess counts
/// toward the five that kill it; an expired code is dropped.
pub(super) fn redeem_pairing_code(slot: &Mutex<Option<PairingCode>>, code: &str, now_ms: i64) -> bool {
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(current) = slot.as_mut() else { return false };
    if now_ms > current.expires_at_ms {
        *slot = None;
        return false;
    }
    if current.code == code {
        *slot = None;
        return true;
    }
    current.wrong_tries += 1;
    if current.wrong_tries >= WRONG_TRIES_ALLOWED {
        *slot = None;
    }
    false
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device_name: String,
}

/// `POST /phone/v1/pair {code, deviceName}` → `{deviceId, deviceToken, machine:{id, name}}`.
async fn pair(State(state): State<ServerState>, Json(request): Json<PairRequest>) -> Response {
    // Checked before the code is spent, so a bad name does not burn it.
    let device_name = request.device_name.trim();
    if device_name.is_empty() || device_name.chars().count() > 100 {
        return refuse(StatusCode::BAD_REQUEST, "The device name must be 1 to 100 characters");
    }
    if !redeem_pairing_code(&state.pairing, &request.code, now_ms()) {
        return refuse(StatusCode::UNAUTHORIZED, "This pairing code is wrong or has expired; show a new one on the desktop");
    }
    match add_device(&state, device_name) {
        Ok(body) => Json(body).into_response(),
        Err(error) => server_error(&error),
    }
}

fn add_device(state: &ServerState, device_name: &str) -> Result<Value, String> {
    let store = state.manager.store();
    let (machine_id, machine_name) = store.phone_machine().map_err(|error| error.to_string())?
        .ok_or("This machine has not been named for phones yet")?;
    let device_id = uuid::Uuid::new_v4().to_string();
    let token = URL_SAFE_NO_PAD.encode(random_bytes::<32>()?);
    store.add_phone_device(&device_id, device_name, &token_hash(&token), now_ms()).map_err(|error| error.to_string())?;
    Ok(json!({ "deviceId": device_id, "deviceToken": token, "machine": { "id": machine_id, "name": machine_name } }))
}

/// `GET /phone/v1/me` → `{deviceId, machine:{id, name}, hostsOrchestrator, orchestratorMarkedAt}`.
async fn me(State(state): State<ServerState>, Extension(device): Extension<Device>) -> Response {
    let device_id = device.id;
    let store = state.manager.store();
    let body = store.phone_machine().and_then(|machine| Ok((machine, store.main_orchestrator()?)))
        .map(|(machine, orchestrator)| json!({
            "deviceId": device_id,
            "machine": machine.map(|(id, name)| json!({ "id": id, "name": name })),
            "hostsOrchestrator": orchestrator.is_some(),
            "orchestratorMarkedAt": orchestrator.map(|(_, marked_at)| marked_at),
        }));
    match body {
        Ok(body) => Json(body).into_response(),
        Err(error) => server_error(&error),
    }
}

/// The paired device whose bearer token this is, found among non-revoked
/// devices in one SQL statement that also marks it seen. Anything else is 401.
fn authorized_device(state: &ServerState, headers: &HeaderMap) -> Result<Device, Response> {
    let token = bearer(headers).ok_or_else(|| refuse(StatusCode::UNAUTHORIZED, REMOVED))?;
    let token_hash = token_hash(token);
    match state.manager.store().touch_phone_device(&token_hash, now_ms()) {
        Ok(Some(id)) => Ok(Device { id, token_hash }),
        Ok(None) => Err(refuse(StatusCode::UNAUTHORIZED, REMOVED)),
        Err(error) => Err(server_error(&error)),
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get("authorization").and_then(|value| value.to_str().ok()).and_then(|value| value.strip_prefix("Bearer "))
}

/// `DELETE /phone/v1/me` → 204: the phone removes itself with the desktop's Remove query.
async fn forget_me(State(state): State<ServerState>, Extension(device): Extension<Device>) -> Reply {
    state.manager.store().revoke_phone_device(&device.id, now_ms()).map_err(|error| server_error(&error))?;
    DEVICE_REMOVED.send_modify(|count| *count = count.wrapping_add(1));
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The orchestrator session the owner marked on this machine (`broker.main_orchestrator`); 404 when none is.
fn orchestrator(state: &ServerState) -> Result<String, Response> {
    state.manager.store().main_orchestrator().map_err(|error| server_error(&error))?
        .map(|(owned_id, _)| owned_id).ok_or_else(|| refuse(StatusCode::NOT_FOUND, NO_ORCHESTRATOR))
}

/// `GET /phone/v1/sessions` → `{sessions}`: this machine's sessions, newest activity first (one SQL statement).
async fn sessions(State(state): State<ServerState>) -> Reply {
    let sessions = state.manager.store().list_phone_sessions().map_err(|error| server_error(&error))?;
    let sessions: Vec<Value> = sessions.into_iter().map(|(entry, last_activity_at)| {
        let mut row = json!(entry);
        row["lastActivityAt"] = json!(last_activity_at);
        row
    }).collect();
    Ok(Json(json!({ "sessions": sessions })).into_response())
}

/// A session in this machine's directory, the one the sessions list shows; 404 for any other.
fn known_session(state: &ServerState, owned_id: &str) -> Result<(), Response> {
    state.manager.store().directory_name(owned_id).map_err(|error| server_error(&error))?
        .map(|_| ()).ok_or_else(|| refuse(StatusCode::NOT_FOUND, NO_SESSION))
}

/// `GET /phone/v1/chat?after=<seq>`: the orchestrator's chat, as the first phone app reads it.
async fn chat(State(state): State<ServerState>, Query(query): Query<HashMap<String, String>>) -> Reply {
    let owned_id = orchestrator(&state)?;
    read_chat(&state, &owned_id, &query)
}

/// `GET /phone/v1/sessions/{id}/chat?after=<seq>`: that session's chat.
async fn session_chat(State(state): State<ServerState>, Path(owned_id): Path<String>,
    Query(query): Query<HashMap<String, String>>) -> Reply {
    known_session(&state, &owned_id)?;
    read_chat(&state, &owned_id, &query)
}

/// A session's chat items after `after`, or its newest 100.
fn read_chat(state: &ServerState, owned_id: &str, query: &HashMap<String, String>) -> Reply {
    let after = query.get("after").map(|after| after.parse::<i64>()).transpose()
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "after must be a sequence number"))?;
    let page = match after {
        Some(after) => state.manager.list_events_after(owned_id, after, CHAT_PAGE_BYTES),
        None => state.manager.list_events_before(owned_id, i64::MAX, CHAT_PAGE_BYTES),
    }.map_err(|error| server_error(&error))?;
    let mut items = reduce_chat(&page.events);
    if after.is_none() { items.drain(..items.len().saturating_sub(LATEST_CHAT_ITEMS)); }
    let owner_items: Vec<Value> = inbox_items(state)?.into_iter()
        .filter(|(session, _)| session.as_deref() == Some(owned_id)).map(|(_, item)| item).collect();
    Ok(Json(json!({
        "items": items,
        "cursor": page.events.last().map_or(after.unwrap_or(0), |event| event.sequence),
        "turn": if state.manager.turn_running(owned_id) { "running" } else { "idle" },
        "ownerItems": owner_items,
    })).into_response())
}

/// `GET /phone/v1/sessions/{id}/items/{itemId}`: one tool call whole, its latest event in the
/// newest page of events, the same page the chat list reads.
async fn tool_item(State(state): State<ServerState>, Path((owned_id, item_id)): Path<(String, String)>) -> Reply {
    known_session(&state, &owned_id)?;
    let page = state.manager.list_events_before(&owned_id, i64::MAX, CHAT_PAGE_BYTES).map_err(|error| server_error(&error))?;
    let Some(Payload::Tool { name, state: tool_state, summary, output, path, diff, .. }) = page.events.iter().rev()
        .map(|event| &event.payload).find(|payload| matches!(payload, Payload::Tool { item_id: id, .. } if *id == item_id))
    else { return Err(refuse(StatusCode::NOT_FOUND, NO_TOOL_CALL)) };
    let (output, output_truncated) = capped(output.as_deref(), MAX_TOOL_OUTPUT_BYTES);
    let (diff, diff_truncated) = capped(diff.as_deref(), MAX_TOOL_DIFF_BYTES);
    Ok(Json(json!({ "name": name, "state": tool_state, "summary": summary, "path": path, "output": output,
        "outputTruncated": output_truncated, "diff": diff, "diffTruncated": diff_truncated })).into_response())
}

/// `GET /phone/v1/sessions/{id}/changes`: what the session's branch changed since the default branch,
/// the desktop Changes tab's branch diff. The folder is the session's worktree, else its cwd, from the
/// store, never from the request. A git refusal, such as no `origin/HEAD`, is a 409 with git's message.
async fn changes(State(state): State<ServerState>, Path(owned_id): Path<String>) -> Reply {
    known_session(&state, &owned_id)?;
    let session = state.manager.store().get_session(&owned_id).map_err(|error| server_error(&error))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, NO_SESSION))?;
    let root = session.worktree.filter(|worktree| !worktree.is_empty()).unwrap_or(session.cwd);
    let branch = crate::commands::source_control::read_git_branch_diff(root).await
        .map_err(|error| refuse(StatusCode::CONFLICT, &error))?;
    let (diff, diff_truncated) = capped(Some(&branch.diff), MAX_CHANGES_DIFF_BYTES);
    Ok(Json(json!({ "base": branch.base, "files": changed_files(&branch.diff), "diff": diff,
        "diffTruncated": diff_truncated })).into_response())
}

/// Each file in a `git diff` text, in order, with its added and removed lines. The path is the `b/`
/// side of the `diff --git` header; only lines inside a hunk count, so `---`/`+++` headers do not.
fn changed_files(diff: &str) -> Vec<Value> {
    let mut files: Vec<(&str, u32, u32)> = Vec::new();
    let mut in_hunk = false;
    for line in diff.lines() {
        if let Some(header) = line.strip_prefix("diff --git ") {
            files.push((header.rsplit_once(" b/").map_or(header, |(_, path)| path), 0, 0));
            in_hunk = false;
        } else if line.starts_with("@@") {
            in_hunk = true;
        } else if let (true, Some(file)) = (in_hunk, files.last_mut()) {
            if line.starts_with('+') { file.1 += 1 } else if line.starts_with('-') { file.2 += 1 }
        }
    }
    files.into_iter().map(|(path, added, removed)| json!({ "path": path, "added": added, "removed": removed })).collect()
}

/// `text` cut to at most `max` bytes on a char boundary, and whether it was cut.
fn capped(text: Option<&str>, max: usize) -> (Option<&str>, bool) {
    match text {
        Some(text) if text.len() > max => (Some(&text[..text.floor_char_boundary(max)]), true),
        text => (text, false),
    }
}

#[derive(Deserialize)]
struct ChatRequest {
    text: String,
}

/// `POST /phone/v1/chat {text}` → `{itemId}`: a message to the orchestrator, as the first phone app sends it.
async fn send_chat(State(state): State<ServerState>, body: Bytes) -> Reply {
    let owned_id = orchestrator(&state)?;
    send_to(&state, owned_id, &body).await
}

/// `POST /phone/v1/sessions/{id}/chat {text}` → `{itemId}`: a message to that session.
async fn send_session_chat(State(state): State<ServerState>, Path(owned_id): Path<String>, body: Bytes) -> Reply {
    known_session(&state, &owned_id)?;
    send_to(&state, owned_id, &body).await
}

/// The owner's own turn in `owned_id`: the desktop's ensure, which wakes an idle or suspended
/// session, then its send.
async fn send_to(state: &ServerState, owned_id: String, body: &[u8]) -> Reply {
    let request: ChatRequest = parse(body)?;
    let text = request.text.trim();
    if text.is_empty() || text.chars().count() > MAX_CHAT_CHARS {
        return Err(refuse(StatusCode::BAD_REQUEST, "A message must be 1 to 10,000 characters"));
    }
    let row = state.manager.store().get_session(&owned_id).map_err(|error| server_error(&error))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, NO_SESSION))?;
    let provider = super::transcript::parse_provider(&row.provider).map_err(|error| server_error(&error))?;
    // After a restart the manager holds no sessions; ensure loads the stored one and gives its generation.
    let ensure = RemoteCommand::Ensure(EnsureAgentConversationRequest {
        owned_id: owned_id.clone(), execution_environment: ExecutionEnvironment::Local, remote_profile_id: None,
        provider, cwd: row.cwd, native_session_id: row.native_session_id,
        native_session_mode: AgentNativeSessionMode::Resume, reasoning_effort: None, project_id: row.project_id,
    });
    let RemoteResponse::Connection(connection) = execute_phone_command(state, ensure).await
        .map_err(|error| server_error(&error))? else { return Err(server_error(&"ensure gave the wrong response")) };
    let send = RemoteCommand::Send(SendAgentConversationMessageRequest {
        owned_id, generation: connection.generation, text: text.into(), content: Vec::new(),
        attachment_ids: Vec::new(), model: None, approval_policy: None,
    });
    let RemoteResponse::SendReceipt(receipt) = execute_phone_command(state, send).await
        .map_err(|error| server_error(&error))? else { return Err(server_error(&"send gave the wrong response")) };
    Ok(Json(json!({ "itemId": receipt.user_item_id })).into_response())
}

/// `GET /phone/v1/runs` → `{active, recent}`: unfinished runs and the 20 newest finished ones.
async fn runs(State(state): State<ServerState>) -> Reply {
    let runs = state.workflow.recent_runs().map_err(|error| server_error(&error))?;
    let (recent, active): (Vec<_>, Vec<_>) = runs.iter()
        .partition(|run| matches!(run.state, WorkflowRunState::Completed | WorkflowRunState::Failed | WorkflowRunState::Cancelled));
    Ok(Json(json!({
        "active": active.into_iter().map(run_row).collect::<Vec<_>>(),
        "recent": recent.into_iter().map(run_row).collect::<Vec<_>>(),
    })).into_response())
}

/// `GET /phone/v1/runs/{id}` → the run's row, steps with their latest attempt, and its last 20 changes.
async fn run_detail(State(state): State<ServerState>, Path(run_id): Path<String>) -> Reply {
    let run = match state.workflow.get_run(&run_id) {
        Ok(run) => run,
        Err(WorkflowError::NotFound(_)) => return Err(refuse(StatusCode::NOT_FOUND, "No such run")),
        Err(error) => return Err(server_error(&error)),
    };
    let timeline = state.workflow.run_timeline(&run_id).map_err(|error| server_error(&error))?;
    let mut body = run_row(&run);
    body["branch"] = run.input_snapshot["worktree"]["branch"].clone();
    body["steps"] = run.definition.nodes.iter().filter_map(|step| {
        let node = run.nodes.iter().find(|node| node.node_id == step.id)?;
        let output = node.structured_output.as_ref().unwrap_or(&Value::Null);
        let failing: Vec<Value> = output["commands"].as_array().into_iter().flatten().filter(|command| command["exitCode"] != 0)
            .map(|command| {
                let text = command["output"].as_str().unwrap_or_default();
                let tail = text.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                json!({ "command": command["command"], "exit": command["exitCode"], "tail": tail })
            }).collect();
        let findings = output["findings"].as_array().map(|findings| findings.iter()
            .map(|finding| finding.as_str().map_or_else(|| finding.to_string(), str::to_owned)).collect::<Vec<_>>());
        let kind = ["agent", "review", "run", "gate"].into_iter().find(|kind| step.step.get(kind).is_some()).unwrap_or("agent");
        Some(json!({
            "id": step.id,
            "kind": kind,
            "state": node.state,
            "attempts": [{
                "n": node.attempt, "state": node.state, "startedAt": node.started_at_ms, "endedAt": node.finished_at_ms,
                "summary": output["summary"].as_str().or(node.failure.as_ref().map(|failure| failure.message.as_str())),
                "findings": findings, "failingCommands": failing,
            }],
        }))
    }).collect();
    body["timeline"] = timeline.into_iter().map(|(at, text)| json!({ "at": at, "text": text })).collect();
    Ok(Json(body).into_response())
}

/// `POST /phone/v1/runs/{id}/pause|resume|stop` → the run's row, through the desktop's engine operations.
async fn control_run(State(state): State<ServerState>, Path((run_id, action)): Path<(String, String)>) -> Reply {
    let operation = match action.as_str() {
        "pause" => "pause",
        "resume" => "resume",
        "stop" => "cancel",
        _ => return Err(refuse(StatusCode::NOT_FOUND, "Unknown run action")),
    };
    let command = RemoteCommand::Workflow { operation: operation.into(),
        args: json!({ "runId": run_id, "idempotencyKey": format!("phone:{}", uuid::Uuid::new_v4()) }) };
    // The engine's own refusal, such as "Only a running workflow can be paused", is the phone's to
    // show; anything else is an internal error and stays in the log.
    let run = match execute_phone_command(&state, command).await {
        Ok(RemoteResponse::Workflow(run)) => run,
        Ok(_) => return Err(server_error(&"run control gave the wrong response")),
        Err(error) => return Err(match serde_json::from_str::<WorkflowError>(&error) {
            Ok(WorkflowError::InvalidTransition(message) | WorkflowError::PolicyDenied(message)) => refuse(StatusCode::CONFLICT, &message),
            Ok(WorkflowError::NotFound(_)) => refuse(StatusCode::NOT_FOUND, "No such run"),
            _ => server_error(&error),
        }),
    };
    let run: WorkflowRunRecord = serde_json::from_value(run).map_err(|error| server_error(&error))?;
    Ok(Json(run_row(&run)).into_response())
}

fn run_row(run: &WorkflowRunRecord) -> Value {
    let waiting = run.nodes.iter().find(|node| matches!(node.state, WorkflowNodeState::WaitingInput | WorkflowNodeState::WaitingApproval));
    let current = waiting.or_else(|| run.nodes.iter().find(|node| matches!(node.state, WorkflowNodeState::Starting | WorkflowNodeState::Running)));
    let pr_url = run.nodes.iter()
        .filter(|node| matches!(node.output_contract, WorkflowOutputContract::PullRequestReceipt | WorkflowOutputContract::PullRequestMergeReceipt))
        .find_map(|node| node.structured_output.as_ref()?["url"].as_str());
    json!({
        "id": run.id, "taskKey": run.input_snapshot["task"]["key"], "taskTitle": run.input_snapshot["task"]["title"],
        "workflow": run.definition.name, "state": run.state, "currentStep": current.map(|node| &node.node_id),
        "round": run.nodes.iter().map(|node| node.attempt).max().unwrap_or(1),
        "startedAt": run.created_at_ms, "updatedAt": run.updated_at_ms,
        "iMerge": run.input_snapshot["inputs"]["i_merge"].as_bool().unwrap_or(false),
        "prUrl": pr_url, "waitingItemId": waiting.map(|node| format!("step:{}:{}", run.id, node.node_id)),
    })
}

/// `GET /phone/v1/inbox` → `{items}`: everything waiting on the owner, newest first.
async fn inbox(State(state): State<ServerState>) -> Reply {
    let items: Vec<Value> = inbox_items(&state)?.into_iter().map(|(_, item)| item).collect();
    Ok(Json(json!({ "items": items })).into_response())
}

/// The inbox, newest first, each item with the session it belongs to: the desktop inbox's own
/// read of waiting steps (one SQL statement) and the manager's pending permission requests.
fn inbox_items(state: &ServerState) -> Result<Vec<(Option<String>, Value)>, Response> {
    let steps = state.manager.store().waiting_workflow_steps().map_err(|error| server_error(&error))?;
    let mut items: Vec<(Option<String>, Value)> = steps.into_iter().map(|step| {
        let gate = step.state == "waiting-approval";
        let kind = match step.code.as_deref() {
            _ if gate => "gate",
            Some("loop-stopped") if step.reason.as_deref().is_some_and(|reason| reason.starts_with("Stopped after")) => "round_limit",
            Some("loop-stopped") => "loop_stalled",
            Some("review-report") => "invalid_review",
            Some("budget-exhausted") => "timeout",
            _ => "restart",
        };
        // The desktop inbox's buttons: approve a gate; another round, accept or stop a pause.
        let options = if gate {
            json!([{ "id": "approve", "label": "Approve", "style": "primary" }])
        } else {
            json!([{ "id": "another-round", "label": "Another round", "style": "primary" },
                { "id": "accept", "label": "Accept", "style": "neutral" }, { "id": "stop", "label": "Stop", "style": "danger" }])
        };
        let title = [step.task_key.as_deref(), step.workflow_name.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        (step.owned_id, json!({
            "id": format!("step:{}:{}", step.run_id, step.node_id), "kind": kind, "runId": step.run_id,
            "taskKey": step.task_key, "stepId": step.node_id, "title": if title.is_empty() { step.run_id.clone() } else { title },
            "detail": step.reason.unwrap_or_default(), "evidence": [], "options": options,
            "createdAt": step.updated_at_ms, "version": step.attempt,
        }))
    }).collect();
    items.extend(state.manager.pending_permissions().into_iter().map(|request| {
        let options: Vec<Value> = request.options.iter().map(|(id, label, kind)| json!({
            "id": id, "label": label,
            "style": if kind.starts_with("reject") { "danger" } else if kind == "allow_once" { "primary" } else { "neutral" },
        })).collect();
        (Some(request.owned_id.clone()), json!({
            "id": request.request_id, "kind": "permission",
            "sessionName": request.session_title.unwrap_or_else(|| request.owned_id.clone()),
            "title": request.title, "detail": request.command.unwrap_or_default(),
            "evidence": [{ "label": "Provider", "text": request.provider }], "options": options,
            "createdAt": request.requested_at_ms as i64, "version": request.generation,
        }))
    }));
    items.sort_by_key(|(_, item)| std::cmp::Reverse(item["createdAt"].as_i64()));
    Ok(items)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnswerRequest {
    option_id: String,
    version: i64,
}

/// `POST /phone/v1/inbox/{id}/answer {optionId, version}` → 204, or 409 `{error:"stale"}`. The option must be
/// one the item offers; the answer goes the way the desktop sends it, with the owner's origin. The
/// idempotency key names the item, its version and the option, so a repeat is recorded once and a
/// different answer to the same version is refused as stale instead of being taken for a repeat.
async fn answer(State(state): State<ServerState>, Path(id): Path<String>, body: Bytes) -> Reply {
    let request: AnswerRequest = parse(&body)?;
    let stale = || refuse(StatusCode::CONFLICT, "stale");
    let (session, item) = inbox_items(&state)?.into_iter().find(|(_, item)| item["id"] == id.as_str()).ok_or_else(stale)?;
    if item["version"] != request.version {
        return Err(stale());
    }
    if !item["options"].as_array().is_some_and(|options| options.iter().any(|option| option["id"] == request.option_id.as_str())) {
        return Err(refuse(StatusCode::BAD_REQUEST, "That option is not offered for this item"));
    }
    let key = format!("phone:{id}:{}:{}", request.version, request.option_id);
    let command = match item["kind"].as_str() {
        Some("permission") => RemoteCommand::RespondPermission(RespondAgentConversationPermissionRequest {
            owned_id: session.unwrap_or_default(), generation: request.version as u64, request_id: id, option_id: request.option_id,
        }),
        Some("gate") => RemoteCommand::Workflow { operation: "approveGate".into(), args: json!({
            "runId": item["runId"], "nodeId": item["stepId"], "approval": { "approved": true }, "expectedAttempt": request.version, "idempotencyKey": key }) },
        _ => RemoteCommand::Workflow { operation: "answer".into(), args: json!({
            "runId": item["runId"], "nodeId": item["stepId"], "decision": request.option_id, "expectedAttempt": request.version, "idempotencyKey": key }) },
    };
    match execute_phone_command(&state, command).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT.into_response()),
        // It was waiting a moment ago, so these refusals mean another answer got there first.
        Err(error) if answered_elsewhere(&error) => Err(stale()),
        Err(error) => Err(server_error(&error)),
    }
}

/// Whether an answer's refusal says the item is no longer waiting: the engine's "That step is not
/// waiting for the owner" or a lost write race, or the manager's settled or replaced request.
fn answered_elsewhere(error: &str) -> bool {
    match serde_json::from_str::<WorkflowError>(error) {
        Ok(WorkflowError::InvalidTransition(_) | WorkflowError::NotFound(_)) => true,
        Ok(WorkflowError::Ledger(message)) => message.starts_with("Stale workflow transition"),
        Ok(_) => false,
        Err(_) => ["Approval request is no longer pending", "Stale approval request", "Conversation connection changed"]
            .iter().any(|refusal| error.starts_with(refusal)),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushTokenRequest {
    fcm_token: String,
}

/// `POST /phone/v1/push-token {fcmToken}` → 204: this phone, and no other, gets the pushes for that token.
async fn push_token(State(state): State<ServerState>, Extension(device): Extension<Device>, body: Bytes) -> Reply {
    let request: PushTokenRequest = parse(&body)?;
    if request.fcm_token.trim().is_empty() || request.fcm_token.chars().count() > MAX_PUSH_TOKEN_CHARS {
        return Err(refuse(StatusCode::BAD_REQUEST, "The push token must be 1 to 4096 characters"));
    }
    state.manager.store().set_phone_push_token(&device.id, &request.fcm_token).map_err(|error| server_error(&error))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `GET /phone/v1/live`: a WebSocket of `chat`, `chatDelta`, `turn` and `changed` frames (§5.3).
async fn live(State(state): State<ServerState>, Extension(device): Extension<Device>,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>) -> Reply {
    let upgrade = upgrade.map_err(IntoResponse::into_response)?;
    // Subscribed before the token is checked again, so a removal at any moment after the first
    // check either fails this check or wakes the socket, which then checks and closes.
    let removed = DEVICE_REMOVED.subscribe();
    if state.manager.store().touch_phone_device(&device.token_hash, now_ms()).map_err(|error| server_error(&error))?.is_none() {
        return Err(refuse(StatusCode::UNAUTHORIZED, REMOVED));
    }
    let orchestrator = state.manager.store().main_orchestrator().map_err(|error| server_error(&error))?.map(|(owned_id, _)| owned_id);
    Ok(upgrade.max_message_size(MAX_BODY_BYTES)
        .on_upgrade(move |socket| serve_live(socket, state, device.token_hash, orchestrator, removed)))
}

/// Sends frames until the phone hangs up, falls behind, or is removed. Its three subscriptions are
/// dropped when this returns, which is the only place they are held. Chat frames are for the session
/// the phone last named in a `watch` frame, or the orchestrator marked at connect when it named none.
async fn serve_live(mut socket: WebSocket, state: ServerState, hash: String, orchestrator: Option<String>,
    mut removed: tokio::sync::watch::Receiver<u64>) {
    let mut watched = orchestrator.clone();
    let mut events = state.events.subscribe();
    let mut runs = state.workflow.run_changes();
    let mut waiting = HashMap::<String, usize>::new();
    loop {
        // A lagged or closed subscription ends the socket; the phone reconnects and refetches.
        let frames = tokio::select! {
            event = events.recv() => match event {
                Ok(event) => live_frames(&event, watched.as_deref()),
                Err(_) => break,
            },
            change = runs.recv() => match change {
                Ok((run_id, now)) => {
                    let inbox = waiting.insert(run_id.clone(), now) != Some(now);
                    let mut frames = vec![json!({ "type": "changed", "entity": "runs" }), json!({ "type": "changed", "entity": "run", "id": run_id })];
                    if inbox { frames.push(json!({ "type": "changed", "entity": "inbox" })); }
                    frames
                }
                Err(_) => break,
            },
            Ok(()) = removed.changed() => match state.manager.store().touch_phone_device(&hash, now_ms()) {
                Ok(Some(_)) => Vec::new(),
                _ => break,
            },
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(owned_id) = watch_frame(&text) { watched = owned_id.or_else(|| orchestrator.clone()); }
                    Vec::new()
                }
                Some(Ok(_)) => Vec::new(),
                _ => break,
            },
        };
        for frame in frames {
            if socket.send(Message::Text(frame.to_string().into())).await.is_err() { return; }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

/// The session a phone's `{"type":"watch","ownedId":<id or null>}` frame names: `Some(None)` for
/// null, `None` for any other frame, which is ignored.
fn watch_frame(text: &str) -> Option<Option<String>> {
    let frame: Value = serde_json::from_str(text).ok()?;
    (frame["type"] == "watch").then(|| frame["ownedId"].as_str().map(str::to_owned))
}

/// One agent event's live frames: chat, delta and turn frames for the watched session's own events,
/// and an inbox change for any permission request asked or settled in any session.
fn live_frames(event: &AgentConversationEvent, watched: Option<&str>) -> Vec<Value> {
    if matches!(event.payload, Payload::Approval { .. }) {
        return vec![json!({ "type": "changed", "entity": "inbox" })];
    }
    if watched != Some(event.owned_id.as_str()) {
        return Vec::new();
    }
    match &event.payload {
        Payload::AssistantDelta { item_id, delta } => vec![json!({ "type": "chatDelta", "seq": event.sequence, "itemId": item_id, "delta": delta })],
        Payload::Turn { state, .. } => vec![json!({ "type": "turn", "seq": event.sequence,
            "state": if matches!(state, TurnState::Started) { "running" } else { "idle" } })],
        _ => reduce_chat(std::slice::from_ref(event)).into_iter()
            .map(|item| json!({ "type": "chat", "seq": event.sequence, "item": item })).collect(),
    }
}

#[derive(Serialize)]
struct ChatItem {
    id: String,
    seq: i64,
    kind: &'static str,
    text: String,
    at: i64,
    done: bool,
    /// A tool call with output, a path or a diff the phone can open; the row itself carries none of them.
    #[serde(rename = "hasDetail", skip_serializing_if = "std::ops::Not::not")]
    has_detail: bool,
}

/// The phone's chat from a session's events (§5.3): owner, relayed, agent, tool and error items,
/// one per item id in first-seen order, the latest event winning. Deltas for an item with no full
/// message yet fold into one open agent item. Everything else is dropped.
fn reduce_chat(events: &[AgentConversationEvent]) -> Vec<ChatItem> {
    let mut items: Vec<ChatItem> = Vec::new();
    let mut index = HashMap::<String, usize>::new();
    let mut messaged = HashSet::<&str>::new();
    for event in events {
        let (id, kind, text, done) = match &event.payload {
            Payload::UserMessage { item_id, text, completed, .. } =>
                (item_id.clone(), if text.starts_with("[message from ") { "relayed" } else { "owner" }, text.clone(), *completed),
            Payload::AssistantMessage { item_id, text, completed, .. } => {
                messaged.insert(item_id);
                (item_id.clone(), "agent", text.clone(), *completed)
            }
            Payload::AssistantDelta { item_id, delta } if !messaged.contains(item_id.as_str()) => {
                if let Some(&at) = index.get(item_id) {
                    let item = &mut items[at];
                    item.text.push_str(delta);
                    (item.seq, item.at) = (event.sequence, event.timestamp_ms as i64);
                    continue;
                }
                (item_id.clone(), "agent", delta.clone(), false)
            }
            Payload::Tool { item_id, name, state, summary, .. } => (item_id.clone(), "tool",
                summary.clone().filter(|summary| !summary.is_empty()).unwrap_or_else(|| name.clone()),
                matches!(state, ToolState::Completed | ToolState::Failed)),
            Payload::Error { message, .. } => (format!("error-{}", event.sequence), "error", message.clone(), true),
            _ => continue,
        };
        let has_detail = matches!(&event.payload, Payload::Tool { output, path, diff, .. }
            if [output, path, diff].iter().any(|field| field.as_deref().is_some_and(|text| !text.is_empty())));
        let item = ChatItem { id, seq: event.sequence, kind, text, at: event.timestamp_ms as i64, done, has_detail };
        match index.get(&item.id) {
            Some(&at) => items[at] = item,
            None => {
                index.insert(item.id.clone(), items.len());
                items.push(item);
            }
        }
    }
    items
}

fn parse<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Response> {
    serde_json::from_slice(body).map_err(|_| refuse(StatusCode::BAD_REQUEST, "The request body is not valid"))
}

fn refuse(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

/// Logs the detail here and tells the caller only "Server error". Details never hold a token or code.
fn server_error(detail: &dyn std::fmt::Display) -> Response {
    eprintln!("Phone API error: {detail}");
    refuse(StatusCode::INTERNAL_SERVER_ERROR, "Server error")
}

/// Only this hash of a device token is stored.
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    std::fs::File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut bytes))
        .map_err(|error| format!("Could not read random bytes: {error}"))?;
    Ok(bytes)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::future::IntoFuture;
    use std::sync::{Arc, Mutex};

    use super::super::manager::AgentRuntimeManager;
    use super::super::providers::ProviderRegistry;
    use super::super::remote::{bind_listeners, server_state, ServerState};
    use super::{answered_elsewhere, execute_phone_command, new_pairing_code, operation, redeem_pairing_code, reduce_chat, router, start_listener, RemoteCommand, PAIRING_CODE_LIFETIME_MS};
    use super::super::protocol::{AgentConversationEvent, AgentConversationPayload as Payload, AgentConversationProvider,
        AgentNativeSessionMode, EnsureAgentConversationRequest, ExecutionEnvironment, ToolState, TurnState};
    use futures_util::StreamExt;
    use serde_json::{json, Value};
    use std::time::Duration;

    const PHONE_URL: &str = "https://phone.example.ts.net:8778";

    async fn phone_server() -> (ServerState, String) {
        phone_server_with(|_| ProviderRegistry::default()).await
    }

    /// A phone server whose providers `providers` makes from the server's data folder.
    async fn phone_server_with(providers: impl FnOnce(&std::path::Path) -> ProviderRegistry) -> (ServerState, String) {
        let directory = std::env::temp_dir().join(format!("assembly-phone-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let manager = AgentRuntimeManager::open(providers(&directory), &directory.join("sessions.db")).unwrap();
        let state = server_state(manager, directory, Arc::from("desktop-server-token-0123456789abcdef")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(axum::serve(listener, router(state.clone())).into_future());
        (state, format!("http://{address}"))
    }

    async fn pair(base: &str, code: &str) -> reqwest::Response {
        reqwest::Client::new().post(format!("{base}/phone/v1/pair"))
            .json(&serde_json::json!({"code": code, "deviceName": "Test phone"})).send().await.unwrap()
    }

    async fn me(base: &str, authorization: Option<&str>) -> reqwest::StatusCode {
        let request = reqwest::Client::new().get(format!("{base}/phone/v1/me"));
        let request = match authorization { Some(value) => request.header("authorization", value), None => request };
        request.send().await.unwrap().status()
    }

    fn pairing_code(state: &ServerState) -> String {
        // Edition 2021: setting the variable is safe; every phone test sets the same value.
        std::env::set_var("ASSEMBLY_PHONE_URL", PHONE_URL);
        let created = operation(state, "phonePairing", &serde_json::json!({"machineName": "Workbox Test"}))
            .expect("phonePairing is a phone operation").unwrap();
        assert_eq!(created["url"], PHONE_URL);
        assert_eq!(created["machineName"], "Workbox Test");
        created["code"].as_str().unwrap().to_string()
    }

    #[test]
    fn pairing_code_is_single_use_and_expires() {
        let slot = Mutex::new(None);
        let (code, expires_at) = new_pairing_code(&slot, 1_000).unwrap();
        assert_eq!(code.len(), 10);
        assert!(code.bytes().all(|byte| b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(&byte)));
        assert_eq!(expires_at, 1_000 + PAIRING_CODE_LIFETIME_MS);
        assert!(redeem_pairing_code(&slot, &code, 2_000));
        assert!(!redeem_pairing_code(&slot, &code, 2_000), "a code pairs one phone only");

        let (code, expires_at) = new_pairing_code(&slot, 1_000).unwrap();
        assert!(!redeem_pairing_code(&slot, &code, expires_at + 1), "an expired code is refused");

        let (old, _) = new_pairing_code(&slot, 1_000).unwrap();
        let (new, _) = new_pairing_code(&slot, 1_000).unwrap();
        assert!(!redeem_pairing_code(&slot, &old, 2_000), "a new code replaces the old one");
        assert!(redeem_pairing_code(&slot, &new, 2_000));
    }

    #[test]
    fn pairing_code_dies_after_five_wrong_tries() {
        let slot = Mutex::new(None);
        let (code, _) = new_pairing_code(&slot, 0).unwrap();
        for _ in 0..4 { assert!(!redeem_pairing_code(&slot, "WRONGWRONG", 1)); }
        assert!(redeem_pairing_code(&slot, &code, 1), "four wrong tries leave the code alive");

        let (code, _) = new_pairing_code(&slot, 0).unwrap();
        for _ in 0..5 { assert!(!redeem_pairing_code(&slot, "WRONGWRONG", 1)); }
        assert!(!redeem_pairing_code(&slot, &code, 1), "the fifth wrong try kills the code");
    }

    #[tokio::test]
    async fn phone_routes_reject_missing_unknown_and_revoked_tokens() {
        let (state, base) = phone_server().await;
        let code = pairing_code(&state);
        for device_name in ["  ", &"x".repeat(101)] {
            let refused = reqwest::Client::new().post(format!("{base}/phone/v1/pair"))
                .json(&serde_json::json!({"code": code, "deviceName": device_name})).send().await.unwrap();
            assert_eq!(refused.status(), reqwest::StatusCode::BAD_REQUEST, "a bad device name leaves the code unused");
        }
        let paired = pair(&base, &code).await;
        assert_eq!(paired.status(), reqwest::StatusCode::OK);
        let paired: serde_json::Value = paired.json().await.unwrap();
        let token = paired["deviceToken"].as_str().unwrap();
        assert_eq!(paired["machine"]["name"], "Workbox Test");

        assert_eq!(me(&base, None).await, reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(me(&base, Some("Bearer not-a-device-token")).await, reqwest::StatusCode::UNAUTHORIZED);
        let response = reqwest::Client::new().get(format!("{base}/phone/v1/me"))
            .bearer_auth(token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["deviceId"], paired["deviceId"]);
        assert_eq!(body["machine"], paired["machine"]);
        assert_eq!(body["hostsOrchestrator"], false);

        let devices = operation(&state, "phoneDevices", &serde_json::json!({})).unwrap().unwrap();
        assert_eq!(devices["devices"][0]["id"], paired["deviceId"]);
        assert_eq!(devices["devices"][0]["name"], "Test phone");
        operation(&state, "phoneRevoke", &serde_json::json!({"deviceId": paired["deviceId"]})).unwrap().unwrap();
        assert_eq!(me(&base, Some(&format!("Bearer {token}"))).await, reqwest::StatusCode::UNAUTHORIZED);
        let devices = operation(&state, "phoneDevices", &serde_json::json!({})).unwrap().unwrap();
        assert_eq!(devices["devices"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn phone_listener_does_not_serve_desktop_socket() {
        let (state, base) = phone_server().await;
        let response = reqwest::Client::new().get(format!("{base}/assembly"))
            .bearer_auth(&*state.token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        assert!(operation(&state, "list", &serde_json::json!({})).is_none(), "engine operations are not phone operations");
    }

    #[tokio::test]
    async fn invalid_phone_bind_does_not_stop_main_server() {
        let (state, _) = phone_server().await;
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let taken = taken.local_addr().unwrap().to_string();
        for bind in [None, Some(""), Some("not-an-address"), Some(taken.as_str())] {
            assert!(!start_listener(bind, state.clone()).await, "{bind:?} starts no phone listener and is not an error");
        }
        assert!(start_listener(Some("127.0.0.1:0"), state).await);
    }

    #[tokio::test]
    async fn phone_listener_overlapping_desktop_address_does_not_stop_desktop() {
        let (state, _) = phone_server().await;
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = free.local_addr().unwrap().to_string();
        drop(free);
        // The phone address is the desktop address: the desktop must still bind it.
        let desktop = bind_listeners(&address, Some(&address), &state).await
            .expect("the desktop listener binds even when the phone address overlaps it");
        assert_eq!(desktop.local_addr().unwrap().to_string(), address);
    }

    #[tokio::test]
    async fn desktop_server_token_is_not_a_phone_token() {
        let (state, base) = phone_server().await;
        assert_eq!(pair(&base, &pairing_code(&state)).await.status(), reqwest::StatusCode::OK);
        assert_eq!(me(&base, Some(&format!("Bearer {}", state.token))).await, reqwest::StatusCode::UNAUTHORIZED);
    }

    /// Pairs a phone with the test server and returns its device token.
    async fn paired(state: &ServerState, base: &str) -> String {
        pair(base, &pairing_code(state)).await.json::<Value>().await.unwrap()["deviceToken"].as_str().unwrap().to_string()
    }

    fn client(method: reqwest::Method, base: &str, path: &str, token: &str) -> reqwest::RequestBuilder {
        reqwest::Client::new().request(method, format!("{base}/phone/v1{path}")).bearer_auth(token)
    }

    /// Starts a run of the repository workflow `steps` in its own folder; returns its id.
    async fn start_run(state: &ServerState, steps: &str) -> String {
        let root = std::path::PathBuf::from("/var/tmp").join(format!("tsk-1416-phone-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join(".assembly/workflows")).unwrap();
        let path = std::path::Path::new(".assembly/workflows/phone.yml");
        std::fs::write(root.join(path), format!("name: Phone\nsteps:\n{steps}")).unwrap();
        let definition = crate::workflow::WorkflowEngine::validate_file(&root, path).unwrap();
        let run = state.workflow.create_run(definition, json!({"cwd": root, "worktree": {"path": root, "branch": "main", "removed": true},
            "run": {"proof_dir": root}, "task": {"key": "TSK-1", "title": "Phone test"}, "inputs": {}}),
            uuid::Uuid::new_v4().to_string()).unwrap();
        state.workflow.start(&run.id, "start").await.unwrap().id
    }

    const LOOP_STOPS: &str = "  prep: { run: [\"true\"] }\n  checks: { needs: [prep], run: [\"exit 1\"], on_fail: prep }\n";

    /// The inbox once it holds an item for `run_id`.
    async fn inbox_item(base: &str, token: &str, run_id: &str) -> Value {
        for _ in 0..200 {
            let inbox: Value = client(reqwest::Method::GET, base, "/inbox", token).send().await.unwrap().json().await.unwrap();
            if let Some(item) = inbox["items"].as_array().unwrap().iter().find(|item| item["runId"] == run_id) {
                return item.clone();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("run {run_id} never reached the inbox");
    }

    fn event(sequence: i64, payload: Payload) -> AgentConversationEvent {
        AgentConversationEvent { runtime_state: None, owned_id: "orchestrator".into(), provider: AgentConversationProvider::Claude,
            generation: 1, sequence, timestamp_ms: sequence as u128, turn_id: None, payload }
    }

    fn tool(state: ToolState, summary: Option<&str>) -> Payload {
        Payload::Tool { item_id: "t1".into(), name: "start_run".into(), state, summary: summary.map(str::to_string),
            output: None, path: None, diff: None }
    }

    #[test]
    fn phone_chat_keeps_owner_agent_relayed_tool_and_drops_the_rest() {
        let events = vec![
            event(1, Payload::UserMessage { item_id: "u1".into(), text: "Start the build".into(), completed: true, attachment_ids: vec![] }),
            event(2, Payload::UserMessage { item_id: "u2".into(), text: "[message from tsk-1412/build]\nDone".into(), completed: true, attachment_ids: vec![] }),
            event(3, Payload::Turn { turn_id: "turn".into(), state: TurnState::Started }),
            event(4, tool(ToolState::Started, None)),
            event(5, tool(ToolState::Completed, Some("2 runs started"))),
            event(6, Payload::Plan { items: vec![] }),
            event(7, Payload::AssistantMessage { item_id: "a1".into(), text: "On it.".into(), completed: true, blocks: None }),
            event(8, Payload::Usage { input_tokens: None, output_tokens: None, used_tokens: None, context_window: None, total_tokens: None }),
            event(9, Payload::Error { code: "provider".into(), message: "Provider failed".into(), recoverable: true }),
        ];
        assert_eq!(serde_json::to_value(reduce_chat(&events)).unwrap(), json!([
            {"id": "u1", "seq": 1, "kind": "owner", "text": "Start the build", "at": 1, "done": true},
            {"id": "u2", "seq": 2, "kind": "relayed", "text": "[message from tsk-1412/build]\nDone", "at": 2, "done": true},
            {"id": "t1", "seq": 5, "kind": "tool", "text": "2 runs started", "at": 5, "done": true},
            {"id": "a1", "seq": 7, "kind": "agent", "text": "On it.", "at": 7, "done": true},
            {"id": "error-9", "seq": 9, "kind": "error", "text": "Provider failed", "at": 9, "done": true},
        ]));
        assert_eq!(serde_json::to_value(reduce_chat(&events[3..4])).unwrap()[0]["text"], "start_run", "a tool with no summary shows its name");
    }

    #[test]
    fn phone_chat_folds_unfinished_deltas_into_open_agent_item() {
        let delta = |sequence, item: &str, text: &str| event(sequence, Payload::AssistantDelta { item_id: item.into(), delta: text.into() });
        let mut events = vec![
            event(1, Payload::AssistantMessage { item_id: "a1".into(), text: "Done.".into(), completed: true, blocks: None }),
            delta(2, "a1", " ignored"),
            delta(3, "a2", "Hel"),
            delta(4, "a2", "lo"),
        ];
        assert_eq!(serde_json::to_value(reduce_chat(&events)).unwrap(), json!([
            {"id": "a1", "seq": 1, "kind": "agent", "text": "Done.", "at": 1, "done": true},
            {"id": "a2", "seq": 4, "kind": "agent", "text": "Hello", "at": 4, "done": false},
        ]));
        events.push(event(5, Payload::AssistantMessage { item_id: "a2".into(), text: "Hello there".into(), completed: true, blocks: None }));
        assert_eq!(serde_json::to_value(reduce_chat(&events)).unwrap()[1],
            json!({"id": "a2", "seq": 5, "kind": "agent", "text": "Hello there", "at": 5, "done": true}));
    }

    #[tokio::test]
    async fn session_mcp_token_cannot_call_phone_routes() {
        let (state, base) = phone_server().await;
        let device = paired(&state, &base).await;
        let endpoint = super::super::messaging::MessagingEndpoint::start().unwrap();
        let session = endpoint.issue("orchestrator");
        use reqwest::Method;
        for (method, path) in [(Method::GET, "/me"), (Method::DELETE, "/me"), (Method::GET, "/chat"), (Method::POST, "/chat"),
            (Method::GET, "/runs"), (Method::GET, "/runs/r"), (Method::POST, "/runs/r/pause"), (Method::GET, "/inbox"),
            (Method::POST, "/inbox/i/answer"), (Method::GET, "/live"), (Method::GET, "/sessions"),
            (Method::GET, "/sessions/s/chat"), (Method::POST, "/sessions/s/chat"), (Method::GET, "/sessions/s/items/i"),
            (Method::GET, "/sessions/s/changes")] {
            let response = client(method.clone(), &base, path, &session.token).json(&json!({"text": "hi"})).send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED, "{method} {path}");
            assert_eq!(response.json::<Value>().await.unwrap(), json!({"error": "This phone was removed — pair again"}));
        }
        let mcp = reqwest::Client::new().post(&session.url).bearer_auth(&device)
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).send().await.unwrap();
        assert_eq!(mcp.status(), reqwest::StatusCode::UNAUTHORIZED, "a device token is not a session token");

        assert_eq!(client(Method::DELETE, &base, "/me", &device).send().await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        assert_eq!(me(&base, Some(&format!("Bearer {device}"))).await, reqwest::StatusCode::UNAUTHORIZED, "DELETE /me revokes the token");
    }

    #[tokio::test]
    async fn phone_answer_uses_owner_origin_once() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let run_id = start_run(&state, LOOP_STOPS).await;
        let item = inbox_item(&base, &token, &run_id).await;
        assert_eq!((item["kind"].as_str(), item["stepId"].as_str()), (Some("loop_stalled"), Some("checks")));
        assert_eq!(item["options"].as_array().unwrap().iter().map(|option| option["id"].as_str().unwrap()).collect::<Vec<_>>(),
            ["another-round", "accept", "stop"], "the desktop's own choices");
        let answer = || client(reqwest::Method::POST, &base, &format!("/inbox/{}/answer", item["id"].as_str().unwrap()), &token)
            .json(&json!({"optionId": "accept", "version": item["version"]}));
        assert_eq!(answer().send().await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        let again = answer().send().await.unwrap();
        assert_eq!(again.status(), reqwest::StatusCode::CONFLICT);
        assert_eq!(again.json::<Value>().await.unwrap(), json!({"error": "stale"}));
        // The engine takes an answer only from the owner, and recorded this one once.
        let run = state.workflow.get_run(&run_id).unwrap();
        assert_eq!(run.input_snapshot["answers"], json!([{"step": "checks", "decision": "accept", "reply": null}]));
        assert_eq!(serde_json::to_value(run.state).unwrap(), "completed");
    }

    #[tokio::test]
    async fn phone_answer_with_unknown_option_or_stale_version_is_rejected() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let run_id = start_run(&state, LOOP_STOPS).await;
        let item = inbox_item(&base, &token, &run_id).await;
        let id = item["id"].as_str().unwrap();
        let version = item["version"].as_i64().unwrap();
        for (path, option, version, status) in [
            (id, "approve", version, reqwest::StatusCode::BAD_REQUEST),
            (id, "accept", version + 1, reqwest::StatusCode::CONFLICT),
            ("step:no-such-run:checks", "accept", version, reqwest::StatusCode::CONFLICT),
        ] {
            let response = client(reqwest::Method::POST, &base, &format!("/inbox/{path}/answer"), &token)
                .json(&json!({"optionId": option, "version": version})).send().await.unwrap();
            assert_eq!(response.status(), status, "{path} {option} {version}");
        }
        assert_eq!(inbox_item(&base, &token, &run_id).await, item, "nothing was answered");
        assert!(state.workflow.get_run(&run_id).unwrap().input_snapshot.get("answers").is_none());
    }

    #[tokio::test]
    async fn answer_with_old_attempt_is_stale_after_another_round() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let run_id = start_run(&state, LOOP_STOPS).await;
        let old = inbox_item(&base, &token, &run_id).await;
        // The desktop sends another round, and the step stops again as a newer attempt.
        crate::workflow::execute_operation(&state.workflow, "answer", json!({"runId": run_id, "nodeId": "checks",
            "decision": "another-round", "idempotencyKey": "desktop:round"})).await.unwrap();
        let mut item = inbox_item(&base, &token, &run_id).await;
        for _ in 0..200 {
            if item["version"] != old["version"] { break; }
            tokio::time::sleep(Duration::from_millis(50)).await;
            item = inbox_item(&base, &token, &run_id).await;
        }
        assert!(item["version"].as_i64() > old["version"].as_i64(), "the step is waiting again as a newer attempt");

        // The phone's own version check refuses it ...
        let refused = client(reqwest::Method::POST, &base, &format!("/inbox/{}/answer", old["id"].as_str().unwrap()), &token)
            .json(&json!({"optionId": "accept", "version": old["version"]})).send().await.unwrap();
        assert_eq!(refused.status(), reqwest::StatusCode::CONFLICT);
        // ... and so does the engine, for an answer that passed that check before the round.
        let late = execute_phone_command(&state, RemoteCommand::Workflow { operation: "answer".into(), args: json!({
            "runId": run_id, "nodeId": "checks", "decision": "accept", "expectedAttempt": old["version"], "idempotencyKey": "phone:late" }) }).await;
        assert!(answered_elsewhere(&late.err().expect("an answer to an older attempt is refused")));
        assert_eq!(inbox_item(&base, &token, &run_id).await["version"], item["version"], "the newer attempt still waits");
        assert_eq!(serde_json::to_value(state.workflow.get_run(&run_id).unwrap().state).unwrap(), "waiting-input");
    }

    #[tokio::test]
    async fn phone_run_controls_match_desktop_engine_calls() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let steps = "  wait: { run: [\"sleep 5\"] }\n";
        let (phone, desktop) = (start_run(&state, steps).await, start_run(&state, steps).await);
        for (action, operation, expected) in [("pause", "pause", "paused"), ("resume", "resume", "running"), ("stop", "cancel", "cancelled")] {
            let row: Value = client(reqwest::Method::POST, &base, &format!("/runs/{phone}/{action}"), &token)
                .send().await.unwrap().json().await.unwrap();
            assert_eq!((row["id"].as_str(), row["state"].as_str()), (Some(phone.as_str()), Some(expected)), "{action}");
            let desktop_run = crate::workflow::execute_operation(&state.workflow, operation,
                json!({"runId": desktop, "idempotencyKey": format!("desktop:{action}")})).await.unwrap();
            assert_eq!(desktop_run["state"], expected, "{operation}");
        }
        let lines = |run_id: &str| state.workflow.run_timeline(run_id).unwrap().into_iter().map(|(_, line)| line).collect::<Vec<_>>();
        assert_eq!(lines(&phone), lines(&desktop), "the phone made the same engine calls");
        assert_eq!(lines(&phone).last().map(String::as_str), Some("run cancelled"));
        let unknown = client(reqwest::Method::POST, &base, &format!("/runs/{phone}/merge"), &token).send().await.unwrap();
        assert_eq!(unknown.status(), reqwest::StatusCode::NOT_FOUND);
        let refused = client(reqwest::Method::POST, &base, &format!("/runs/{phone}/pause"), &token).send().await.unwrap();
        assert_eq!(refused.status(), reqwest::StatusCode::CONFLICT);
        assert_eq!(refused.json::<Value>().await.unwrap(), json!({"error": "Only a running workflow can be paused"}), "the engine's own words");

        let runs: Value = client(reqwest::Method::GET, &base, "/runs", &token).send().await.unwrap().json().await.unwrap();
        assert_eq!(runs["active"], json!([]));
        assert_eq!(runs["recent"].as_array().unwrap().len(), 2);
        let detail: Value = client(reqwest::Method::GET, &base, &format!("/runs/{phone}"), &token).send().await.unwrap().json().await.unwrap();
        assert_eq!((detail["taskKey"].as_str(), detail["state"].as_str()), (Some("TSK-1"), Some("cancelled")));
        assert_eq!(detail["steps"][0]["id"], "wait");
        assert_eq!(detail["steps"][0]["attempts"][0]["n"], 1);
        assert_eq!(detail["timeline"].as_array().unwrap().len(), lines(&phone).len());
    }

    #[tokio::test]
    async fn phone_live_sends_changed_inbox_when_item_created() {
        use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let mut request = format!("{}/phone/v1/live", base.replacen("http", "ws", 1)).into_client_request().unwrap();
        request.headers_mut().insert("authorization", format!("Bearer {token}").parse().unwrap());
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let run_id = start_run(&state, "  ask: { gate: { prompt: \"Continue?\", options: [\"yes\", \"no\"] } }\n").await;
        let mut seen = Vec::new();
        while !seen.contains(&json!({"type": "changed", "entity": "inbox"})) {
            let Message::Text(text) = tokio::time::timeout(Duration::from_secs(10), socket.next()).await
                .expect("a changed inbox frame").unwrap().unwrap() else { continue };
            seen.push(serde_json::from_str::<Value>(&text).unwrap());
        }
        assert!(seen.contains(&json!({"type": "changed", "entity": "run", "id": run_id})));
        assert_eq!(inbox_item(&base, &token, &run_id).await["kind"], "gate");

        // Removing the phone closes its live socket.
        assert_eq!(client(reqwest::Method::DELETE, &base, "/me", &token).send().await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        loop {
            match tokio::time::timeout(Duration::from_secs(10), socket.next()).await.expect("the socket closes") {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => continue,
            }
        }
    }

    #[tokio::test]
    async fn phone_live_sends_changed_inbox_for_run_waiting_before_connect() {
        use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let run_id = start_run(&state, LOOP_STOPS).await;
        inbox_item(&base, &token, &run_id).await;
        let mut request = format!("{}/phone/v1/live", base.replacen("http", "ws", 1)).into_client_request().unwrap();
        request.headers_mut().insert("authorization", format!("Bearer {token}").parse().unwrap());
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        // The step was already waiting when the phone connected; answering it on the desktop empties the inbox.
        crate::workflow::execute_operation(&state.workflow, "answer", json!({"runId": run_id, "nodeId": "checks",
            "decision": "accept", "idempotencyKey": "desktop:accept"})).await.unwrap();
        let mut seen = Vec::new();
        while !seen.contains(&json!({"type": "changed", "entity": "inbox"})) {
            let Message::Text(text) = tokio::time::timeout(Duration::from_secs(3), socket.next()).await
                .expect("a changed inbox frame for the answered step").unwrap().unwrap() else { continue };
            seen.push(serde_json::from_str::<Value>(&text).unwrap());
        }
    }

    #[tokio::test]
    async fn unauthenticated_post_with_body_is_refused_before_reading_it() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (_state, base) = phone_server().await;
        let address = base.trim_start_matches("http://");
        for path in ["/phone/v1/chat", "/phone/v1/inbox/step:run:checks/answer"] {
            // Headers promise a 60,000-byte body that never comes: a server that read the body
            // before checking the token would wait for it instead of answering.
            let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
            socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer not-a-device-token\r\n\
                Content-Type: application/json\r\nContent-Length: 60000\r\n\r\n{{\"text\":").as_bytes()).await.unwrap();
            let mut head = [0u8; 12];
            tokio::time::timeout(Duration::from_secs(3), socket.read_exact(&mut head)).await
                .expect("refused without waiting for the body").unwrap();
            assert_eq!(&head, b"HTTP/1.1 401", "{path}");
        }
    }

    #[tokio::test]
    async fn replayed_answer_with_different_option_is_stale() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        // Two answers to one item at once, with different options: exactly one is applied and
        // the other is refused as stale, never accepted without being applied.
        for _ in 0..8 {
            let run_id = start_run(&state, LOOP_STOPS).await;
            let item = inbox_item(&base, &token, &run_id).await;
            let send = |option: &'static str| client(reqwest::Method::POST, &base, &format!("/inbox/{}/answer", item["id"].as_str().unwrap()), &token)
                .json(&json!({"optionId": option, "version": item["version"]})).send();
            let (accept, stop) = tokio::join!(send("accept"), send("stop"));
            let (accept, stop) = (accept.unwrap().status(), stop.unwrap().status());
            let run = state.workflow.get_run(&run_id).unwrap();
            let state_now = serde_json::to_value(run.state).unwrap();
            match (accept, stop) {
                (reqwest::StatusCode::NO_CONTENT, reqwest::StatusCode::CONFLICT) => assert_eq!(state_now, "completed"),
                (reqwest::StatusCode::CONFLICT, reqwest::StatusCode::NO_CONTENT) => assert_eq!(state_now, "cancelled"),
                other => panic!("one answer must win and the other be stale, got {other:?} with the run {state_now}"),
            }
        }
    }

    #[tokio::test]
    async fn push_token_moves_off_other_device_rows() {
        let (state, base) = phone_server().await;
        let (a, b) = (paired(&state, &base).await, paired(&state, &base).await);
        let set = |token: &str, fcm: &str| client(reqwest::Method::POST, &base, "/push-token", token).json(&json!({"fcmToken": fcm})).send();
        assert_eq!(set(&a, "fcm-1").await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        assert_eq!(state.manager.store().phone_push_tokens().unwrap(), ["fcm-1"]);
        assert_eq!(set(&b, "fcm-1").await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        assert_eq!(state.manager.store().phone_push_tokens().unwrap(), ["fcm-1"], "one row holds it");
        // Removing B must leave nothing: A no longer holds the token.
        assert_eq!(client(reqwest::Method::DELETE, &base, "/me", &b).send().await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        assert!(state.manager.store().phone_push_tokens().unwrap().is_empty());
        for bad in ["".to_string(), "  ".to_string(), "x".repeat(4097)] {
            assert_eq!(set(&a, &bad).await.unwrap().status(), reqwest::StatusCode::BAD_REQUEST, "{} chars", bad.len());
        }
        assert_eq!(set(&a, &"x".repeat(4096)).await.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
        let stranger = reqwest::Client::new().post(format!("{base}/phone/v1/push-token")).json(&json!({"fcmToken": "x"})).send().await.unwrap();
        assert_eq!(stranger.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn inbox_push_missing_service_account_skips_without_error() {
        let (state, _) = phone_server().await;
        std::env::set_var("ASSEMBLY_FCM_SERVICE_ACCOUNT", "/var/tmp/tsk-1416-no-such-service-account.json");
        assert!(start_listener(Some("127.0.0.1:0"), state).await, "the phone listener starts without a key");
        std::env::remove_var("ASSEMBLY_FCM_SERVICE_ACCOUNT");
    }

    #[tokio::test]
    async fn inbox_push_rings_once_per_new_item() {
        use super::super::phone_push::tests::{fake_fcm, test_sender};
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let set = client(reqwest::Method::POST, &base, "/push-token", &token).json(&json!({"fcmToken": "fcm-1"})).send().await.unwrap();
        assert_eq!(set.status(), reqwest::StatusCode::NO_CONTENT);
        let fake = fake_fcm(200, "{}").await;
        super::spawn_push(state.clone(), test_sender(&fake));
        let run_id = start_run(&state, LOOP_STOPS).await;
        let item = inbox_item(&base, &token, &run_id).await;
        for _ in 0..200 {
            if !fake.messages.lock().unwrap().is_empty() { break; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // Further run changes (a second run that finishes) must not ring for the first item again.
        let other = start_run(&state, "  ok: { run: [\"true\"] }\n").await;
        for _ in 0..200 {
            if serde_json::to_value(state.workflow.get_run(&other).unwrap().state).unwrap() == "completed" { break; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let messages = fake.messages.lock().unwrap();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(messages[0]["message"]["data"]["itemId"], item["id"]);
        assert_eq!(messages[0]["message"]["token"], "fcm-1");
    }

    #[tokio::test]
    async fn phone_sessions_lists_this_machines_sessions_newest_first() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let store = state.manager.store();
        for (owned_id, title, activity) in [("old", "Old lane", 100), ("orch", "Orchestrator", 200), ("new", "New lane", 300)] {
            store.upsert_session(&mcb_core::session_store::SessionRow { owned_id: owned_id.into(), native_session_id: None,
                provider: "codex".into(), model: None, effort: None, cwd: "/work/assembly".into(), worktree: None, branch: None,
                title: Some(title.into()), title_source: None, project: Some("assembly".into()), project_id: None,
                state: "ready".into(), suspended: false, created_at_ms: 1, last_activity_at_ms: activity, extra_json: "{}".into() }).unwrap();
        }
        store.set_main_orchestrator(Some("orch")).unwrap();
        // Another machine's session cached here is newer, but it is not this machine's.
        store.file_remote_sessions("other-machine", "{}", r#"[["cached", null, 999]]"#).unwrap();

        let body: Value = client(reqwest::Method::GET, &base, "/sessions", &token).send().await.unwrap().json().await.unwrap();
        let sessions = body["sessions"].as_array().unwrap();
        assert_eq!(sessions.iter().map(|session| session["ownedId"].as_str().unwrap()).collect::<Vec<_>>(), ["new", "orch", "old"]);
        assert_eq!(sessions.iter().map(|session| session["orchestrator"].as_bool().unwrap()).collect::<Vec<_>>(), [false, true, false]);
        let newest = &sessions[0];
        assert_eq!((newest["name"].as_str(), newest["provider"].as_str(), newest["project"].as_str(), newest["state"].as_str()),
            (Some("New lane"), Some("codex"), Some("assembly"), Some("idle")));
        assert_eq!((newest["lastActivityAt"].as_i64(), &newest["runId"], &newest["stepId"]), (Some(300), &Value::Null, &Value::Null));
        assert!(newest["handle"].as_str().is_some_and(|handle| !handle.is_empty()));
    }

    #[tokio::test]
    async fn phone_chat_with_any_session_reads_and_sends() {
        let (state, base) = phone_server_with(|directory| {
            let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(&directory.join("acp.jsonl"), "default");
            ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)]).unwrap()
        }).await;
        let token = paired(&state, &base).await;
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        for owned_id in ["orchestrator", "lane"] {
            state.manager.ensure_async(EnsureAgentConversationRequest { owned_id: owned_id.into(),
                execution_environment: ExecutionEnvironment::Local, remote_profile_id: None, provider: AgentConversationProvider::Codex,
                cwd: cwd.clone(), native_session_id: None,
                native_session_mode: AgentNativeSessionMode::Resume, reasoning_effort: None, project_id: None }).await.unwrap();
        }
        state.manager.store().set_main_orchestrator(Some("orchestrator")).unwrap();

        let sent = client(reqwest::Method::POST, &base, "/sessions/lane/chat", &token)
            .json(&json!({"text": "Reply with the single word: pong"})).send().await.unwrap();
        assert_eq!(sent.status(), reqwest::StatusCode::OK);
        let item_id = sent.json::<Value>().await.unwrap()["itemId"].clone();
        let mut chat = Value::Null;
        for _ in 0..200 {
            chat = client(reqwest::Method::GET, &base, "/sessions/lane/chat", &token).send().await.unwrap().json().await.unwrap();
            if chat["items"].as_array().unwrap().iter().any(|item| item["kind"] == "agent" && item["done"] == true) { break; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let items: Vec<(&str, &str)> = chat["items"].as_array().unwrap().iter()
            .map(|item| (item["kind"].as_str().unwrap(), item["text"].as_str().unwrap())).collect();
        assert_eq!(items, [("owner", "Reply with the single word: pong"), ("agent", "generated text")], "{chat}");
        assert_eq!(chat["items"][0]["id"], item_id, "the send's own item");
        let orchestrator: Value = client(reqwest::Method::GET, &base, "/chat", &token).send().await.unwrap().json().await.unwrap();
        assert_eq!(orchestrator["items"], json!([]), "the orchestrator's chat holds nothing of it");
        for method in [reqwest::Method::GET, reqwest::Method::POST] {
            let unknown = client(method.clone(), &base, "/sessions/nobody/chat", &token).json(&json!({"text": "hi"})).send().await.unwrap();
            assert_eq!(unknown.status(), reqwest::StatusCode::NOT_FOUND, "{method}");
        }
        state.manager.shutdown().await;
    }

    /// Files session `owned_id` in the store with `payloads` as its events, numbered from 1.
    fn file_session(state: &ServerState, owned_id: &str, payloads: Vec<Payload>) {
        let store = state.manager.store();
        store.upsert_session(&mcb_core::session_store::SessionRow { owned_id: owned_id.into(), native_session_id: None,
            provider: "claude".into(), model: None, effort: None, cwd: "/work".into(), worktree: None, branch: None,
            title: Some("Lane".into()), title_source: None, project: None, project_id: None,
            state: "ready".into(), suspended: false, created_at_ms: 1, last_activity_at_ms: 1, extra_json: "{}".into() }).unwrap();
        for (index, payload) in payloads.into_iter().enumerate() {
            let event = session_event(owned_id, index as i64 + 1, payload);
            store.append_event(&mcb_core::session_store::EventRow { owned_id: owned_id.into(), seq: event.sequence, turn_id: None,
                kind: "tool".into(), payload_json: serde_json::to_string(&event).unwrap(), created_at_ms: 0 }).unwrap();
        }
    }

    fn edit(output: Option<String>, diff: Option<String>) -> Payload {
        Payload::Tool { item_id: "edit".into(), name: "Edit".into(), state: ToolState::Completed, summary: Some("notes.txt".into()),
            output, path: Some("/work/notes.txt".into()), diff }
    }

    #[tokio::test]
    async fn phone_tool_item_returns_output_path_and_diff() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let diff = "--- a/notes.txt\n+++ b/notes.txt\n@@ -0,0 +1,2 @@\n+one\n+two\n";
        file_session(&state, "lane", vec![
            Payload::Tool { item_id: "edit".into(), name: "Edit".into(), state: ToolState::Started, summary: None,
                output: None, path: None, diff: None },
            edit(Some("Wrote 2 lines".into()), Some(diff.into())),
            tool(ToolState::Completed, Some("wc -l notes.txt")),
        ]);
        let body: Value = client(reqwest::Method::GET, &base, "/sessions/lane/items/edit", &token).send().await.unwrap()
            .json().await.unwrap();
        assert_eq!(body, json!({"name": "Edit", "state": "completed", "summary": "notes.txt", "path": "/work/notes.txt",
            "output": "Wrote 2 lines", "outputTruncated": false, "diff": diff, "diffTruncated": false}), "the latest event wins");

        let chat: Value = client(reqwest::Method::GET, &base, "/sessions/lane/chat", &token).send().await.unwrap().json().await.unwrap();
        let details: Vec<(&str, bool)> = chat["items"].as_array().unwrap().iter()
            .map(|item| (item["id"].as_str().unwrap(), item["hasDetail"].as_bool().unwrap_or(false))).collect();
        assert_eq!(details, [("edit", true), ("t1", false)], "only a tool with something to open has detail");
        assert!(chat["items"][0].get("output").is_none(), "chat rows carry no output");
    }

    #[tokio::test]
    async fn phone_tool_item_caps_large_output() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        // "é" is two bytes, and the leading "x" puts the 32 KiB mark inside one.
        let output = format!("x{}", "é".repeat(20 * 1024));
        file_session(&state, "lane", vec![edit(Some(output.clone()), Some("+".repeat(70 * 1024)))]);
        let body: Value = client(reqwest::Method::GET, &base, "/sessions/lane/items/edit", &token).send().await.unwrap()
            .json().await.unwrap();
        let cut = body["output"].as_str().unwrap();
        assert_eq!(cut.len(), 32 * 1024 - 1, "cut on the char boundary below 32 KiB");
        assert!(output.starts_with(cut));
        assert_eq!(body["outputTruncated"], true);
        assert_eq!(body["diff"].as_str().unwrap().len(), 64 * 1024);
        assert_eq!(body["diffTruncated"], true);
    }

    #[tokio::test]
    async fn phone_tool_item_unknown_session_or_item_is_404() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        file_session(&state, "lane", vec![edit(None, None),
            Payload::AssistantMessage { item_id: "a1".into(), text: "Done".into(), completed: true, blocks: None }]);
        for (path, error) in [("/sessions/nobody/items/edit", "No such session on this machine"),
            ("/sessions/lane/items/missing", "No such tool call in this session"), ("/sessions/lane/items/a1", "No such tool call in this session")] {
            let response = client(reqwest::Method::GET, &base, path, &token).send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND, "{path}");
            assert_eq!(response.json::<Value>().await.unwrap(), json!({"error": error}), "{path}");
        }
    }

    /// A git repository with `docs/notes.txt` committed on `main`, then changed in the working tree;
    /// with `origin/HEAD` pointing at `main` when `origin_head` is set. Files session `owned_id` with it as its worktree.
    fn session_in_repo(state: &ServerState, owned_id: &str, origin_head: bool) -> std::path::PathBuf {
        let repo = std::path::PathBuf::from("/var/tmp").join(format!("assembly-phone-changes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(repo.join("docs")).unwrap();
        let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&repo)
            .args(["-c", "user.name=T", "-c", "user.email=t@example.invalid"]).args(args).status().unwrap().success(), "git {args:?}");
        git(&["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("docs/notes.txt"), "one\ntwo\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "initial"]);
        if origin_head {
            git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
            git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
        }
        std::fs::write(repo.join("docs/notes.txt"), "one\nthree\nfour\n").unwrap();
        file_session(state, owned_id, vec![]);
        let store = state.manager.store();
        let row = store.get_session(owned_id).unwrap().unwrap();
        store.upsert_session(&mcb_core::session_store::SessionRow { worktree: Some(repo.to_string_lossy().into()), ..row }).unwrap();
        repo
    }

    #[tokio::test]
    async fn phone_changes_lists_files_with_counts_and_diff() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let repo = session_in_repo(&state, "lane", true);
        let response = client(reqwest::Method::GET, &base, "/sessions/lane/changes", &token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["base"], "origin/main");
        assert_eq!(body["files"], json!([{"path": "docs/notes.txt", "added": 2, "removed": 1}]));
        let diff = body["diff"].as_str().unwrap();
        assert!(diff.starts_with("diff --git a/docs/notes.txt b/docs/notes.txt") && diff.contains("+three\n+four\n"), "{diff}");
        assert_eq!(body["diffTruncated"], false);
        std::fs::remove_dir_all(repo).unwrap();
    }

    #[tokio::test]
    async fn phone_changes_without_origin_head_is_409_with_message() {
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        let repo = session_in_repo(&state, "lane", false);
        let response = client(reqwest::Method::GET, &base, "/sessions/lane/changes", &token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
        assert_eq!(response.json::<Value>().await.unwrap(),
            json!({"error": "This repository has no origin/HEAD, so there is no default branch to compare with."}));
        let response = client(reqwest::Method::GET, &base, "/sessions/nobody/changes", &token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        std::fs::remove_dir_all(repo).unwrap();
    }

    type LiveClient = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

    async fn next_frame(socket: &mut LiveClient, wait: Duration) -> Option<Value> {
        match tokio::time::timeout(wait, socket.next()).await {
            Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text)))) => Some(serde_json::from_str(&text).unwrap()),
            _ => None,
        }
    }

    fn session_event(owned_id: &str, sequence: i64, payload: Payload) -> AgentConversationEvent {
        AgentConversationEvent { owned_id: owned_id.into(), ..event(sequence, payload) }
    }

    fn message(owned_id: &str, sequence: i64, item: &str) -> AgentConversationEvent {
        session_event(owned_id, sequence, Payload::AssistantMessage { item_id: item.into(), text: "hi".into(), completed: true, blocks: None })
    }

    /// Sends `owned_id`'s message `item` until its chat frame arrives: the socket may not have
    /// subscribed, or read the phone's watch frame, yet.
    async fn chat_frame_arrives(state: &ServerState, socket: &mut LiveClient, owned_id: &str, item: &str) {
        for sequence in 1..100 {
            let _ = state.events.send(message(owned_id, sequence, item));
            if next_frame(socket, Duration::from_millis(100)).await.is_some_and(|frame| frame["item"]["id"] == item) { return; }
        }
        panic!("no chat frame for {owned_id}");
    }

    #[tokio::test]
    async fn phone_live_watch_switches_chat_frames_to_that_session() {
        use futures_util::SinkExt;
        use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
        let (state, base) = phone_server().await;
        let token = paired(&state, &base).await;
        state.manager.store().set_main_orchestrator(Some("orchestrator")).unwrap();
        let mut request = format!("{}/phone/v1/live", base.replacen("http", "ws", 1)).into_client_request().unwrap();
        request.headers_mut().insert("authorization", format!("Bearer {token}").parse().unwrap());
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        // With no watch frame, the orchestrator's chat comes, as before.
        chat_frame_arrives(&state, &mut socket, "orchestrator", "o1").await;

        socket.send(Message::Text(json!({"type": "watch", "ownedId": "lane"}).to_string().into())).await.unwrap();
        chat_frame_arrives(&state, &mut socket, "lane", "l1").await;
        let started = || Payload::Turn { turn_id: "turn".into(), state: TurnState::Started };
        for event in [message("orchestrator", 1000, "o2"), session_event("orchestrator", 1001, started()),
            session_event("lane", 2000, started()),
            session_event("lane", 2001, Payload::AssistantDelta { item_id: "l2".into(), delta: "h".into() }), message("lane", 2002, "l2")] {
            state.events.send(event).unwrap();
        }
        let mut seen = Vec::new();
        while seen.last().is_none_or(|frame: &Value| frame["item"]["id"] != "l2") {
            seen.push(next_frame(&mut socket, Duration::from_secs(5)).await.expect("the lane's frames"));
        }
        // Frames below 1000 are late copies of the messages sent while waiting.
        let frames: Vec<(&str, i64)> = seen.iter().filter(|frame| frame["seq"].as_i64() >= Some(1000))
            .map(|frame| (frame["type"].as_str().unwrap(), frame["seq"].as_i64().unwrap())).collect();
        assert_eq!(frames, [("turn", 2000), ("chatDelta", 2001), ("chat", 2002)], "only the watched session's frames");

        // Unknown frames are ignored, and watching null goes back to the orchestrator.
        socket.send(Message::Text("not json".into())).await.unwrap();
        socket.send(Message::Text(json!({"type": "hello"}).to_string().into())).await.unwrap();
        socket.send(Message::Text(json!({"type": "watch", "ownedId": null}).to_string().into())).await.unwrap();
        chat_frame_arrives(&state, &mut socket, "orchestrator", "o3").await;
    }
}
