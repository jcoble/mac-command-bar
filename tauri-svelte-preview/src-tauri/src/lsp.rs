use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const LSP_REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const LSP_DIAGNOSTICS_TIMEOUT: Duration = Duration::from_millis(1200);
const MAX_LSP_HEADER_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspPreview {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    content: String,
    line_count: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspLookupRequest {
    root: String,
    line: usize,
    column: usize,
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspStatus {
    language: String,
    #[serde(rename = "languageID")]
    language_id: String,
    available: bool,
    server_name: String,
    command: String,
    args: Vec<String>,
    reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspHover {
    contents: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspDiagnostic {
    severity: String,
    message: String,
    line: usize,
    column: usize,
    source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspSymbol {
    name: String,
    kind: String,
    line: usize,
    column: usize,
    detail: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspDefinitionTarget {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    symbol_name: String,
    kind: String,
    line: usize,
    column: usize,
    detail: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspReferenceTarget {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    symbol_name: String,
    line: usize,
    column: usize,
    excerpt: String,
}

#[derive(Debug, Clone, Copy)]
struct LspServerSpec {
    server_name: &'static str,
    language_id: &'static str,
    command: &'static str,
    args: &'static [&'static str],
}

#[derive(Debug)]
struct ResolvedLspServer {
    spec: LspServerSpec,
    command: String,
}

#[derive(Debug)]
struct LspLocation {
    path: PathBuf,
    line: usize,
    column: usize,
}

enum LspReaderMessage {
    Message(Value),
    Error(String),
}

#[derive(Clone, Default)]
pub(crate) struct SourceLspRegistry {
    sessions: Arc<Mutex<HashMap<SourceLspSessionKey, Arc<Mutex<SourceLspSession>>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceLspSessionKey {
    root: String,
    language: String,
}

struct SourceLspSession {
    server: ResolvedLspServer,
    stdin: ChildStdin,
    child: Child,
    receiver: mpsc::Receiver<LspReaderMessage>,
    next_id: i64,
    open_documents: HashMap<String, i32>,
    diagnostics_by_uri: HashMap<String, Vec<SourceLspDiagnostic>>,
}

impl SourceLspRegistry {
    pub(crate) fn find_definitions(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspDefinitionTarget>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/definition")? else {
            return Ok(Vec::new());
        };

        let symbol_name = symbol_at_position(&preview.content, request.line, request.column)
            .unwrap_or_else(|| preview.file_name.clone());
        let mut targets = Vec::new();
        for location in lsp_locations_from_result(&result) {
            targets.push(definition_target_from_location(
                &preview,
                &request,
                &location,
                &symbol_name,
            ));
            if targets.len() >= request.limit.unwrap_or(20) {
                break;
            }
        }
        Ok(targets)
    }

    pub(crate) fn find_references(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspReferenceTarget>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/references")? else {
            return Ok(Vec::new());
        };

        let symbol_name = symbol_at_position(&preview.content, request.line, request.column)
            .unwrap_or_else(|| preview.file_name.clone());
        let mut targets = Vec::new();
        for location in lsp_locations_from_result(&result) {
            targets.push(reference_target_from_location(
                &preview,
                &request,
                &location,
                &symbol_name,
            ));
            if targets.len() >= request.limit.unwrap_or(50) {
                break;
            }
        }
        Ok(targets)
    }

    pub(crate) fn find_hover(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Option<SourceLspHover>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/hover")? else {
            return Ok(None);
        };

        let contents = hover_contents_from_result(&result);
        if contents.is_empty() {
            return Ok(None);
        }
        Ok(Some(SourceLspHover { contents }))
    }

    pub(crate) fn find_symbols(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspSymbol>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/documentSymbol")? else {
            return Ok(Vec::new());
        };

        Ok(lsp_symbols_from_result(
            &result,
            request.limit.unwrap_or(100),
        ))
    }

    pub(crate) fn read_diagnostics(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspDiagnostic>, String> {
        for attempt in 0..2 {
            let Some(session) = self.session_for(&preview, &request)? else {
                return Ok(Vec::new());
            };
            let (result, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.read_diagnostics(&preview);
                let session_alive = session.is_alive();
                (result, session_alive)
            };

            match result {
                Ok(diagnostics) => return Ok(diagnostics),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(&preview, &request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(Vec::new())
    }

    fn request(
        &self,
        preview: &SourceLspPreview,
        request: &SourceLspLookupRequest,
        method: &str,
    ) -> Result<Option<Value>, String> {
        for attempt in 0..2 {
            let Some(session) = self.session_for(preview, request)? else {
                return Ok(None);
            };
            let (result, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.request(preview, request, method);
                let session_alive = session.is_alive();
                (result, session_alive)
            };

            match result {
                Ok(value) => return Ok(value),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(preview, request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(None)
    }

    fn session_for(
        &self,
        preview: &SourceLspPreview,
        request: &SourceLspLookupRequest,
    ) -> Result<Option<Arc<Mutex<SourceLspSession>>>, String> {
        let Some(key) = SourceLspSessionKey::from_preview(preview, request) else {
            return Ok(None);
        };

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Language server registry lock poisoned".to_string())?;
        if let Some(session) = sessions.get(&key).cloned() {
            let alive = {
                let mut session = lock_lsp_session(&session)?;
                session.is_alive()
            };
            if alive {
                return Ok(Some(session));
            }
            sessions.remove(&key);
        }

        let Some(server) = resolve_server_for_language(&preview.language) else {
            return Ok(None);
        };
        let session = SourceLspSession::start(PathBuf::from(&request.root), server)?;
        let session = Arc::new(Mutex::new(session));
        sessions.insert(key, Arc::clone(&session));
        Ok(Some(session))
    }

    fn remove_session(
        &self,
        preview: &SourceLspPreview,
        request: &SourceLspLookupRequest,
    ) -> Result<(), String> {
        let Some(key) = SourceLspSessionKey::from_preview(preview, request) else {
            return Ok(());
        };
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Language server registry lock poisoned".to_string())?;
        sessions.remove(&key);
        Ok(())
    }
}

impl SourceLspSessionKey {
    fn from_preview(
        preview: &SourceLspPreview,
        request: &SourceLspLookupRequest,
    ) -> Option<SourceLspSessionKey> {
        let root = normalized_lsp_root(&request.root)?;
        let spec = server_spec_for_language(&preview.language)?;
        Some(SourceLspSessionKey {
            root,
            language: spec.language_id.to_string(),
        })
    }
}

impl SourceLspSession {
    fn start(root: PathBuf, server: ResolvedLspServer) -> Result<SourceLspSession, String> {
        if !root.is_dir() {
            return Err("Project root is not a directory".to_string());
        }

        let mut child = Command::new(&server.command)
            .args(server.spec.args)
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not start {}: {error}", server.spec.server_name))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Language server stdin unavailable".to_string())?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Language server stdout unavailable".to_string())?;

        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || loop {
            match read_lsp_message(&mut stdout) {
                Ok(message) => {
                    if sender.send(LspReaderMessage::Message(message)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(LspReaderMessage::Error(error.to_string()));
                    break;
                }
            }
        });

        let root_uri = path_to_file_uri(&root);
        write_lsp_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
                    "capabilities": {}
                }
            }),
        )?;
        let response = wait_for_lsp_response(&receiver, 1, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server initialize failed: {error}"));
        }
        write_lsp_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        )?;

        Ok(SourceLspSession {
            server,
            stdin,
            child,
            receiver,
            next_id: 2,
            open_documents: HashMap::new(),
            diagnostics_by_uri: HashMap::new(),
        })
    }

    fn request(
        &mut self,
        preview: &SourceLspPreview,
        request: &SourceLspLookupRequest,
        method: &str,
    ) -> Result<Option<Value>, String> {
        let file_uri = self.ensure_document_open(preview)?;
        let id = self.next_request_id();
        let params = match method {
            "textDocument/documentSymbol" => json!({
                "textDocument": { "uri": file_uri }
            }),
            "textDocument/references" => json!({
                "textDocument": { "uri": file_uri },
                "position": lsp_position(request),
                "context": { "includeDeclaration": true }
            }),
            _ => json!({
                "textDocument": { "uri": file_uri },
                "position": lsp_position(request)
            }),
        };

        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        )?;
        let response = self.wait_for_response(id, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server request failed: {error}"));
        }

        Ok(response.get("result").cloned())
    }

    fn read_diagnostics(
        &mut self,
        preview: &SourceLspPreview,
    ) -> Result<Vec<SourceLspDiagnostic>, String> {
        let file_uri = self.ensure_document_open(preview)?;
        self.drain_messages_until(Instant::now() + LSP_DIAGNOSTICS_TIMEOUT);
        Ok(self
            .diagnostics_by_uri
            .get(&file_uri)
            .cloned()
            .unwrap_or_default())
    }

    fn ensure_document_open(&mut self, preview: &SourceLspPreview) -> Result<String, String> {
        let file_uri = path_to_file_uri(Path::new(&preview.path));
        let next_version = self.open_documents.get(&file_uri).copied().unwrap_or(0) + 1;

        if next_version == 1 {
            write_lsp_message(
                &mut self.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": file_uri,
                            "languageId": self.server.spec.language_id,
                            "version": next_version,
                            "text": preview.content
                        }
                    }
                }),
            )?;
        } else {
            write_lsp_message(
                &mut self.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": {
                            "uri": file_uri,
                            "version": next_version
                        },
                        "contentChanges": [
                            { "text": preview.content }
                        ]
                    }
                }),
            )?;
        }

        self.open_documents.insert(file_uri.clone(), next_version);
        self.drain_ready_messages();
        Ok(file_uri)
    }

    fn next_request_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn wait_for_response(&mut self, id: i64, deadline: Instant) -> Result<Value, String> {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err("Language server timed out".to_string());
            }

            match self
                .receiver
                .recv_timeout(deadline.saturating_duration_since(now))
            {
                Ok(LspReaderMessage::Message(message)) => {
                    self.record_diagnostics(&message);
                    if message.get("id").and_then(Value::as_i64) == Some(id) {
                        return Ok(message);
                    }
                }
                Ok(LspReaderMessage::Error(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err("Language server timed out".to_string());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("Language server exited before responding".to_string());
                }
            }
        }
    }

    fn drain_ready_messages(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(LspReaderMessage::Message(message)) => self.record_diagnostics(&message),
                Ok(LspReaderMessage::Error(_)) | Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
    }

    fn drain_messages_until(&mut self, deadline: Instant) {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return;
            }

            match self
                .receiver
                .recv_timeout(deadline.saturating_duration_since(now))
            {
                Ok(LspReaderMessage::Message(message)) => self.record_diagnostics(&message),
                Ok(LspReaderMessage::Error(_))
                | Err(mpsc::RecvTimeoutError::Timeout)
                | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn record_diagnostics(&mut self, message: &Value) {
        let Some(uri) = diagnostic_uri_from_message(message) else {
            return;
        };
        self.diagnostics_by_uri
            .insert(uri.clone(), diagnostics_from_message(message, &uri));
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for SourceLspSession {
    fn drop(&mut self) {
        let shutdown_id = self.next_request_id();
        let _ = write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": shutdown_id,
                "method": "shutdown"
            }),
        );
        let _ = write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "exit"
            }),
        );
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn lock_lsp_session(
    session: &Arc<Mutex<SourceLspSession>>,
) -> Result<std::sync::MutexGuard<'_, SourceLspSession>, String> {
    session
        .lock()
        .map_err(|_| "Language server session lock poisoned".to_string())
}

