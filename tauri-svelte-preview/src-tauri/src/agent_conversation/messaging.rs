use agent_client_protocol::schema::v1 as acp;
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::post, Json, Router};
use serde_json::{json, Value};
use mcb_core::{broker::MessageKind, session_store::SessionStore};
use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock, Weak}};

pub struct SessionCredential { pub url: String, pub token: String }
impl SessionCredential {
    pub fn server(&self) -> acp::McpServer {
        acp::McpServer::Http(acp::McpServerHttp::new("assembly", &self.url)
            .headers(vec![acp::HttpHeader::new("Authorization", format!("Bearer {}", self.token))]))
    }
}

/// Starts delivery to a session that was just sent a message.
pub type Wake = Arc<dyn Fn(String) + Send + Sync>;
type Caller = (String, Weak<SessionStore>, Option<Wake>);

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

    pub fn bind_store(&self, token: &str, store: &Arc<SessionStore>, wake: Option<Wake>) {
        if let Some((_, bound, bound_wake)) = self.callers.lock().unwrap().get_mut(token) {
            *bound = Arc::downgrade(store);
            *bound_wake = wake;
        }
    }

    pub fn revoke(&self, owned_id: &str) {
        self.callers.lock().unwrap().retain(|_, (id, _, _)| id != owned_id);
    }

    fn caller(&self, token: &str) -> Result<Caller, String> {
        self.callers.lock().unwrap().get(token).cloned().ok_or_else(|| "Unauthorized".into())
    }

    pub fn handle(&self, token: &str, tool_call: Value) -> Result<Value, String> {
        let (owned_id, store, wake) = self.caller(token)?;
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
                if let Some(wake) = wake { wake(target); }
                Ok(json!({"content":[{"type":"text","text":json!({"id":message.id,"status":"queued"}).to_string()}]}))
            }
            Some("message_status") => {
                let id = tool_call["arguments"]["id"].as_str().ok_or("id must be a message id")?;
                let status = store.message_status(id, &owned_id).map_err(|error| error.to_string())?;
                Ok(json!({"content":[{"type":"text","text":json!({"id":id,"status":status}).to_string()}]}))
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
                {"name":"message_status","description":"Read whether a message you sent is queued, delivered or failed.",
                 "inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}}
            ]}),
            Some("tools/call") => match endpoint.handle(token, request["params"].clone()) {
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
            "arguments":{"to":"missing","text":"hello"}})).unwrap_err();
        assert!(error.contains("Unknown session name"), "{error}");
        let error = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"orchestrator","text":"hello"}})).unwrap_err();
        assert!(error.contains("Ambiguous session name"), "{error}");
        let listing = endpoint.handle(&caller.token, json!({"name":"list_sessions"})).unwrap();
        let entries: Value = serde_json::from_str(listing["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(entries.as_array().unwrap().len(), 7);
        let entries = store.list_directory().unwrap();
        assert!(entries.iter().any(|entry| entry.name == "Duplicate [d1]"));
        assert_eq!(store.resolve_session_name("Duplicate [d2]").unwrap(), "d2");
        let result = endpoint.handle(&caller.token, json!({"name":"send_message",
            "arguments":{"to":"b","text":"hello"}})).unwrap();
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
    fn receipt(endpoint: &MessagingEndpoint, token: &str, tool: Value) -> Result<Value, String> {
        let result = endpoint.handle(token, tool)?;
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
            let sent = receipt(&endpoint, &a.token, send.clone()).unwrap();
            assert_eq!(receipt(&endpoint, &a.token, status(&sent["id"])).unwrap(),
                json!({"id":sent["id"],"status":"queued"}));
            store.set_receipt(sent["id"].as_str().unwrap(), outcome).unwrap();
            assert_eq!(receipt(&endpoint, &a.token, status(&sent["id"])).unwrap(),
                json!({"id":sent["id"],"status":label}));
            let error = receipt(&endpoint, &b.token, status(&sent["id"])).unwrap_err();
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
            "arguments":{"from":"o1","to":"b","text":"hello"}})).unwrap();
        assert_eq!(store.pending_for("owned-b").unwrap().unwrap().from_agent, "owned-a");
        for tool in ["approve_gate", "answer_decision", "answer_pause"] {
            let error = endpoint.handle(&a.token, json!({"name":tool,"arguments":{}})).unwrap_err();
            assert_eq!(error, "Tool not allowed");
        }
    }

    #[tokio::test]
    async fn send_message_wakes_the_target_once() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let store = store();
        let a = endpoint.issue("owned-a");
        let woken = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&woken);
        endpoint.bind_store(&a.token, &store, Some(Arc::new(move |target| sink.lock().unwrap().push(target))));
        receipt(&endpoint, &a.token, json!({"name":"send_message","arguments":{"to":"b","text":"hello"}})).unwrap();
        assert_eq!(*woken.lock().unwrap(), vec!["owned-b".to_string()]);
    }

    #[tokio::test]
    async fn messaging_endpoint_answers_mcp_initialize_and_tools_list() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let c = endpoint.issue("owned");
        let init: Value = call(&c, Some(&c.token), "initialize", json!({"protocolVersion":"2024-11-05"})).await.json().await.unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(init["result"]["capabilities"], json!({"tools":{}}));
        let list: Value = call(&c, Some(&c.token), "tools/list", json!({})).await.json().await.unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 3);
        assert_eq!(list["result"]["tools"][0]["name"], "list_sessions");
        assert_eq!(list["result"]["tools"][1]["name"], "send_message");
        assert_eq!(list["result"]["tools"][2]["name"], "message_status");
        assert_eq!(call(&c, Some(&c.token), "notifications/initialized", json!({})).await.status(), 202);
    }
}
