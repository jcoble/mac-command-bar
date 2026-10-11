use agent_client_protocol::schema::v1 as acp;
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::post, Json, Router};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock, Weak}};

pub struct SessionCredential { pub url: String, pub token: String }
impl SessionCredential {
    pub fn server(&self) -> acp::McpServer {
        acp::McpServer::Http(acp::McpServerHttp::new("assembly", &self.url)
            .headers(vec![acp::HttpHeader::new("Authorization", format!("Bearer {}", self.token))]))
    }
}

pub struct MessagingEndpoint {
    url: String,
    callers: Mutex<HashMap<String, String>>,
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
        let token = callers.iter().find(|(_, id)| id.as_str() == owned_id).map(|(token, _)| token.clone())
            .unwrap_or_else(|| {
                let token = uuid::Uuid::new_v4().to_string();
                callers.insert(token.clone(), owned_id.to_string());
                token
            });
        SessionCredential { url: self.url.clone(), token }
    }

    pub fn revoke(&self, owned_id: &str) {
        self.callers.lock().unwrap().retain(|_, id| id != owned_id);
    }

    fn caller(&self, token: &str) -> Result<String, String> {
        self.callers.lock().unwrap().get(token).cloned().ok_or_else(|| "Unauthorized".into())
    }

    pub fn handle(&self, token: &str, tool_call: Value) -> Result<Value, String> {
        let owned_id = self.caller(token)?;
        match tool_call["name"].as_str() {
            Some(name @ ("list_sessions" | "send_message")) =>
                Err(format!("{name} not implemented yet for session {owned_id}")),
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
                 "inputSchema":{"type":"object","properties":{"to":{"type":"string"},"text":{"type":"string"}},"required":["to","text"]}}
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
    async fn call(c: &SessionCredential, token: Option<&str>, method: &str, params: Value) -> reqwest::Response {
        let request = reqwest::Client::new().post(&c.url).json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}));
        let request = if let Some(token) = token { request.bearer_auth(token) } else { request };
        request.send().await.unwrap()
    }
    #[tokio::test]
    async fn messaging_token_cannot_impersonate_another_session() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let a = endpoint.issue("owned-a");
        let b = endpoint.issue("owned-b");
        for token in [None, Some("wrong")] {
            assert_eq!(call(&a, token, "tools/call", json!({"name":"send_message","arguments":{"token":b.token}})).await.status(), 401);
        }
        let result: Value = call(&a, Some(&a.token), "tools/call",
            json!({"name":"send_message","arguments":{"from":"owned-b","token":b.token,"to":"b","text":"hello"}}))
            .await.json().await.unwrap();
        assert_eq!(result["result"]["content"][0]["text"], "send_message not implemented yet for session owned-a");
        endpoint.revoke("owned-a");
        assert_eq!(call(&a, Some(&a.token), "tools/list", json!({})).await.status(), 401);
        assert_eq!(call(&b, Some(&b.token), "tools/list", json!({})).await.status(), 200);
    }
    #[tokio::test]
    async fn messaging_endpoint_answers_mcp_initialize_and_tools_list() {
        let endpoint = MessagingEndpoint::start().unwrap();
        let c = endpoint.issue("owned");
        let init: Value = call(&c, Some(&c.token), "initialize", json!({"protocolVersion":"2024-11-05"})).await.json().await.unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(init["result"]["capabilities"], json!({"tools":{}}));
        let list: Value = call(&c, Some(&c.token), "tools/list", json!({})).await.json().await.unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 2);
        assert_eq!(list["result"]["tools"][0]["name"], "list_sessions");
        assert_eq!(list["result"]["tools"][1]["name"], "send_message");
        assert_eq!(call(&c, Some(&c.token), "notifications/initialized", json!({})).await.status(), 202);
    }
}