pub(crate) fn read_source_lsp_status_sync(
    root: PathBuf,
    language: String,
) -> Result<SourceLspStatus, String> {
    let Some(spec) = server_spec_for_language(&language) else {
        return Ok(SourceLspStatus {
            language,
            language_id: "plaintext".to_string(),
            available: false,
            server_name: "none".to_string(),
            command: String::new(),
            args: Vec::new(),
            reason: Some("No language server configured for this file type".to_string()),
        });
    };

    let root_exists = root.is_dir();
    let resolved = resolve_command(spec.command);
    let available = root_exists && resolved.is_some();
    let command = resolved.unwrap_or_else(|| spec.command.to_string());
    Ok(SourceLspStatus {
        language,
        language_id: spec.language_id.to_string(),
        available,
        server_name: spec.server_name.to_string(),
        command,
        args: spec.args.iter().map(|arg| (*arg).to_string()).collect(),
        reason: if !root_exists {
            Some("Project root is not a directory".to_string())
        } else if !available {
            Some(format!("{} is not installed or not on PATH", spec.command))
        } else {
            None
        },
    })
}

fn wait_for_lsp_response(
    receiver: &mpsc::Receiver<LspReaderMessage>,
    id: i64,
    deadline: Instant,
) -> Result<Value, String> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err("Language server timed out".to_string());
        }

        match receiver.recv_timeout(deadline.saturating_duration_since(now)) {
            Ok(LspReaderMessage::Message(message)) => {
                if message.get("id").and_then(Value::as_i64) == Some(id) {
                    return Ok(message);
                }
            }
            Ok(LspReaderMessage::Error(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err("Language server timed out".to_string());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Language server exited before responding".to_string());
            }
        }
    }
}

