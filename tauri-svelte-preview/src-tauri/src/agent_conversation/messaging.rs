use agent_client_protocol::schema::v1 as acp;
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::post, Json, Router};
use serde_json::{json, Value};
use mcb_core::{broker::MessageKind, session_store::SessionStore};
use super::{manager::AgentRuntimeManager, protocol::AgentConversationProvider};
use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock, Weak}};

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
    lines.join("\n")
}

/// The caller's session id, its backend's store, and the manager that delivers
/// messages and starts helper sessions on that backend.
type Caller = (String, Weak<SessionStore>, Option<AgentRuntimeManager>);

pub struct MessagingEndpoint {
    url: String,
    callers: Mutex<HashMap<String, Caller>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
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
        let endpoint = Arc::new(Self { url, callers: Mutex::new(HashMap::new()), task: Mutex::new(None) });
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
                let entries = store.list_directory().map_err(|error| error.to_string())?;
                Ok(json!({"content":[{"type":"text","text":serde_json::to_string(&entries).map_err(|e| e.to_string())?}]}))
            }
            Some("send_message") => {
                let args = &tool_call["arguments"];
                let to = args["to"].as_str().ok_or("to must be a session name")?;
                let text = args["text"].as_str().ok_or("text must be a string")?;
                let target = store.resolve_session_name(to).map_err(|error| error.to_string())?;
                let message = store.append_message("", &owned_id, &target, MessageKind::Message, text)
                    .map_err(|error| error.to_string())?;
                if let Some(manager) = manager {
                    tauri::async_runtime::spawn(async move { manager.deliver_if_idle(&target).await });
                }
                Ok(json!({"content":[{"type":"text","text":json!({"id":message.id,"status":"queued"}).to_string()}]}))
            }
            Some("message_status") => {
                let id = tool_call["arguments"]["id"].as_str().ok_or("id must be a message id")?;
                let status = store.message_status(id, &owned_id).map_err(|error| error.to_string())?;
                Ok(json!({"content":[{"type":"text","text":json!({"id":id,"status":status}).to_string()}]}))
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
                let name = store.directory_name(&helper).map_err(|error| error.to_string())?
                    .unwrap_or_else(|| name.to_string());
                Ok(json!({"content":[{"type":"text","text":json!({"name":name,"owned_id":helper}).to_string()}]}))
            }
            Some("workflow_format") => Ok(text(WORKFLOW_FORMAT)),
            Some("check_workflow") => {
                let path = tool_call["arguments"]["path"].as_str()
                    .ok_or("path must be a workflow file relative to your project")?;
                let project = store.get_session(&owned_id).map_err(|error| error.to_string())?
                    .map(|row| row.cwd).filter(|cwd| !cwd.is_empty())
                    .ok_or("This session has no project folder")?;
                match crate::workflow::WorkflowEngine::validate_file(std::path::Path::new(&project), std::path::Path::new(path)) {
                    Ok(definition) => Ok(text(workflow_summary(&definition))),
                    Err(crate::workflow::WorkflowError::InvalidDefinition(message)) => Err(message),
                    Err(error) => Err(error.to_string()),
                }
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
                {"name":"list_sessions","description":"List Assembly sessions on this backend.",
                 "inputSchema":{"type":"object","properties":{}}},
                {"name":"send_message","description":"Send a message to a named Assembly session.",
                 "inputSchema":{"type":"object","properties":{"to":{"type":"string"},"text":{"type":"string"}},"required":["to","text"]}},
                {"name":"start_session","description":"Start a Codex or Claude helper session in your project. It is listed with your agents, and its final reply is sent to you as a message when it finishes.",
                 "inputSchema":{"type":"object","properties":{"provider":{"type":"string","enum":["codex","claude"]},
                     "model":{"type":"string"},"prompt":{"type":"string"},"name":{"type":"string"}},"required":["provider","prompt","name"]}},
                {"name":"workflow_format","description":"Read the reference for writing an Assembly workflow (.assembly/workflows/*.yml).",
                 "inputSchema":{"type":"object","properties":{}}},
                {"name":"check_workflow","description":"Check a workflow file in your project, the same way starting it would. Returns ok and its steps, or the first error.",
                 "inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
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
    #[tokio::test]
    async fn directory_rejects_ambiguous_or_missing_name() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let caller = endpoint.issue("caller");
        endpoint.bind_store(&caller.token, &store, None);
        let error = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"missing","text":"hello"}})).await.unwrap_err();
        assert!(error.contains("Unknown session name"), "{error}");
        let error = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"orchestrator","text":"hello"}})).await.unwrap_err();
        assert!(error.contains("Ambiguous session name"), "{error}");
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
        assert_eq!(message.sender_name, "Caller");
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
        assert_eq!(message.sender_name, "a");
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
        assert_eq!(summary, "ok\nbuild agent\nchecks run needs=build on_fail=build\nreview review needs=checks of=build");
        assert!(store.pending_for("owned-a").unwrap().is_none());
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
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 6);
        assert_eq!(list["result"]["tools"][0]["name"], "list_sessions");
        assert_eq!(list["result"]["tools"][1]["name"], "send_message");
        assert_eq!(list["result"]["tools"][2]["name"], "start_session");
        assert_eq!(list["result"]["tools"][5]["name"], "message_status");
        assert_eq!(call(&c, Some(&c.token), "notifications/initialized", json!({})).await.status(), 202);
    }
}
