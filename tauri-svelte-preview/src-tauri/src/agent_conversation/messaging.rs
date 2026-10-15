use agent_client_protocol::schema::v1 as acp;
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::post, Json, Router};
use serde_json::{json, Value};
use mcb_core::{broker::MessageKind, session_store::SessionStore};
use super::{manager::AgentRuntimeManager, protocol::AgentConversationProvider};
use std::{collections::HashMap, future::Future, pin::Pin, sync::{Arc, Mutex, OnceLock, Weak}};

/// Carries one messaging operation (`directory`, `enqueue`, `status`) to the
/// other machines. The desktop routes it itself; a remote server asks the
/// desktop over its socket.
pub type Relay = Arc<dyn Fn(String, Value) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>> + Send + Sync>;

/// `brisk-heron@workbox-test`, or a received label `brisk-heron@workbox-test · Title`, as
/// the session and the machine it names. Anything else names a session on any machine.
pub fn split_address(to: &str) -> (&str, Option<&str>) {
    let handle = to.split(" · ").next().unwrap_or(to);
    match handle.rsplit_once('@') {
        Some((name, machine)) if !name.is_empty() && !name.contains(char::is_whitespace) && !machine.is_empty()
            && machine.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') => (name, Some(machine)),
        _ => (to, None),
    }
}

/// One messaging operation on this machine's own sessions, asked for by the desktop's router.
pub fn answer(store: &SessionStore, manager: Option<&AgentRuntimeManager>, op: &str, args: &Value) -> Result<Value, String> {
    let arg = |key: &str| args[key].as_str().ok_or_else(|| format!("{key} is required"));
    match op {
        "directory" => serde_json::to_value(store.list_directory().map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string()),
        "resolve" => Ok(json!(store.resolve_session_name(arg("to")?).map_err(|error| error.to_string())?)),
        "enqueue" => {
            let target = store.resolve_session_name(arg("to")?).map_err(|error| error.to_string())?;
            let message = store.append_message_as("", arg("from")?, &target, MessageKind::Message, arg("text")?, Some(arg("sender")?))
                .map_err(|error| error.to_string())?;
            if let Some(manager) = manager.cloned() {
                tauri::async_runtime::spawn(async move { manager.deliver_if_idle(&target).await });
            }
            Ok(json!(message.id))
        }
        "status" => Ok(json!(store.message_status(arg("id")?, arg("from")?).map_err(|error| error.to_string())?)),
        // The owner's main orchestrator mark on this machine: a session here, or none.
        "designate" => {
            let owned_id = args["owned_id"].as_str();
            if let Some(owned_id) = owned_id {
                store.get_session(owned_id).map_err(|error| error.to_string())?.ok_or("Unknown session")?;
            }
            store.set_main_orchestrator(owned_id).map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
        _ => Err(format!("Unknown messaging operation {op}")),
    }
}

pub struct SessionCredential { pub url: String, pub token: String }
impl SessionCredential {
    pub fn server(&self) -> acp::McpServer {
        acp::McpServer::Http(acp::McpServerHttp::new("assembly", &self.url)
            .headers(vec![acp::HttpHeader::new("Authorization", format!("Bearer {}", self.token))]))
    }
}

/// The workflow format reference agents read before writing a workflow.
const WORKFLOW_FORMAT: &str = include_str!("../../../../docs/workflows/format.md");

fn text(value: impl Into<String>) -> Value {
    json!({"content":[{"type":"text","text":value.into()}]})
}

fn text_json(value: Value) -> Value {
    text(value.to_string())
}

/// `ok`, then one line per step: id, kind, needs, and review/redo targets.
fn workflow_summary(definition: &crate::workflow::WorkflowDefinitionV1) -> String {
    let mut lines = vec!["ok".to_string()];
    for node in &definition.nodes {
        let kind = ["agent", "run", "review", "gate"].into_iter()
            .find(|kind| node.step.get(*kind).is_some()).unwrap_or("agent");
        let mut line = format!("{} {kind}", node.id);
        if !node.depends_on.is_empty() { line += &format!(" needs={}", node.depends_on.join(",")); }
        if let Some(of) = node.step["review"]["of"].as_str() { line += &format!(" of={of}"); }
        if let Some(on_fail) = node.step["on_fail"].as_str() { line += &format!(" on_fail={on_fail}"); }
        lines.push(line);
    }
    // A redone step only learns why from these placeholders in its own prompt.
    for node in &definition.nodes {
        let redo = [(node.step["on_fail"].as_str(), "redo.output", "failure"),
            (node.step["review"]["of"].as_str(), "redo.findings", "findings")];
        for (target, placeholder, what) in redo {
            let Some(target) = target.and_then(|id| definition.nodes.iter().find(|other| other.id == id)) else { continue };
            let prompt = target.step["prompt"].as_str().unwrap_or_default().replace(' ', "");
            if !prompt.contains(&format!("${{{{{placeholder}}}}}")) {
                lines.push(format!("warning: `{}` redoes `{}`, but its prompt has no ${{{{ {placeholder} }}}}, so it will not see the {what}", node.id, target.id));
            }
        }
    }
    lines.join("\n")
}

/// The caller's project folder: workflows it checks and starts are read from there.
fn project_of(store: &SessionStore, owned_id: &str) -> Result<String, String> {
    store.get_session(owned_id).map_err(|error| error.to_string())?
        .map(|row| row.cwd).filter(|cwd| !cwd.is_empty())
        .ok_or_else(|| "This session has no project folder".into())
}

fn workflow_error(error: crate::workflow::WorkflowError) -> String {
    match error {
        crate::workflow::WorkflowError::InvalidDefinition(message) => message,
        error => format!("{error:?}"),
    }
}

/// What happened in a run, step by step: no prompts and no definition.
fn run_facts(run: &crate::workflow::WorkflowRunRecord, store: &SessionStore) -> Value {
    let steps = run.nodes.iter().map(|node| {
        let output = node.structured_output.clone().unwrap_or(Value::Null);
        let mut step = json!({"id":node.node_id,"state":node.state,"attempt":node.attempt,
            "worker":store.directory_name(&node.owned_id).ok().flatten().map(|(handle, _)| handle)});
        if let Some(failure) = &node.failure { step["failure"] = json!(failure.message); }
        if let Some(commands) = output["commands"].as_array() {
            step["commands"] = commands.iter().map(|command| {
                let lines: Vec<_> = command["output"].as_str().unwrap_or_default().lines().collect();
                json!({"command":command["command"],"exit_code":command["exitCode"],
                    "last_lines":lines[lines.len().saturating_sub(5)..].join("\n")})
            }).collect();
        }
        if !output["verdict"].is_null() { step["verdict"] = output["verdict"].clone(); step["findings"] = output["findings"].clone(); }
        if let Some(url) = output["url"].as_str().filter(|url| !url.is_empty()) { step["pr"] = json!(url); }
        step
    }).collect::<Vec<_>>();
    json!({"run_id":run.id,"name":run.definition.name,"task_key":run.input_snapshot["task"]["key"],
        "state":run.state,"phase":run.phase,"started_by":run.input_snapshot["started_by"],"steps":steps})
}

/// The caller's session id, its backend's store, and the manager that delivers
/// messages and starts helper sessions on that backend.
type Caller = (String, Weak<SessionStore>, Option<AgentRuntimeManager>);

pub struct MessagingEndpoint {
    url: String,
    callers: Mutex<HashMap<String, Caller>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    relay: Mutex<Option<Relay>>,
    workflow: Mutex<Option<crate::workflow::WorkflowEngine>>,
}

/// One endpoint per owning backend, including the remote server.
pub fn endpoint() -> Result<&'static Arc<MessagingEndpoint>, String> {
    static ENDPOINT: OnceLock<Result<Arc<MessagingEndpoint>, String>> = OnceLock::new();
    ENDPOINT.get_or_init(MessagingEndpoint::start).as_ref().map_err(Clone::clone)
}