fn write_lsp_message<W: Write>(writer: &mut W, message: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(message)
        .map_err(|error| format!("Could not encode LSP message: {error}"))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())
        .map_err(|error| format!("Could not write LSP header: {error}"))?;
    writer
        .write_all(&body)
        .map_err(|error| format!("Could not write LSP body: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("Could not flush LSP message: {error}"))
}

fn read_lsp_message<R: Read>(reader: &mut R) -> io::Result<Value> {
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        reader.read_exact(&mut byte)?;
        header.push(byte[0]);
        if header.len() > MAX_LSP_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP header exceeded size limit",
            ));
        }
    }

    let header_text = String::from_utf8(header)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length"))?;

    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn lsp_position(request: &SourceLspLookupRequest) -> Value {
    json!({
        "line": request.line.saturating_sub(1),
        "character": request.column.saturating_sub(1)
    })
}

fn lsp_locations_from_result(result: &Value) -> Vec<LspLocation> {
    match result {
        Value::Array(values) => values.iter().filter_map(lsp_location_from_value).collect(),
        Value::Object(_) => lsp_location_from_value(result).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn lsp_location_from_value(value: &Value) -> Option<LspLocation> {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))
        .and_then(Value::as_str)?;
    let range = value
        .get("range")
        .or_else(|| value.get("targetSelectionRange"))
        .or_else(|| value.get("targetRange"))?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as usize + 1;
    let column = start.get("character")?.as_u64()? as usize + 1;
    Some(LspLocation {
        path: file_uri_to_path(uri)?,
        line,
        column,
    })
}