impl MessagingEndpoint {
    pub fn start() -> Result<Arc<Self>, String> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let url = format!("http://{}/mcp", listener.local_addr().map_err(|e| e.to_string())?);
        let endpoint = Arc::new(Self { url, callers: Mutex::new(HashMap::new()), task: Mutex::new(None), relay: Mutex::new(None),
            workflow: Mutex::new(None) });
        let router = Router::new().route("/mcp", post(Self::post)).with_state(Arc::downgrade(&endpoint));
        *endpoint.task.lock().unwrap() = Some(tauri::async_runtime::spawn(async move {
            if let Ok(listener) = tokio::net::TcpListener::from_std(listener) {
                let _ = axum::serve(listener, router).await;
            }
        }));
        Ok(endpoint)
    }

    pub fn issue(&self, owned_id: &str) -> SessionCredential {
        let mut callers = self.callers.lock().unwrap();
        let token = callers.iter().find(|(_, (id, _, _))| id.as_str() == owned_id).map(|(token, _)| token.clone())
            .unwrap_or_else(|| {
                let token = uuid::Uuid::new_v4().to_string();
                callers.insert(token.clone(), (owned_id.to_string(), Weak::new(), None));
                token
            });
        SessionCredential { url: self.url.clone(), token }
    }

    pub fn bind_store(&self, token: &str, store: &Arc<SessionStore>, manager: Option<AgentRuntimeManager>) {
        if let Some((_, bound, bound_manager)) = self.callers.lock().unwrap().get_mut(token) {
            *bound = Arc::downgrade(store);
            *bound_manager = manager;
        }
    }

    pub fn set_relay(&self, relay: Relay) {
        *self.relay.lock().unwrap() = Some(relay);
    }

    /// This machine's workflow engine, which agents start and watch their runs on.
    pub fn set_workflow(&self, engine: crate::workflow::WorkflowEngine) {
        *self.workflow.lock().unwrap() = Some(engine);
    }

    fn workflow(&self) -> Result<crate::workflow::WorkflowEngine, String> {
        self.workflow.lock().unwrap().clone().ok_or_else(|| "Workflows cannot be run from here".into())
    }

    pub fn revoke(&self, owned_id: &str) {
        self.callers.lock().unwrap().retain(|_, (id, _, _)| id != owned_id);
    }

    fn caller(&self, token: &str) -> Result<Caller, String> {
        self.callers.lock().unwrap().get(token).cloned().ok_or_else(|| "Unauthorized".into())
    }

    pub async fn handle(&self, token: &str, tool_call: Value) -> Result<Value, String> {
        let (owned_id, store, manager) = self.caller(token)?;
        let store = store.upgrade().ok_or("Session store unavailable")?;
        match tool_call["name"].as_str() {
            Some("list_sessions") => {
                let mut entries = serde_json::to_value(store.list_directory().map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
                // Other machines' sessions come from the desktop, which also says what this machine is called.
                let relay = self.relay.lock().unwrap().clone();
                let others = match relay {
                    Some(relay) => relay("directory".into(), json!({})).await.unwrap_or(Value::Null),
                    None => Value::Null,
                };
                let entries = entries.as_array_mut().ok_or("the session directory is not a list")?;
                for entry in entries.iter_mut() { entry["machine"] = others["machine"].clone(); }
                entries.extend(others["entries"].as_array().cloned().unwrap_or_default());
                Ok(text(serde_json::to_string(entries).map_err(|e| e.to_string())?))
            }
            Some("send_message") => {
                let args = &tool_call["arguments"];
                let to = args["to"].as_str().ok_or("to must be a session name")?;
                let text = args["text"].as_str().ok_or("text must be a string")?;
                let relay = self.relay.lock().unwrap().clone();
                // A name no session here has, such as handle@machine, is for another machine.
                let local = match store.resolve_session_name(to) {
                    Ok(target) => Some(target),
                    Err(error) if relay.is_some() && error.to_string().contains("Unknown session name") => None,
                    Err(error) => return Err(error.to_string()),
                };
                let id = match (local, relay) {
                    (Some(target), _) => {
                        let message = store.append_message("", &owned_id, &target, MessageKind::Message, text)
                            .map_err(|error| error.to_string())?;
                        if let Some(manager) = manager {
                            tauri::async_runtime::spawn(async move { manager.deliver_if_idle(&target).await });
                        }
                        message.id
                    }
                    (None, Some(relay)) => {
                        let sender = store.directory_name(&owned_id).map_err(|error| error.to_string())?
                            .map_or_else(|| owned_id.clone(), |(handle, name)| format!("{handle} · {name}"));
                        let id = relay("enqueue".into(), json!({"to":to,"text":text,"from":owned_id,"sender":sender})).await?;
                        id.as_str().unwrap_or_default().to_string()
                    }
                    (None, None) => return Err("Unknown session name".into()),
                };
                Ok(text_json(json!({"id":id,"status":"queued"})))
            }
            Some("message_status") => {
                let id = tool_call["arguments"]["id"].as_str().ok_or("id must be a message id")?;
                // A message sent to another machine has that machine's name before its id.
                let relay = self.relay.lock().unwrap().clone().filter(|_| id.contains(':'));
                let status = match relay {
                    Some(relay) => relay("status".into(), json!({"id":id,"from":owned_id})).await?,
                    None => json!(store.message_status(id, &owned_id).map_err(|error| error.to_string())?),
                };
                Ok(text_json(json!({"id":id,"status":status})))
            }
            Some("start_session") => {
                let args = &tool_call["arguments"];
                let provider = match args["provider"].as_str() {
                    Some("codex") => AgentConversationProvider::Codex,
                    Some("claude") => AgentConversationProvider::Claude,
                    _ => return Err("provider must be codex or claude".into()),
                };
                let name = args["name"].as_str().map(str::trim).filter(|name| !name.is_empty())
                    .ok_or("name must be a session name")?;
                let prompt = args["prompt"].as_str().ok_or("prompt must be a string")?;
                let model = args["model"].as_str().map(str::to_string);
                let manager = manager.ok_or("Sessions cannot be started from here")?;
                let helper = manager.start_helper(&owned_id, provider, model, name, prompt).await?;
                let (handle, name) = store.directory_name(&helper).map_err(|error| error.to_string())?
                    .unwrap_or_else(|| (String::new(), name.to_string()));
                Ok(json!({"content":[{"type":"text","text":json!({"handle":handle,"name":name,"owned_id":helper}).to_string()}]}))
            }
            Some("workflow_format") => Ok(text(WORKFLOW_FORMAT)),
            Some("report_review") => {
                let args = &tool_call["arguments"];
                let verdict = args["verdict"].as_str().map(str::to_ascii_lowercase)
                    .filter(|verdict| verdict == "pass" || verdict == "fail")
                    .ok_or("verdict must be pass or fail")?;
                let findings = match &args["findings"] {
                    Value::Null => json!([]),
                    findings @ Value::Array(_) => findings.clone(),
                    _ => return Err("findings must be a list".into()),
                };
                let (run_id, attempt_id) = store.workflow_running_review(&owned_id).map_err(|error| error.to_string())?
                    .ok_or("Only a session running a workflow review step can report a review")?;
                store.add_review_report(&owned_id, &run_id, &attempt_id, &json!({"verdict":verdict,"findings":findings}).to_string())
                    .map_err(|error| error.to_string())?;
                Ok(text("recorded"))
            }
            Some("check_workflow") => {
                let path = tool_call["arguments"]["path"].as_str()
                    .ok_or("path must be a workflow file relative to your project")?;
                let project = project_of(&store, &owned_id)?;
                crate::workflow::WorkflowEngine::validate_file(std::path::Path::new(&project), std::path::Path::new(path))
                    .map(|definition| text(workflow_summary(&definition))).map_err(workflow_error)
            }
            Some("start_workflow") => {
                let engine = self.workflow()?;
                let args = &tool_call["arguments"];
                let path = args["path"].as_str().ok_or("path must be a workflow file relative to your project")?;
                let task: crate::workflow::TaskSnapshot = serde_json::from_value(args["task"].clone())
                    .map_err(|_| "task must have a key, a title and a body")?;
                let project = project_of(&store, &owned_id)?;
                if store.unfinished_workflow_runs_started_by(&owned_id).map_err(|error| error.to_string())? >= 3 {
                    return Err("at most 3 unfinished runs".into());
                }
                let key = format!("agent:{owned_id}:{}", task.key);
                let run = engine.create_run_from_file(std::path::Path::new(&project), std::path::Path::new(path), task,
                    json!({"started_by":owned_id}), key.clone()).map_err(workflow_error)?;
                let run = engine.start(&run.id, &format!("{key}:start")).await.map_err(workflow_error)?;
                Ok(text_json(json!({"run_id":run.id,"state":run.state})))
            }
            Some("list_workflow_runs") => {
                let runs = self.workflow()?.list_runs().map_err(workflow_error)?;
                Ok(text_json(Value::Array(runs.iter().map(|run| json!({"run_id":run.id,"name":run.definition.name,
                    "task_key":run.input_snapshot["task"]["key"],"state":run.state,"phase":run.phase,
                    "current_step":run.nodes.iter().find(|node| !matches!(node.state, crate::workflow::WorkflowNodeState::Completed
                        | crate::workflow::WorkflowNodeState::Skipped | crate::workflow::WorkflowNodeState::Blocked)).map(|node| &node.node_id),
                    "started_by":run.input_snapshot["started_by"]})).collect())))
            }
            Some("get_workflow_run") => {
                let run_id = tool_call["arguments"]["run_id"].as_str().ok_or("run_id must be a run id")?;
                let run = self.workflow()?.get_run(run_id).map_err(workflow_error)?;
                Ok(text_json(run_facts(&run, &store)))
            }
            Some("stop_workflow_run") => {
                let engine = self.workflow()?;
                let run_id = tool_call["arguments"]["run_id"].as_str().ok_or("run_id must be a run id")?;
                let run = engine.get_run(run_id).map_err(workflow_error)?;
                if run.input_snapshot["started_by"].as_str() != Some(owned_id.as_str()) {
                    return Err("only the session that started this run, or the owner, can stop it".into());
                }
                let run = engine.cancel(run_id, &format!("agent:{owned_id}:stop")).await.map_err(workflow_error)?;
                Ok(text_json(json!({"run_id":run.id,"state":run.state})))
            }
            _ => Err("Tool not allowed".into()),
        }
    }

    async fn post(State(endpoint): State<Weak<Self>>, headers: HeaderMap, Json(request): Json<Value>) -> Response {
        let Some(endpoint) = endpoint.upgrade() else { return StatusCode::SERVICE_UNAVAILABLE.into_response(); };
        let token = headers.get("Authorization").and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer ")).unwrap_or("");
        if endpoint.caller(token).is_err() { return StatusCode::UNAUTHORIZED.into_response(); }
        let result = match request["method"].as_str() {
            Some("notifications/initialized") => return StatusCode::ACCEPTED.into_response(),
            Some("initialize") => json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},
                "serverInfo":{"name":"assembly","version":"1"}}),
            Some("tools/list") => json!({"tools":[
                {"name":"list_sessions","description":"List Assembly sessions on this machine and on the other machines connected to the desktop. Each has a handle, a name and the machine it runs on; send to handle@machine.",
                 "inputSchema":{"type":"object","properties":{}}},
                {"name":"send_message","description":"Send a message to an Assembly session, addressed by its handle (such as brisk-heron), its name, or its owned id; add @machine (brisk-heron@workbox-test) for a session on another machine. A handle never changes when the session is renamed.",
                 "inputSchema":{"type":"object","properties":{"to":{"type":"string"},"text":{"type":"string"}},"required":["to","text"]}},
                {"name":"start_session","description":"Start a Codex or Claude helper session in your project. It is listed with your agents, and its final reply is sent to you as a message when it finishes.",
                 "inputSchema":{"type":"object","properties":{"provider":{"type":"string","enum":["codex","claude"]},
                     "model":{"type":"string"},"prompt":{"type":"string"},"name":{"type":"string"}},"required":["provider","prompt","name"]}},
                {"name":"workflow_format","description":"Read the reference for writing an Assembly workflow (.assembly/workflows/*.yml).",
                 "inputSchema":{"type":"object","properties":{}}},
                {"name":"check_workflow","description":"Check a workflow file in your project, the same way starting it would. Returns ok, its steps and any warnings, or the first error.",
                 "inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
                {"name":"start_workflow","description":"Start a workflow from your project's .assembly/workflows on this machine for a task. You are messaged when it waits for the owner or ends. At most 3 of your runs can be unfinished.",
                 "inputSchema":{"type":"object","properties":{"path":{"type":"string"},
                     "task":{"type":"object","properties":{"key":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}},"required":["key","title","body"]}},
                     "required":["path","task"]}},
                {"name":"list_workflow_runs","description":"List the workflow runs on this machine.",
                 "inputSchema":{"type":"object","properties":{}}},
                {"name":"get_workflow_run","description":"Read what happened in a workflow run, step by step: state, attempt, worker, failure, command exit codes, review verdict and findings, pull request.",
                 "inputSchema":{"type":"object","properties":{"run_id":{"type":"string"}},"required":["run_id"]}},
                {"name":"stop_workflow_run","description":"Stop a workflow run you started.",
                 "inputSchema":{"type":"object","properties":{"run_id":{"type":"string"}},"required":["run_id"]}},
                {"name":"report_review","description":"Workflow review steps only: report your verdict once. Use pass with no findings, or fail with each finding.",
                 "inputSchema":{"type":"object","properties":{"verdict":{"type":"string","enum":["pass","fail"]},
                     "findings":{"type":"array","items":{"type":"string"}}},"required":["verdict","findings"]}},
                {"name":"message_status","description":"Read whether a message you sent is queued, delivered or failed.",
                 "inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}}
            ]}),
            Some("tools/call") => match endpoint.handle(token, request["params"].clone()).await {
                Ok(value) => value,
                Err(error) => json!({"isError":true,"content":[{"type":"text","text":error}]}),
            },
            _ => return Json(json!({"jsonrpc":"2.0","id":request["id"],
                "error":{"code":-32601,"message":"Method not found"}})).into_response(),
        };
        Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
    }
}
impl Drop for MessagingEndpoint {
    fn drop(&mut self) { if let Some(task) = self.task.lock().unwrap().take() { task.abort(); } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn store() -> Arc<SessionStore> {
        use mcb_core::session_store::SessionRow;
        let store = Arc::new(SessionStore::open_in_memory().unwrap());
        for (id, title) in [("owned-a","a"),("owned-b","b"),("caller","Caller"),
            ("o1","orchestrator"),("o2","orchestrator"),("d1","Duplicate"),("d2","Duplicate")] {
            store.upsert_session(&SessionRow {
                owned_id:id.into(), native_session_id:None, provider:"codex".into(), model:None,
                effort:None, cwd:"/work".into(), worktree:None, branch:None, title:Some(title.into()),
                title_source:None, project:None, project_id:None, state:"ready".into(), suspended:false,
                created_at_ms:1, last_activity_at_ms:1, extra_json:"{}".into(),
            }).unwrap();
        }
        store
    }
    async fn call(c: &SessionCredential, token: Option<&str>, method: &str, params: Value) -> reqwest::Response {
        let request = reqwest::Client::new().post(&c.url).json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}));
        let request = if let Some(token) = token { request.bearer_auth(token) } else { request };
        request.send().await.unwrap()
    }
    #[test]
    fn designate_moves_the_mark() {
        let store = store();
        let marked = |store: &SessionStore| store.list_directory().unwrap().into_iter()
            .filter(|entry| entry.orchestrator).map(|entry| entry.owned_id).collect::<Vec<_>>();
        answer(&store, None, "designate", &json!({"owned_id":"owned-a"})).unwrap();
        assert_eq!(marked(&store), ["owned-a"]);
        answer(&store, None, "designate", &json!({"owned_id":"owned-b"})).unwrap();
        assert_eq!(marked(&store), ["owned-b"]);
        let error = answer(&store, None, "designate", &json!({"owned_id":"missing"})).unwrap_err();
        assert!(error.contains("Unknown session"), "{error}");
        assert_eq!(marked(&store), ["owned-b"]);
        answer(&store, None, "designate", &json!({"owned_id":null})).unwrap();
        assert!(marked(&store).is_empty());
    }

    #[tokio::test]
    async fn directory_rejects_ambiguous_or_missing_name() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let caller = endpoint.issue("caller");
        endpoint.bind_store(&caller.token, &store, None);
        let error = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"missing","text":"hello"}})).await.unwrap_err();
        assert!(error.contains("Unknown session name"), "{error}");
        // A title never makes an orchestrator: duplicate titles get their ids, like any other.
        let error = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"orchestrator","text":"hello"}})).await.unwrap_err();
        assert!(error.contains("Unknown session name"), "{error}");
        let listing = endpoint.handle(&caller.token, json!({"name":"list_sessions"})).await.unwrap();
        let entries: Value = serde_json::from_str(listing["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(entries.as_array().unwrap().len(), 7);
        let entries = store.list_directory().unwrap();
        assert!(entries.iter().any(|entry| entry.name == "Duplicate [d1]"));
        assert_eq!(store.resolve_session_name("Duplicate [d2]").unwrap(), "d2");
        let result = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"b","text":"hello"}})).await.unwrap();
        let receipt: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        let message = store.pending_for("owned-b").unwrap().unwrap();
        assert_eq!(receipt["id"], message.id);
        assert_eq!(receipt["status"], "queued");
        assert!(message.sender_name.ends_with(" · Caller"), "{}", message.sender_name);
        assert_eq!(store.resolve_session_name(&message.sender_name).unwrap(), "caller");
    }

    #[tokio::test]
    async fn messaging_token_cannot_impersonate_another_session() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        let b = endpoint.issue("owned-b");
        endpoint.bind_store(&a.token, &store, None);
        endpoint.bind_store(&b.token, &store, None);
        for token in [None, Some("wrong")] {
            assert_eq!(call(&a, token, "tools/call", json!({"name":"send_message","arguments":{"token":b.token}})).await.status(), 401);
        }
        let result: Value = call(&a, Some(&a.token), "tools/call",
            json!({"name":"send_message","arguments":{"from":"owned-b","token":b.token,"to":"b","text":"hello"}}))
            .await.json().await.unwrap();
        assert_ne!(result["result"]["isError"], true);
        let message = store.pending_for("owned-b").unwrap().unwrap();
        assert_eq!(message.from_agent, "owned-a");
        assert!(message.sender_name.ends_with(" · a"), "{}", message.sender_name);
        endpoint.revoke("owned-a");
        assert_eq!(call(&a, Some(&a.token), "tools/list", json!({})).await.status(), 401);
        assert_eq!(call(&b, Some(&b.token), "tools/list", json!({})).await.status(), 200);
    }
    async fn receipt(endpoint: &MessagingEndpoint, token: &str, tool: Value) -> Result<Value, String> {
        let result = endpoint.handle(token, tool).await?;
        Ok(serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap())
    }

    #[tokio::test]
    async fn mcp_message_status_tracks_local_receipt() {
        use mcb_core::broker::Receipt;
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        let b = endpoint.issue("owned-b");
        endpoint.bind_store(&a.token, &store, None);
        endpoint.bind_store(&b.token, &store, None);
        let send = json!({"name":"send_message","arguments":{"to":"b","text":"hello"}});
        let status = |id: &Value| json!({"name":"message_status","arguments":{"id":id}});
        for (outcome, label) in [(Receipt::Delivered, "delivered"), (Receipt::Failed, "failed")] {
            let sent = receipt(&endpoint, &a.token, send.clone()).await.unwrap();
            assert_eq!(receipt(&endpoint, &a.token, status(&sent["id"])).await.unwrap(),
                json!({"id":sent["id"],"status":"queued"}));
            store.set_receipt(sent["id"].as_str().unwrap(), outcome).unwrap();
            assert_eq!(receipt(&endpoint, &a.token, status(&sent["id"])).await.unwrap(),
                json!({"id":sent["id"],"status":label}));
            let error = receipt(&endpoint, &b.token, status(&sent["id"])).await.unwrap_err();
            assert!(error.contains("Unknown message"), "{error}");
        }
    }

    #[tokio::test]
    async fn mcp_send_uses_token_caller_and_rejects_approval() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        endpoint.bind_store(&a.token, &store, None);
        receipt(&endpoint, &a.token, json!({"name":"send_message",
            "arguments":{"from":"o1","to":"b","text":"hello"}})).await.unwrap();
        assert_eq!(store.pending_for("owned-b").unwrap().unwrap().from_agent, "owned-a");
        for tool in ["approve_gate", "answer_decision", "answer_pause"] {
            let error = endpoint.handle(&a.token, json!({"name":tool,"arguments":{}})).await.unwrap_err();
            assert_eq!(error, "Tool not allowed");
        }
    }

    #[tokio::test]
    async fn start_session_rejects_unknown_provider_and_missing_credential() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        let start = |provider: &str| json!({"name":"start_session",
            "arguments":{"provider":provider,"name":"Helper","prompt":"Look"}});
        assert_eq!(endpoint.handle("missing", start("codex")).await.unwrap_err(), "Unauthorized");
        endpoint.bind_store(&a.token, &store, None);
        assert_eq!(endpoint.handle(&a.token, start("gemini")).await.unwrap_err(), "provider must be codex or claude");
        assert_eq!(endpoint.handle(&a.token, start("codex")).await.unwrap_err(), "Sessions cannot be started from here");
        let manager = AgentRuntimeManager::default();
        endpoint.bind_store(&a.token, &store, Some(manager));
        let error = endpoint.handle(&a.token, start("claude")).await.unwrap_err();
        assert_eq!(error, "The calling session was not found");
        assert!(store.list_directory().unwrap().iter().all(|entry| entry.name != "Helper"));
    }

    fn project(label: &str, steps: &str) -> std::path::PathBuf {
        let root = std::path::PathBuf::from("/var/tmp").join(format!("tsk1416-authoring-{label}-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".assembly/workflows/prompts")).unwrap();
        std::fs::write(root.join(".assembly/workflows/prompts/build.md"), "Build it").unwrap();
        std::fs::write(root.join(".assembly/workflows/flow.yml"), format!("name: Flow\nsteps:\n{steps}")).unwrap();
        root
    }

    fn session_in(store: &SessionStore, owned_id: &str, provider: &str, cwd: &std::path::Path) {
        use mcb_core::session_store::SessionRow;
        store.upsert_session(&SessionRow {
            owned_id: owned_id.into(), native_session_id: None, provider: provider.into(), model: None,
            effort: None, cwd: cwd.display().to_string(), worktree: None, branch: None, title: Some(owned_id.into()),
            title_source: None, project: None, project_id: None, state: "ready".into(), suspended: false,
            created_at_ms: 1, last_activity_at_ms: 1, extra_json: "{}".into(),
        }).unwrap();
    }

    async fn tool_text(endpoint: &MessagingEndpoint, token: &str, tool: Value) -> Result<String, String> {
        let result = endpoint.handle(token, tool).await?;
        Ok(result["content"][0]["text"].as_str().unwrap().to_string())
    }

    fn check(path: &str) -> Value { json!({"name":"check_workflow","arguments":{"path":path}}) }

    const BUILD: &str = "  build: { agent: { provider: codex }, prompt: prompts/build.md }\n";

    #[tokio::test]
    async fn workflow_authoring_format_is_embedded_for_every_session() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let (a, b) = (endpoint.issue("owned-a"), endpoint.issue("owned-b"));
        session_in(&store, "owned-a", "codex", std::path::Path::new("/work/a"));
        session_in(&store, "owned-b", "claude", std::path::Path::new("/work/b"));
        for caller in [&a, &b] {
            endpoint.bind_store(&caller.token, &store, None);
            let list: Value = call(caller, Some(&caller.token), "tools/list", json!({})).await.json().await.unwrap();
            let names: Vec<_> = list["result"]["tools"].as_array().unwrap().iter().map(|tool| tool["name"].clone()).collect();
            assert!(names.contains(&json!("workflow_format")) && names.contains(&json!("check_workflow")));
            let format = tool_text(&endpoint, &caller.token, json!({"name":"workflow_format"})).await.unwrap();
            assert_eq!(format, include_str!("../../../../docs/workflows/format.md"));
        }
    }

    #[tokio::test]
    async fn workflow_authoring_check_is_read_only() {
        let root = project("summary", &format!("{BUILD}  checks: {{ needs: [build], run: [\"cargo test\"], on_fail: build }}\n  review:\n    needs: [checks]\n    review: {{ of: build, agent: {{ provider: claude }}, prompt: prompts/build.md }}\n"));
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        session_in(&store, "owned-a", "codex", &root);
        endpoint.bind_store(&a.token, &store, None);
        let summary = tool_text(&endpoint, &a.token, check(".assembly/workflows/flow.yml")).await.unwrap();
        assert!(summary.starts_with("ok\nbuild agent\nchecks run needs=build on_fail=build\nreview review needs=checks of=build\n"), "{summary}");
        // A failed check reaches the agent as an MCP tool error.
        std::fs::write(root.join(".assembly/workflows/flow.yml"), "name: Flow\nsteps:\n  build: { run: [] }\n").unwrap();
        let result: Value = call(&a, Some(&a.token), "tools/call", check(".assembly/workflows/flow.yml")).await.json().await.unwrap();
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(result["result"]["content"][0]["text"], "Step `build`: run must be a non-empty list of commands");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn check_workflow_warns_when_a_redone_step_cannot_see_why() {
        let root = project("redo", &format!("{BUILD}  checks: {{ needs: [build], run: [ls], on_fail: build }}\n  review:\n    needs: [checks]\n    review: {{ of: build, agent: {{ provider: claude }}, prompt: prompts/build.md }}\n"));
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        session_in(&store, "owned-a", "codex", &root);
        endpoint.bind_store(&a.token, &store, None);
        let path = ".assembly/workflows/flow.yml";
        let summary = tool_text(&endpoint, &a.token, check(path)).await.unwrap();
        assert!(summary.ends_with("\nwarning: `checks` redoes `build`, but its prompt has no ${{ redo.output }}, so it will not see the failure\nwarning: `review` redoes `build`, but its prompt has no ${{ redo.findings }}, so it will not see the findings"), "{summary}");
        std::fs::write(root.join(".assembly/workflows/prompts/build.md"), "Build it. Fix ${{ redo.findings }} and ${{redo.output}}").unwrap();
        assert!(!tool_text(&endpoint, &a.token, check(path)).await.unwrap().contains("warning"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn workflow_authoring_check_uses_callers_project() {
        let first = project("first", BUILD);
        let second = project("second", "  other: { run: [ls] }\n");
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let (a, b, c) = (endpoint.issue("owned-a"), endpoint.issue("owned-b"), endpoint.issue("caller"));
        session_in(&store, "owned-a", "codex", &first);
        session_in(&store, "owned-b", "claude", &second);
        session_in(&store, "caller", "codex", std::path::Path::new(""));
        for caller in [&a, &b, &c] { endpoint.bind_store(&caller.token, &store, None); }
        let path = ".assembly/workflows/flow.yml";
        assert_eq!(tool_text(&endpoint, &a.token, check(path)).await.unwrap(), "ok\nbuild agent");
        assert_eq!(tool_text(&endpoint, &b.token, check(path)).await.unwrap(), "ok\nother run");
        // Another project or session named in the arguments is ignored.
        let other = json!({"name":"check_workflow","arguments":{"path":path,"root":second,"owned_id":"owned-b"}});
        assert_eq!(tool_text(&endpoint, &a.token, other).await.unwrap(), "ok\nbuild agent");
        assert_eq!(tool_text(&endpoint, &c.token, check(path)).await.unwrap_err(), "This session has no project folder");
        let absolute = second.join(path).display().to_string();
        for bad in [absolute.as_str(), "../flow.yml", ".assembly/workflows/../workflows/flow.yml", ".assembly/flow.yml", ".assembly/workflows/flow.yaml"] {
            assert!(tool_text(&endpoint, &a.token, check(bad)).await.is_err(), "{bad}");
        }
        std::os::unix::fs::symlink(second.join(".assembly/workflows/prompts/build.md"), first.join(".assembly/workflows/prompts/linked.md")).unwrap();
        std::fs::write(first.join(".assembly/workflows/flow.yml"), "name: Flow\nsteps:\n  build: { agent: { provider: codex }, prompt: prompts/linked.md }\n").unwrap();
        assert_eq!(tool_text(&endpoint, &a.token, check(path)).await.unwrap_err(), "File path escapes its directory");
        std::os::unix::fs::symlink(second.join(path), first.join(".assembly/workflows/linked.yml")).unwrap();
        assert_eq!(tool_text(&endpoint, &a.token, check(".assembly/workflows/linked.yml")).await.unwrap_err(), "File path escapes its directory");
        std::fs::remove_dir_all(first).unwrap();
        std::fs::remove_dir_all(second).unwrap();
    }

    #[tokio::test]
    async fn workflow_authoring_check_preserves_loader_errors() {
        let root = project("errors", BUILD);
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        session_in(&store, "owned-a", "codex", &root);
        endpoint.bind_store(&a.token, &store, None);
        let path = std::path::Path::new(".assembly/workflows/flow.yml");
        for steps in [
            format!("{BUILD}{BUILD}"),
            "  bad.id: { run: [ls] }\n".to_string(),
            format!("{BUILD}  checks: {{ needs: [missing], run: [ls] }}\n"),
            "  a: { needs: [b], run: [ls] }\n  b: { needs: [a], run: [ls] }\n".to_string(),
            format!("{BUILD}  review:\n    needs: [build]\n    review: {{ of: missing, agent: {{ provider: codex }}, prompt: prompts/build.md }}\n"),
            "  build: { agent: { provider: codex }, prompt: prompts/missing.md }\n".to_string(),
            "  build: { agent: { provider: codex }, prompt: ../../outside.md }\n".to_string(),
        ] {
            std::fs::write(root.join(path), format!("name: Flow\nsteps:\n{steps}")).unwrap();
            let Err(crate::workflow::WorkflowError::InvalidDefinition(expected)) = crate::workflow::WorkflowEngine::validate_file(&root, path) else {
                panic!("{steps} should fail");
            };
            assert_eq!(tool_text(&endpoint, &a.token, check(".assembly/workflows/flow.yml")).await.unwrap_err(), expected);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn messaging_endpoint_answers_mcp_initialize_and_tools_list() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let c = endpoint.issue("owned");
        let init: Value = call(&c, Some(&c.token), "initialize", json!({"protocolVersion":"2024-11-05"})).await.json().await.unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(init["result"]["capabilities"], json!({"tools":{}}));
        let list: Value = call(&c, Some(&c.token), "tools/list", json!({})).await.json().await.unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 11);
        assert_eq!(list["result"]["tools"][0]["name"], "list_sessions");
        assert_eq!(list["result"]["tools"][1]["name"], "send_message");
        assert_eq!(list["result"]["tools"][2]["name"], "start_session");
        assert_eq!(list["result"]["tools"][10]["name"], "message_status");
        assert_eq!(call(&c, Some(&c.token), "notifications/initialized", json!({})).await.status(), 202);
    }
}