fn hover_contents_from_result(result: &Value) -> Vec<String> {
    result
        .get("contents")
        .map(hover_contents_from_value)
        .unwrap_or_default()
}

fn hover_contents_from_value(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => vec![text.clone()],
        Value::Array(values) => values
            .iter()
            .flat_map(hover_contents_from_value)
            .filter(|text| !text.trim().is_empty())
            .collect(),
        Value::Object(map) => {
            if let Some(text) = map.get("value").and_then(Value::as_str) {
                if let Some(language) = map.get("language").and_then(Value::as_str) {
                    return vec![format!("```{language}\n{text}\n```")];
                }
                return vec![text.to_string()];
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn diagnostics_from_message(message: &Value, file_uri: &str) -> Vec<SourceLspDiagnostic> {
    if message.get("method").and_then(Value::as_str) != Some("textDocument/publishDiagnostics") {
        return Vec::new();
    }

    let Some(params) = message.get("params") else {
        return Vec::new();
    };
    if params.get("uri").and_then(Value::as_str) != Some(file_uri) {
        return Vec::new();
    }

    params
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|diagnostics| {
            diagnostics
                .iter()
                .filter_map(lsp_diagnostic_from_value)
                .collect()
        })
        .unwrap_or_default()
}

fn diagnostic_uri_from_message(message: &Value) -> Option<String> {
    if message.get("method").and_then(Value::as_str) != Some("textDocument/publishDiagnostics") {
        return None;
    }
    message
        .get("params")?
        .get("uri")?
        .as_str()
        .map(|uri| uri.to_string())
}

fn lsp_diagnostic_from_value(value: &Value) -> Option<SourceLspDiagnostic> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as usize + 1;
    let column = start.get("character")?.as_u64()? as usize + 1;
    let message = value.get("message")?.as_str()?.trim().to_string();
    if message.is_empty() {
        return None;
    }

    Some(SourceLspDiagnostic {
        severity: lsp_diagnostic_severity(value.get("severity").and_then(Value::as_u64)),
        message,
        line,
        column,
        source: value
            .get("source")
            .and_then(Value::as_str)
            .map(|source| source.to_string()),
    })
}

fn lsp_diagnostic_severity(severity: Option<u64>) -> String {
    match severity {
        Some(1) => "error",
        Some(2) => "warning",
        Some(4) => "hint",
        _ => "info",
    }
    .to_string()
}

fn lsp_symbols_from_result(result: &Value, limit: usize) -> Vec<SourceLspSymbol> {
    let mut symbols = Vec::new();
    if let Value::Array(items) = result {
        for item in items {
            collect_lsp_symbol(item, &mut symbols, limit);
            if symbols.len() >= limit {
                break;
            }
        }
    }
    symbols
}

fn collect_lsp_symbol(value: &Value, symbols: &mut Vec<SourceLspSymbol>, limit: usize) {
    if symbols.len() >= limit {
        return;
    }

    if let Some(symbol) = lsp_symbol_from_value(value) {
        symbols.push(symbol);
    }

    if let Some(children) = value.get("children").and_then(Value::as_array) {
        for child in children {
            collect_lsp_symbol(child, symbols, limit);
            if symbols.len() >= limit {
                break;
            }
        }
    }
}

fn lsp_symbol_from_value(value: &Value) -> Option<SourceLspSymbol> {
    let name = value.get("name")?.as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let kind = lsp_symbol_kind(value.get("kind").and_then(Value::as_u64));
    let detail = value
        .get("detail")
        .or_else(|| value.get("containerName"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let start = value
        .get("selectionRange")
        .or_else(|| value.get("range"))
        .or_else(|| {
            value
                .get("location")
                .and_then(|location| location.get("range"))
        })?
        .get("start")?;
    let line = start.get("line")?.as_u64()? as usize + 1;
    let column = start.get("character")?.as_u64()? as usize + 1;

    Some(SourceLspSymbol {
        name,
        kind,
        line,
        column,
        detail,
    })
}

fn lsp_symbol_kind(kind: Option<u64>) -> String {
    match kind {
        Some(1) => "file",
        Some(2) => "module",
        Some(3) => "namespace",
        Some(4) => "package",
        Some(5) => "class",
        Some(6) => "method",
        Some(7) => "property",
        Some(8) => "field",
        Some(9) => "constructor",
        Some(10) => "enum",
        Some(11) => "interface",
        Some(12) => "function",
        Some(13) => "variable",
        Some(14) => "constant",
        Some(15) => "string",
        Some(16) => "number",
        Some(17) => "boolean",
        Some(18) => "array",
        Some(19) => "object",
        Some(20) => "key",
        Some(21) => "null",
        Some(22) => "enumMember",
        Some(23) => "struct",
        Some(24) => "event",
        Some(25) => "operator",
        Some(26) => "typeParameter",
        _ => "symbol",
    }
    .to_string()
}

fn definition_target_from_location(
    preview: &SourceLspPreview,
    request: &SourceLspLookupRequest,
    location: &LspLocation,
    symbol_name: &str,
) -> SourceLspDefinitionTarget {
    let metadata = std::fs::metadata(&location.path).ok();
    let relative_path = relative_path_for(&location.path, Path::new(&request.root));
    SourceLspDefinitionTarget {
        path: location.path.display().to_string(),
        file_name: file_name_for(&location.path),
        language: language_for_path(&location.path).unwrap_or_else(|| preview.language.clone()),
        byte_count: metadata.map(|metadata| metadata.len()).unwrap_or(0),
        relative_path: relative_path.clone(),
        symbol_name: symbol_name.to_string(),
        kind: "lsp".to_string(),
        line: location.line,
        column: location.column,
        detail: format!("{relative_path}:{}:{}", location.line, location.column),
    }
}

fn reference_target_from_location(
    preview: &SourceLspPreview,
    request: &SourceLspLookupRequest,
    location: &LspLocation,
    symbol_name: &str,
) -> SourceLspReferenceTarget {
    let metadata = std::fs::metadata(&location.path).ok();
    SourceLspReferenceTarget {
        path: location.path.display().to_string(),
        relative_path: relative_path_for(&location.path, Path::new(&request.root)),
        file_name: file_name_for(&location.path),
        language: language_for_path(&location.path).unwrap_or_else(|| preview.language.clone()),
        byte_count: metadata.map(|metadata| metadata.len()).unwrap_or(0),
        symbol_name: symbol_name.to_string(),
        line: location.line,
        column: location.column,
        excerpt: source_line_excerpt(preview, location),
    }
}

fn source_line_excerpt(preview: &SourceLspPreview, location: &LspLocation) -> String {
    let content = if Path::new(&preview.path) == location.path {
        preview.content.clone()
    } else {
        std::fs::read_to_string(&location.path).unwrap_or_default()
    };
    content
        .lines()
        .nth(location.line.saturating_sub(1))
        .map(compact_line)
        .unwrap_or_default()
}

fn relative_path_for(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn file_name_for(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source")
        .to_string()
}

fn language_for_path(path: &Path) -> Option<String> {
    match path.extension().and_then(|extension| extension.to_str())? {
        "cs" => Some("csharp".to_string()),
        "ts" => Some("typescript".to_string()),
        "tsx" => Some("typescript".to_string()),
        "js" => Some("javascript".to_string()),
        "jsx" => Some("javascript".to_string()),
        _ => None,
    }
}

fn symbol_at_position(content: &str, line: usize, column: usize) -> Option<String> {
    let line_text = content.lines().nth(line.saturating_sub(1))?;
    let chars: Vec<char> = line_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut index = column.saturating_sub(1).min(chars.len().saturating_sub(1));
    if !is_symbol_char(chars[index]) && index > 0 {
        index -= 1;
    }
    if !is_symbol_char(chars[index]) {
        return None;
    }

    let mut start = index;
    while start > 0 && is_symbol_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = index + 1;
    while end < chars.len() && is_symbol_char(chars[end]) {
        end += 1;
    }

    Some(chars[start..end].iter().collect())
}

fn is_symbol_char(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn compact_line(line: &str) -> String {
    let compacted = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if compacted.len() > 140 {
        format!("{}...", &compacted[..137])
    } else {
        compacted
    }
}

fn resolve_server_for_language(language: &str) -> Option<ResolvedLspServer> {
    let spec = server_spec_for_language(language)?;
    let command = resolve_command(spec.command)?;
    Some(ResolvedLspServer { spec, command })
}

fn server_spec_for_language(language: &str) -> Option<LspServerSpec> {
    match language.trim().to_lowercase().as_str() {
        "csharp" | "c#" | "cs" => Some(LspServerSpec {
            server_name: "csharp-ls",
            language_id: "csharp",
            command: "csharp-ls",
            args: &[],
        }),
        "typescript" | "ts" | "tsx" => Some(LspServerSpec {
            server_name: "typescript-language-server",
            language_id: "typescript",
            command: "typescript-language-server",
            args: &["--stdio"],
        }),
        "javascript" | "js" | "jsx" => Some(LspServerSpec {
            server_name: "typescript-language-server",
            language_id: "javascript",
            command: "typescript-language-server",
            args: &["--stdio"],
        }),
        _ => None,
    }
}

fn resolve_command(command: &str) -> Option<String> {
    if command.contains('/') {
        return executable_path(PathBuf::from(command));
    }

    command_search_paths()
        .into_iter()
        .map(|path| path.join(command))
        .find_map(executable_path)
}

fn normalized_lsp_root(root: &str) -> Option<String> {
    let path = PathBuf::from(root);
    if !path.is_dir() {
        return None;
    }

    Some(
        std::fs::canonicalize(&path)
            .unwrap_or(path)
            .display()
            .to_string(),
    )
}

fn executable_path(path: PathBuf) -> Option<String> {
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    Some(path.display().to_string())
}

fn command_search_paths() -> Vec<PathBuf> {
    let mut paths = env::var_os("PATH").map(split_paths).unwrap_or_default();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        paths.push(home.join(".dotnet/tools"));
        paths.extend(nvm_bin_paths(&home));
    }
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    dedupe_paths(paths)
}

fn split_paths(paths: OsString) -> Vec<PathBuf> {
    env::split_paths(&paths).collect()
}

fn nvm_bin_paths(home: &Path) -> Vec<PathBuf> {
    let versions = home.join(".nvm/versions/node");
    let Ok(entries) = std::fs::read_dir(versions) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path().join("bin"))
        .filter(|path| path.is_dir())
        .collect()
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|candidate: &PathBuf| candidate == &path) {
            deduped.push(path);
        }
    }
    deduped
}

fn path_to_file_uri(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    format!(
        "file://{}",
        percent_encode_path(&absolute.display().to_string())
    )
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    percent_decode_path(raw).map(PathBuf::from)
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn percent_decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_language_server_specs() {
        let csharp = server_spec_for_language("csharp").expect("csharp spec");
        assert_eq!(csharp.command, "csharp-ls");
        assert_eq!(csharp.language_id, "csharp");

        let typescript = server_spec_for_language("typescript").expect("typescript spec");
        assert_eq!(typescript.command, "typescript-language-server");
        assert_eq!(typescript.args, &["--stdio"]);
    }

    #[test]
    fn diagnostics_timeout_stays_bounded() {
        assert!(LSP_DIAGNOSTICS_TIMEOUT < LSP_REQUEST_TIMEOUT);
        assert!(LSP_DIAGNOSTICS_TIMEOUT <= Duration::from_millis(1500));
    }

    #[test]
    fn builds_session_key_from_root_and_language() {
        let root = env::temp_dir();
        let preview = SourceLspPreview {
            path: root.join("App.ts").display().to_string(),
            relative_path: "App.ts".to_string(),
            file_name: "App.ts".to_string(),
            language: "typescript".to_string(),
            byte_count: 16,
            content: "export {};".to_string(),
            line_count: 1,
        };
        let request = SourceLspLookupRequest {
            root: root.display().to_string(),
            line: 1,
            column: 1,
            limit: None,
        };

        let key = SourceLspSessionKey::from_preview(&preview, &request).expect("session key");
        assert_eq!(key.language, "typescript");
        assert_eq!(
            key.root,
            std::fs::canonicalize(root).unwrap().display().to_string()
        );
    }

    #[test]
    fn reads_lsp_message_with_content_length() {
        let body = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes();
        let message = read_lsp_message(&mut input.as_slice()).expect("message");
        assert_eq!(message.get("id").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn extracts_locations_from_location_link() {
        let result = json!([
            {
                "targetUri": "file:///Users/blackcolours/dev/work/EdiPlatform/App.cs",
                "targetSelectionRange": {
                    "start": { "line": 9, "character": 4 },
                    "end": { "line": 9, "character": 7 }
                }
            }
        ]);
        let locations = lsp_locations_from_result(&result);
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].line, 10);
        assert_eq!(locations[0].column, 5);
        assert_eq!(
            locations[0].path,
            PathBuf::from("/Users/blackcolours/dev/work/EdiPlatform/App.cs")
        );
    }

    #[test]
    fn extracts_publish_diagnostics_for_current_file() {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///tmp/App.ts",
                "diagnostics": [
                    {
                        "range": {
                            "start": { "line": 4, "character": 8 },
                            "end": { "line": 4, "character": 11 }
                        },
                        "severity": 1,
                        "source": "typescript",
                        "message": "Cannot find name 'foo'."
                    },
                    {
                        "range": {
                            "start": { "line": 7, "character": 2 },
                            "end": { "line": 7, "character": 8 }
                        },
                        "severity": 4,
                        "message": "Unnecessary assignment."
                    }
                ]
            }
        });

        let diagnostics = diagnostics_from_message(&message, "file:///tmp/App.ts");
        assert_eq!(
            diagnostics,
            vec![
                SourceLspDiagnostic {
                    severity: "error".to_string(),
                    message: "Cannot find name 'foo'.".to_string(),
                    line: 5,
                    column: 9,
                    source: Some("typescript".to_string()),
                },
                SourceLspDiagnostic {
                    severity: "hint".to_string(),
                    message: "Unnecessary assignment.".to_string(),
                    line: 8,
                    column: 3,
                    source: None,
                }
            ]
        );
        assert!(diagnostics_from_message(&message, "file:///tmp/Other.ts").is_empty());
    }

    #[test]
    fn extracts_nested_document_symbols() {
        let result = json!([
            {
                "name": "FormatResolver",
                "kind": 5,
                "detail": "class",
                "selectionRange": {
                    "start": { "line": 7, "character": 20 },
                    "end": { "line": 7, "character": 34 }
                },
                "range": {
                    "start": { "line": 7, "character": 0 },
                    "end": { "line": 40, "character": 1 }
                },
                "children": [
                    {
                        "name": "Resolve",
                        "kind": 6,
                        "selectionRange": {
                            "start": { "line": 16, "character": 22 },
                            "end": { "line": 16, "character": 29 }
                        },
                        "range": {
                            "start": { "line": 16, "character": 2 },
                            "end": { "line": 24, "character": 3 }
                        }
                    }
                ]
            }
        ]);

        let symbols = lsp_symbols_from_result(&result, 20);
        assert_eq!(
            symbols,
            vec![
                SourceLspSymbol {
                    name: "FormatResolver".to_string(),
                    kind: "class".to_string(),
                    line: 8,
                    column: 21,
                    detail: "class".to_string(),
                },
                SourceLspSymbol {
                    name: "Resolve".to_string(),
                    kind: "method".to_string(),
                    line: 17,
                    column: 23,
                    detail: String::new(),
                }
            ]
        );
    }

    #[test]
    fn extracts_symbol_at_position() {
        let source = "public sealed class FormatResolver\n{\n}";
        assert_eq!(
            symbol_at_position(source, 1, 23).as_deref(),
            Some("FormatResolver")
        );
    }

    #[test]
    fn round_trips_file_uri_encoding() {
        let path = PathBuf::from("/tmp/source file #1.cs");
        let uri = path_to_file_uri(&path);
        assert_eq!(uri, "file:///tmp/source%20file%20%231.cs");
        assert_eq!(file_uri_to_path(&uri), Some(path));
    }
}
