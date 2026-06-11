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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspRenameRequest {
    root: String,
    line: usize,
    column: usize,
    new_name: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspWorkspaceSymbolRequest {
    root: String,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspCodeActionDiagnostic {
    severity: String,
    message: String,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    source: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspCodeActionRequest {
    root: String,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    diagnostics: Vec<SourceLspCodeActionDiagnostic>,
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
pub(crate) struct SourceLspSignatureParameter {
    label: String,
    documentation: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspSignature {
    label: String,
    documentation: String,
    parameters: Vec<SourceLspSignatureParameter>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspSignatureHelp {
    signatures: Vec<SourceLspSignature>,
    active_signature: usize,
    active_parameter: usize,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspInlayHint {
    label: String,
    tooltip: String,
    kind: String,
    line: usize,
    column: usize,
    padding_left: bool,
    padding_right: bool,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspSemanticToken {
    token_type: String,
    line: usize,
    start_column: usize,
    length: usize,
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
pub(crate) struct SourceLspWorkspaceSymbol {
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
    container_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspDocumentHighlight {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    kind: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspCompletionItem {
    label: String,
    kind: String,
    detail: String,
    insert_text: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspTextEdit {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    new_text: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspWorkspaceEditFile {
    path: String,
    relative_path: String,
    edits: Vec<SourceLspTextEdit>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspRenameResult {
    files: Vec<SourceLspWorkspaceEditFile>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLspCodeAction {
    title: String,
    kind: String,
    is_preferred: bool,
    disabled_reason: Option<String>,
    files: Vec<SourceLspWorkspaceEditFile>,
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
    semantic_token_types: Vec<String>,
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

    pub(crate) fn find_implementations(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspDefinitionTarget>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/implementation")? else {
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

    pub(crate) fn find_type_definitions(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspDefinitionTarget>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/typeDefinition")? else {
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

    pub(crate) fn find_document_highlights(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspDocumentHighlight>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/documentHighlight")?
        else {
            return Ok(Vec::new());
        };

        Ok(lsp_document_highlights_from_result(&result))
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

    pub(crate) fn find_signature_help(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Option<SourceLspSignatureHelp>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/signatureHelp")? else {
            return Ok(None);
        };

        Ok(lsp_signature_help_from_result(&result))
    }

    pub(crate) fn find_inlay_hints(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspInlayHint>, String> {
        let limit = request.limit.unwrap_or(200);
        let Some(result) = self.request(&preview, &request, "textDocument/inlayHint")? else {
            return Ok(Vec::new());
        };

        Ok(lsp_inlay_hints_from_result(&result, limit))
    }

    pub(crate) fn find_semantic_tokens(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspSemanticToken>, String> {
        let limit = request.limit.unwrap_or(5_000);
        for attempt in 0..2 {
            let Some(session) = self.session_for(&preview, &request)? else {
                return Ok(Vec::new());
            };
            let (result, legend, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.request_semantic_tokens(&preview);
                let legend = session.semantic_token_types.clone();
                let session_alive = session.is_alive();
                (result, legend, session_alive)
            };

            match result {
                Ok(Some(value)) => {
                    return Ok(lsp_semantic_tokens_from_result(&value, &legend, limit));
                }
                Ok(None) => return Ok(Vec::new()),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(&preview, &request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(Vec::new())
    }

    pub(crate) fn find_workspace_symbols(
        &self,
        preview: SourceLspPreview,
        request: SourceLspWorkspaceSymbolRequest,
    ) -> Result<Vec<SourceLspWorkspaceSymbol>, String> {
        let query = request.query.trim().to_string();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let lookup_request = SourceLspLookupRequest {
            root: request.root.clone(),
            line: 1,
            column: 1,
            limit: request.limit,
        };
        let limit = lookup_request.limit.unwrap_or(50);
        for attempt in 0..2 {
            let Some(session) = self.session_for(&preview, &lookup_request)? else {
                return Ok(Vec::new());
            };
            let (result, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.request_workspace_symbols(&preview, &query);
                let session_alive = session.is_alive();
                (result, session_alive)
            };

            match result {
                Ok(Some(value)) => {
                    return Ok(lsp_workspace_symbols_from_result(
                        &value,
                        Path::new(&request.root),
                        &preview.language,
                        limit,
                    ));
                }
                Ok(None) => return Ok(Vec::new()),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(&preview, &lookup_request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(Vec::new())
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

    pub(crate) fn find_completions(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspCompletionItem>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/completion")? else {
            return Ok(Vec::new());
        };

        Ok(lsp_completion_items_from_result(
            &result,
            request.limit.unwrap_or(50),
        ))
    }

    pub(crate) fn format_document(
        &self,
        preview: SourceLspPreview,
        request: SourceLspLookupRequest,
    ) -> Result<Vec<SourceLspTextEdit>, String> {
        let Some(result) = self.request(&preview, &request, "textDocument/formatting")? else {
            return Ok(Vec::new());
        };

        Ok(lsp_text_edits_from_result(&result))
    }

    pub(crate) fn rename(
        &self,
        preview: SourceLspPreview,
        request: SourceLspRenameRequest,
    ) -> Result<SourceLspRenameResult, String> {
        let lookup_request = SourceLspLookupRequest {
            root: request.root.clone(),
            line: request.line,
            column: request.column,
            limit: None,
        };

        for attempt in 0..2 {
            let Some(session) = self.session_for(&preview, &lookup_request)? else {
                return Ok(SourceLspRenameResult { files: Vec::new() });
            };
            let (result, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.request_rename(&preview, &request);
                let session_alive = session.is_alive();
                (result, session_alive)
            };

            match result {
                Ok(Some(value)) => {
                    return Ok(SourceLspRenameResult {
                        files: lsp_workspace_edit_files_from_result(
                            &value,
                            Path::new(&request.root),
                        ),
                    });
                }
                Ok(None) => return Ok(SourceLspRenameResult { files: Vec::new() }),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(&preview, &lookup_request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(SourceLspRenameResult { files: Vec::new() })
    }

    pub(crate) fn find_code_actions(
        &self,
        preview: SourceLspPreview,
        request: SourceLspCodeActionRequest,
    ) -> Result<Vec<SourceLspCodeAction>, String> {
        let lookup_request = SourceLspLookupRequest {
            root: request.root.clone(),
            line: request.start_line,
            column: request.start_column,
            limit: request.limit,
        };

        for attempt in 0..2 {
            let Some(session) = self.session_for(&preview, &lookup_request)? else {
                return Ok(Vec::new());
            };
            let (result, session_alive) = {
                let mut session = lock_lsp_session(&session)?;
                let result = session.request_code_actions(&preview, &request);
                let session_alive = session.is_alive();
                (result, session_alive)
            };

            match result {
                Ok(Some(value)) => {
                    return Ok(lsp_code_actions_from_result(
                        &value,
                        Path::new(&request.root),
                        request.limit.unwrap_or(50),
                    ));
                }
                Ok(None) => return Ok(Vec::new()),
                Err(_) if attempt == 0 && !session_alive => {
                    self.remove_session(&preview, &lookup_request)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(Vec::new())
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
                    "capabilities": lsp_client_capabilities()
                }
            }),
        )?;
        let response = wait_for_lsp_response(&receiver, 1, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server initialize failed: {error}"));
        }
        let semantic_token_types = lsp_semantic_token_types_from_initialize(&response);
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
            semantic_token_types,
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
            "textDocument/formatting" => json!({
                "textDocument": { "uri": file_uri },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true,
                    "trimTrailingWhitespace": true,
                    "insertFinalNewline": true,
                    "trimFinalNewlines": true
                }
            }),
            "textDocument/inlayHint" => json!({
                "textDocument": { "uri": file_uri },
                "range": lsp_full_document_range(preview)
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

    fn request_rename(
        &mut self,
        preview: &SourceLspPreview,
        request: &SourceLspRenameRequest,
    ) -> Result<Option<Value>, String> {
        let file_uri = self.ensure_document_open(preview)?;
        let id = self.next_request_id();
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/rename",
                "params": {
                    "textDocument": { "uri": file_uri },
                    "position": {
                        "line": request.line.saturating_sub(1),
                        "character": request.column.saturating_sub(1)
                    },
                    "newName": request.new_name.clone()
                }
            }),
        )?;
        let response = self.wait_for_response(id, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server request failed: {error}"));
        }

        Ok(response.get("result").cloned())
    }

    fn request_code_actions(
        &mut self,
        preview: &SourceLspPreview,
        request: &SourceLspCodeActionRequest,
    ) -> Result<Option<Value>, String> {
        let file_uri = self.ensure_document_open(preview)?;
        let id = self.next_request_id();
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/codeAction",
                "params": {
                    "textDocument": { "uri": file_uri },
                    "range": lsp_code_action_range(request),
                    "context": {
                        "diagnostics": request
                            .diagnostics
                            .iter()
                            .map(lsp_code_action_diagnostic)
                            .collect::<Vec<_>>()
                    }
                }
            }),
        )?;
        let response = self.wait_for_response(id, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server request failed: {error}"));
        }

        Ok(response.get("result").cloned())
    }

    fn request_semantic_tokens(
        &mut self,
        preview: &SourceLspPreview,
    ) -> Result<Option<Value>, String> {
        let file_uri = self.ensure_document_open(preview)?;
        let id = self.next_request_id();
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/semanticTokens/full",
                "params": {
                    "textDocument": { "uri": file_uri }
                }
            }),
        )?;
        let response = self.wait_for_response(id, Instant::now() + LSP_REQUEST_TIMEOUT)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Language server request failed: {error}"));
        }

        Ok(response.get("result").cloned())
    }

    fn request_workspace_symbols(
        &mut self,
        preview: &SourceLspPreview,
        query: &str,
    ) -> Result<Option<Value>, String> {
        self.ensure_document_open(preview)?;
        let id = self.next_request_id();
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "workspace/symbol",
                "params": {
                    "query": query
                }
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
        let published = self
            .diagnostics_by_uri
            .get(&file_uri)
            .cloned()
            .unwrap_or_default();
        if !published.is_empty() {
            return Ok(published);
        }

        Ok(self.request_pull_diagnostics(&file_uri).unwrap_or_default())
    }

    fn request_pull_diagnostics(&mut self, file_uri: &str) -> Option<Vec<SourceLspDiagnostic>> {
        let id = self.next_request_id();
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/diagnostic",
                "params": {
                    "textDocument": { "uri": file_uri }
                }
            }),
        )
        .ok()?;
        let response = self
            .wait_for_response(id, Instant::now() + LSP_DIAGNOSTICS_TIMEOUT)
            .ok()?;
        if response.get("error").is_some() {
            return None;
        }
        let diagnostics = diagnostics_from_pull_result(response.get("result")?);
        if !diagnostics.is_empty() {
            self.diagnostics_by_uri
                .insert(file_uri.to_string(), diagnostics.clone());
        }
        Some(diagnostics)
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

fn lsp_code_action_range(request: &SourceLspCodeActionRequest) -> Value {
    json!({
        "start": {
            "line": request.start_line.saturating_sub(1),
            "character": request.start_column.saturating_sub(1)
        },
        "end": {
            "line": request.end_line.saturating_sub(1),
            "character": request.end_column.saturating_sub(1)
        }
    })
}

fn lsp_full_document_range(preview: &SourceLspPreview) -> Value {
    let line_count = preview.line_count.max(1);
    json!({
        "start": {
            "line": 0,
            "character": 0
        },
        "end": {
            "line": line_count.saturating_sub(1),
            "character": last_line_utf16_len(&preview.content)
        }
    })
}

fn last_line_utf16_len(content: &str) -> usize {
    content
        .rsplit_once('\n')
        .map(|(_, last_line)| last_line)
        .unwrap_or(content)
        .encode_utf16()
        .count()
}

fn lsp_code_action_diagnostic(diagnostic: &SourceLspCodeActionDiagnostic) -> Value {
    json!({
        "range": {
            "start": {
                "line": diagnostic.start_line.saturating_sub(1),
                "character": diagnostic.start_column.saturating_sub(1)
            },
            "end": {
                "line": diagnostic.end_line.saturating_sub(1),
                "character": diagnostic.end_column.saturating_sub(1)
            }
        },
        "severity": lsp_diagnostic_severity_code(&diagnostic.severity),
        "message": diagnostic.message,
        "source": diagnostic.source
    })
}

fn lsp_diagnostic_severity_code(severity: &str) -> u8 {
    match severity {
        "error" => 1,
        "warning" => 2,
        "info" => 3,
        "hint" => 4,
        _ => 3,
    }
}

fn lsp_semantic_token_types_from_initialize(response: &Value) -> Vec<String> {
    response
        .get("result")
        .and_then(|result| result.get("capabilities"))
        .and_then(|capabilities| capabilities.get("semanticTokensProvider"))
        .and_then(|provider| provider.get("legend"))
        .and_then(|legend| legend.get("tokenTypes"))
        .and_then(Value::as_array)
        .map(|token_types| {
            token_types
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn lsp_client_capabilities() -> Value {
    json!({
        "textDocument": {
            "codeAction": {
                "codeActionLiteralSupport": {
                    "codeActionKind": {
                        "valueSet": [
                            "",
                            "quickfix",
                            "refactor",
                            "refactor.extract",
                            "refactor.inline",
                            "refactor.rewrite",
                            "source",
                            "source.organizeImports",
                            "source.fixAll"
                        ]
                    }
                },
                "isPreferredSupport": true
            },
            "completion": {
                "completionItem": {
                    "snippetSupport": false
                }
            },
            "signatureHelp": {
                "signatureInformation": {
                    "documentationFormat": ["markdown", "plaintext"],
                    "parameterInformation": {
                        "labelOffsetSupport": true
                    }
                }
            },
            "inlayHint": {
                "dynamicRegistration": false,
                "resolveSupport": {
                    "properties": [
                        "tooltip",
                        "textEdits",
                        "label.tooltip",
                        "label.location",
                        "label.command"
                    ]
                }
            },
            "semanticTokens": {
                "dynamicRegistration": false,
                "requests": {
                    "range": false,
                    "full": true
                },
                "tokenTypes": [
                    "namespace",
                    "type",
                    "class",
                    "enum",
                    "interface",
                    "struct",
                    "typeParameter",
                    "parameter",
                    "variable",
                    "property",
                    "enumMember",
                    "event",
                    "function",
                    "method",
                    "macro",
                    "keyword",
                    "modifier",
                    "comment",
                    "string",
                    "number",
                    "regexp",
                    "operator"
                ],
                "tokenModifiers": [],
                "formats": ["relative"],
                "overlappingTokenSupport": false,
                "multilineTokenSupport": false
            },
            "definition": {
                "linkSupport": true
            },
            "implementation": {
                "linkSupport": true
            },
            "typeDefinition": {
                "linkSupport": true
            },
            "publishDiagnostics": {
                "relatedInformation": true,
                "tagSupport": {
                    "valueSet": [1, 2]
                }
            }
        }
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

fn lsp_signature_help_from_result(result: &Value) -> Option<SourceLspSignatureHelp> {
    let signatures = result
        .get("signatures")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(lsp_signature_from_value)
        .collect::<Vec<_>>();
    if signatures.is_empty() {
        return None;
    }

    Some(SourceLspSignatureHelp {
        active_signature: result
            .get("activeSignature")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        active_parameter: result
            .get("activeParameter")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        signatures,
    })
}

fn lsp_inlay_hints_from_result(result: &Value, limit: usize) -> Vec<SourceLspInlayHint> {
    let Some(items) = result.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(lsp_inlay_hint_from_value)
        .take(limit)
        .collect()
}

fn lsp_inlay_hint_from_value(value: &Value) -> Option<SourceLspInlayHint> {
    let position = value.get("position")?;
    let label = lsp_inlay_hint_label(value.get("label")?);
    if label.trim().is_empty() {
        return None;
    }
    let mut tooltip = lsp_documentation_from_value(value.get("tooltip"));
    if tooltip.is_empty() {
        tooltip = lsp_inlay_hint_label_tooltip(value.get("label"));
    }

    Some(SourceLspInlayHint {
        label,
        tooltip,
        kind: lsp_inlay_hint_kind(value.get("kind").and_then(Value::as_u64)).to_string(),
        line: position.get("line")?.as_u64()? as usize + 1,
        column: position.get("character")?.as_u64()? as usize + 1,
        padding_left: value
            .get("paddingLeft")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        padding_right: value
            .get("paddingRight")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn lsp_inlay_hint_label(value: &Value) -> String {
    if let Some(label) = value.as_str() {
        return label.to_string();
    }

    value
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("value").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn lsp_inlay_hint_label_tooltip(value: Option<&Value>) -> String {
    let Some(Value::Array(parts)) = value else {
        return String::new();
    };

    parts
        .iter()
        .find_map(|part| {
            let tooltip = lsp_documentation_from_value(part.get("tooltip"));
            if tooltip.is_empty() {
                None
            } else {
                Some(tooltip)
            }
        })
        .unwrap_or_default()
}

fn lsp_inlay_hint_kind(kind: Option<u64>) -> &'static str {
    match kind {
        Some(1) => "type",
        Some(2) => "parameter",
        _ => "other",
    }
}

fn lsp_semantic_tokens_from_result(
    result: &Value,
    legend: &[String],
    limit: usize,
) -> Vec<SourceLspSemanticToken> {
    let Some(data) = result.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut tokens = Vec::new();
    let mut current_line = 0_usize;
    let mut current_start = 0_usize;

    for chunk in data.chunks(5) {
        if chunk.len() != 5 || tokens.len() >= limit {
            break;
        }

        let delta_line = chunk[0].as_u64().unwrap_or(0) as usize;
        let delta_start = chunk[1].as_u64().unwrap_or(0) as usize;
        let length = chunk[2].as_u64().unwrap_or(0) as usize;
        let token_type_index = chunk[3].as_u64().unwrap_or(u64::MAX) as usize;

        current_line += delta_line;
        current_start = if delta_line == 0 {
            current_start + delta_start
        } else {
            delta_start
        };

        if length == 0 {
            continue;
        }
        let Some(raw_token_type) = legend.get(token_type_index) else {
            continue;
        };
        let Some(token_type) = normalized_semantic_token_type(raw_token_type) else {
            continue;
        };

        tokens.push(SourceLspSemanticToken {
            token_type: token_type.to_string(),
            line: current_line + 1,
            start_column: current_start + 1,
            length,
        });
    }

    tokens
}

fn normalized_semantic_token_type(raw: &str) -> Option<&'static str> {
    match raw {
        "namespace" => Some("namespace"),
        "class" | "struct" => Some("class"),
        "interface" => Some("interface"),
        "type" => Some("type"),
        "enum" => Some("enum"),
        "function" => Some("function"),
        "method" | "constructor" => Some("method"),
        "property" => Some("property"),
        "variable" | "local" => Some("variable"),
        "parameter" => Some("parameter"),
        "enumMember" => Some("enumMember"),
        "typeParameter" => Some("typeParameter"),
        "keyword" | "modifier" => Some("keyword"),
        "string" => Some("string"),
        "number" => Some("number"),
        "operator" => Some("operator"),
        "comment" => Some("comment"),
        _ => None,
    }
}

fn lsp_signature_from_value(value: &Value) -> Option<SourceLspSignature> {
    let label = value.get("label")?.as_str()?.trim().to_string();
    if label.is_empty() {
        return None;
    }

    let parameters = value
        .get("parameters")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|parameter| lsp_signature_parameter_from_value(parameter, &label))
                .collect()
        })
        .unwrap_or_default();

    Some(SourceLspSignature {
        documentation: lsp_documentation_from_value(value.get("documentation")),
        label,
        parameters,
    })
}

fn lsp_signature_parameter_from_value(
    value: &Value,
    signature_label: &str,
) -> Option<SourceLspSignatureParameter> {
    let label = lsp_signature_parameter_label(value.get("label")?, signature_label)?;
    if label.trim().is_empty() {
        return None;
    }

    Some(SourceLspSignatureParameter {
        label,
        documentation: lsp_documentation_from_value(value.get("documentation")),
    })
}

fn lsp_signature_parameter_label(value: &Value, signature_label: &str) -> Option<String> {
    if let Some(label) = value.as_str() {
        return Some(label.to_string());
    }

    let range = value.as_array()?;
    if range.len() != 2 {
        return None;
    }
    let start = range[0].as_u64()? as usize;
    let end = range[1].as_u64()? as usize;
    if start >= end {
        return None;
    }

    signature_label.get(start..end).map(str::to_string)
}

fn lsp_documentation_from_value(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };

    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Object(map) => map
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => String::new(),
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

fn diagnostics_from_pull_result(result: &Value) -> Vec<SourceLspDiagnostic> {
    result
        .get("items")
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

fn lsp_workspace_edit_files_from_result(
    result: &Value,
    root: &Path,
) -> Vec<SourceLspWorkspaceEditFile> {
    let mut files = Vec::new();

    if let Some(document_changes) = result.get("documentChanges").and_then(Value::as_array) {
        for document_change in document_changes {
            let Some(uri) = document_change
                .get("textDocument")
                .and_then(|text_document| text_document.get("uri"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let Some(edits) = document_change.get("edits").and_then(Value::as_array) else {
                continue;
            };
            push_lsp_workspace_edit_file(&mut files, uri, edits, root);
        }
    }

    if let Some(changes) = result.get("changes").and_then(Value::as_object) {
        for (uri, edits) in changes {
            let Some(edits) = edits.as_array() else {
                continue;
            };
            push_lsp_workspace_edit_file(&mut files, uri, edits, root);
        }
    }

    files
}

fn push_lsp_workspace_edit_file(
    files: &mut Vec<SourceLspWorkspaceEditFile>,
    uri: &str,
    edits: &[Value],
    root: &Path,
) {
    let Some(path) = file_uri_to_path(uri) else {
        return;
    };
    let parsed_edits = edits
        .iter()
        .filter_map(lsp_text_edit_from_value)
        .collect::<Vec<_>>();
    if parsed_edits.is_empty() {
        return;
    }

    files.push(SourceLspWorkspaceEditFile {
        path: path.display().to_string(),
        relative_path: relative_path_for(&path, root),
        edits: parsed_edits,
    });
}

fn lsp_code_actions_from_result(
    result: &Value,
    root: &Path,
    limit: usize,
) -> Vec<SourceLspCodeAction> {
    let Some(actions) = result.as_array() else {
        return Vec::new();
    };

    actions
        .iter()
        .filter_map(|action| lsp_code_action_from_value(action, root))
        .take(limit)
        .collect()
}

fn lsp_code_action_from_value(value: &Value, root: &Path) -> Option<SourceLspCodeAction> {
    let title = value.get("title").and_then(Value::as_str)?.to_string();
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("command")
        .to_string();
    let is_preferred = value
        .get("isPreferred")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let files = value
        .get("edit")
        .map(|edit| lsp_workspace_edit_files_from_result(edit, root))
        .unwrap_or_default();
    let disabled_reason = value
        .get("disabled")
        .and_then(|disabled| disabled.get("reason"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            if files.is_empty() && value.get("command").is_some() {
                Some("Command-only code action is not supported yet".to_string())
            } else {
                None
            }
        });

    Some(SourceLspCodeAction {
        title,
        kind,
        is_preferred,
        disabled_reason,
        files,
    })
}

fn lsp_document_highlights_from_result(result: &Value) -> Vec<SourceLspDocumentHighlight> {
    let Some(items) = result.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(lsp_document_highlight_from_value)
        .collect()
}

fn lsp_document_highlight_from_value(value: &Value) -> Option<SourceLspDocumentHighlight> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;

    Some(SourceLspDocumentHighlight {
        start_line: start.get("line")?.as_u64()? as usize + 1,
        start_column: start.get("character")?.as_u64()? as usize + 1,
        end_line: end.get("line")?.as_u64()? as usize + 1,
        end_column: end.get("character")?.as_u64()? as usize + 1,
        kind: lsp_document_highlight_kind(value.get("kind").and_then(Value::as_u64)).to_string(),
    })
}

fn lsp_document_highlight_kind(kind: Option<u64>) -> &'static str {
    match kind {
        Some(2) => "read",
        Some(3) => "write",
        _ => "text",
    }
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

fn lsp_workspace_symbols_from_result(
    result: &Value,
    root: &Path,
    fallback_language: &str,
    limit: usize,
) -> Vec<SourceLspWorkspaceSymbol> {
    let Some(items) = result.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|value| lsp_workspace_symbol_from_value(value, root, fallback_language))
        .take(limit)
        .collect()
}

fn lsp_workspace_symbol_from_value(
    value: &Value,
    root: &Path,
    fallback_language: &str,
) -> Option<SourceLspWorkspaceSymbol> {
    let symbol_name = value.get("name")?.as_str()?.trim().to_string();
    if symbol_name.is_empty() {
        return None;
    }

    let location = value.get("location")?;
    let uri = location
        .get("uri")
        .or_else(|| location.get("targetUri"))?
        .as_str()?;
    let path = file_uri_to_path(uri)?;
    let range = location
        .get("range")
        .or_else(|| location.get("targetSelectionRange"))
        .or_else(|| value.get("range"))?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as usize + 1;
    let column = start.get("character")?.as_u64()? as usize + 1;
    let relative_path = relative_path_for(&path, root);
    let file_name = file_name_for(&path);
    let container_name = value
        .get("containerName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|container| !container.is_empty())
        .map(str::to_string);
    let location_detail = format!("{relative_path}:{line}:{column}");
    let detail = container_name
        .as_ref()
        .map(|container| format!("{container} - {location_detail}"))
        .unwrap_or_else(|| location_detail.clone());
    let metadata = std::fs::metadata(&path).ok();

    Some(SourceLspWorkspaceSymbol {
        path: path.display().to_string(),
        relative_path,
        file_name,
        language: language_for_path(&path).unwrap_or_else(|| fallback_language.to_string()),
        byte_count: metadata.map(|metadata| metadata.len()).unwrap_or(0),
        symbol_name,
        kind: lsp_symbol_kind(value.get("kind").and_then(Value::as_u64)),
        line,
        column,
        detail,
        container_name,
    })
}

fn lsp_completion_items_from_result(result: &Value, limit: usize) -> Vec<SourceLspCompletionItem> {
    let items = result
        .as_array()
        .or_else(|| result.get("items").and_then(Value::as_array));
    let Some(items) = items else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(lsp_completion_item_from_value)
        .take(limit)
        .collect()
}

fn lsp_text_edits_from_result(result: &Value) -> Vec<SourceLspTextEdit> {
    let Some(items) = result.as_array() else {
        return Vec::new();
    };

    items.iter().filter_map(lsp_text_edit_from_value).collect()
}

fn lsp_text_edit_from_value(value: &Value) -> Option<SourceLspTextEdit> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;

    Some(SourceLspTextEdit {
        start_line: start.get("line")?.as_u64()? as usize + 1,
        start_column: start.get("character")?.as_u64()? as usize + 1,
        end_line: end.get("line")?.as_u64()? as usize + 1,
        end_column: end.get("character")?.as_u64()? as usize + 1,
        new_text: value
            .get("newText")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn lsp_completion_item_from_value(value: &Value) -> Option<SourceLspCompletionItem> {
    let label = value.get("label")?.as_str()?.trim().to_string();
    if label.is_empty() {
        return None;
    }

    let insert_text = value
        .get("insertText")
        .and_then(Value::as_str)
        .unwrap_or(&label)
        .trim()
        .to_string();

    Some(SourceLspCompletionItem {
        label,
        kind: lsp_completion_item_kind(value.get("kind").and_then(Value::as_u64)),
        detail: value
            .get("detail")
            .or_else(|| value.get("documentation").and_then(|doc| doc.get("value")))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        insert_text,
    })
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

fn lsp_completion_item_kind(kind: Option<u64>) -> String {
    match kind {
        Some(1) => "text",
        Some(2) => "method",
        Some(3) => "function",
        Some(4) => "constructor",
        Some(5) => "field",
        Some(6) => "variable",
        Some(7) => "class",
        Some(8) => "interface",
        Some(9) => "module",
        Some(10) => "property",
        Some(11) => "unit",
        Some(12) => "value",
        Some(13) => "enum",
        Some(14) => "keyword",
        Some(15) => "snippet",
        Some(16) => "color",
        Some(17) => "file",
        Some(18) => "reference",
        Some(19) => "folder",
        Some(20) => "enumMember",
        Some(21) => "constant",
        Some(22) => "struct",
        Some(23) => "event",
        Some(24) => "operator",
        Some(25) => "typeParameter",
        _ => "value",
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
        "rust" | "rs" => Some(LspServerSpec {
            server_name: "rust-analyzer",
            language_id: "rust",
            command: "rust-analyzer",
            args: &[],
        }),
        "svelte" => Some(LspServerSpec {
            server_name: "svelte-language-server",
            language_id: "svelte",
            command: "svelte-language-server",
            args: &["--stdio"],
        }),
        "python" | "py" => Some(LspServerSpec {
            server_name: "pyright",
            language_id: "python",
            command: "pyright-langserver",
            args: &["--stdio"],
        }),
        "go" => Some(LspServerSpec {
            server_name: "gopls",
            language_id: "go",
            command: "gopls",
            args: &[],
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

        let rust = server_spec_for_language("rust").expect("rust spec");
        assert_eq!(rust.command, "rust-analyzer");

        let svelte = server_spec_for_language("svelte").expect("svelte spec");
        assert_eq!(svelte.command, "svelte-language-server");
        assert_eq!(svelte.args, &["--stdio"]);

        let python = server_spec_for_language("python").expect("python spec");
        assert_eq!(python.command, "pyright-langserver");
        assert_eq!(python.args, &["--stdio"]);

        let go = server_spec_for_language("go").expect("go spec");
        assert_eq!(go.command, "gopls");
    }

    #[test]
    fn diagnostics_timeout_stays_bounded() {
        assert!(LSP_DIAGNOSTICS_TIMEOUT < LSP_REQUEST_TIMEOUT);
        assert!(LSP_DIAGNOSTICS_TIMEOUT <= Duration::from_millis(1500));
    }

    #[test]
    fn lsp_client_capabilities_enable_editor_diagnostics() {
        let capabilities = lsp_client_capabilities();
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("publishDiagnostics"))
                .and_then(|publish_diagnostics| publish_diagnostics.get("relatedInformation"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("definition"))
                .and_then(|definition| definition.get("linkSupport"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("implementation"))
                .and_then(|implementation| implementation.get("linkSupport"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("typeDefinition"))
                .and_then(|type_definition| type_definition.get("linkSupport"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("completion"))
                .and_then(|completion| completion.get("completionItem"))
                .and_then(|completion_item| completion_item.get("snippetSupport"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("codeAction"))
                .and_then(|code_action| code_action.get("isPreferredSupport"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("signatureHelp"))
                .and_then(|signature_help| signature_help.get("signatureInformation"))
                .and_then(|signature_information| signature_information.get("parameterInformation"))
                .and_then(|parameter_information| parameter_information.get("labelOffsetSupport"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("inlayHint"))
                .and_then(|inlay_hint| inlay_hint.get("dynamicRegistration"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("semanticTokens"))
                .and_then(|semantic_tokens| semantic_tokens.get("requests"))
                .and_then(|requests| requests.get("full"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            capabilities
                .get("textDocument")
                .and_then(|text_document| text_document.get("semanticTokens"))
                .and_then(|semantic_tokens| semantic_tokens.get("formats"))
                .and_then(Value::as_array)
                .and_then(|formats| formats.first())
                .and_then(Value::as_str),
            Some("relative")
        );
    }

    #[test]
    fn extracts_text_edits_from_formatting_result() {
        let result = json!([
            {
                "range": {
                    "start": { "line": 1, "character": 2 },
                    "end": { "line": 1, "character": 8 }
                },
                "newText": "formatted"
            }
        ]);

        assert_eq!(
            lsp_text_edits_from_result(&result),
            vec![SourceLspTextEdit {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 9,
                new_text: "formatted".to_string(),
            }]
        );
    }

    #[test]
    fn extracts_workspace_edits_from_rename_result() {
        let root = env::temp_dir().join("mcb-lsp-rename-root");
        let source_path = root.join("src/App.ts");
        let other_path = root.join("src/App.test.ts");
        let result = json!({
            "documentChanges": [
                {
                    "textDocument": { "uri": path_to_file_uri(&source_path) },
                    "edits": [
                        {
                            "range": {
                                "start": { "line": 2, "character": 4 },
                                "end": { "line": 2, "character": 12 }
                            },
                            "newText": "nextName"
                        }
                    ]
                },
                {
                    "textDocument": { "uri": path_to_file_uri(&other_path) },
                    "edits": [
                        {
                            "range": {
                                "start": { "line": 4, "character": 6 },
                                "end": { "line": 4, "character": 14 }
                            },
                            "newText": "nextName"
                        }
                    ]
                }
            ]
        });

        assert_eq!(
            lsp_workspace_edit_files_from_result(&result, &root),
            vec![
                SourceLspWorkspaceEditFile {
                    path: source_path.display().to_string(),
                    relative_path: "src/App.ts".to_string(),
                    edits: vec![SourceLspTextEdit {
                        start_line: 3,
                        start_column: 5,
                        end_line: 3,
                        end_column: 13,
                        new_text: "nextName".to_string(),
                    }],
                },
                SourceLspWorkspaceEditFile {
                    path: other_path.display().to_string(),
                    relative_path: "src/App.test.ts".to_string(),
                    edits: vec![SourceLspTextEdit {
                        start_line: 5,
                        start_column: 7,
                        end_line: 5,
                        end_column: 15,
                        new_text: "nextName".to_string(),
                    }],
                },
            ]
        );
    }

    #[test]
    fn extracts_code_actions_with_workspace_edits() {
        let root = env::temp_dir().join("mcb-lsp-code-action-root");
        let source_path = root.join("src/App.ts");
        let result = json!([
            {
                "title": "Add missing import",
                "kind": "quickfix",
                "isPreferred": true,
                "edit": {
                    "changes": {
                        path_to_file_uri(&source_path): [
                            {
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": 0, "character": 0 }
                                },
                                "newText": "import { thing } from './thing';\n"
                            }
                        ]
                    }
                }
            },
            {
                "title": "Organize Imports",
                "kind": "source.organizeImports",
                "command": {
                    "title": "Organize Imports",
                    "command": "typescript.organizeImports"
                }
            }
        ]);

        assert_eq!(
            lsp_code_actions_from_result(&result, &root, 10),
            vec![
                SourceLspCodeAction {
                    title: "Add missing import".to_string(),
                    kind: "quickfix".to_string(),
                    is_preferred: true,
                    disabled_reason: None,
                    files: vec![SourceLspWorkspaceEditFile {
                        path: source_path.display().to_string(),
                        relative_path: "src/App.ts".to_string(),
                        edits: vec![SourceLspTextEdit {
                            start_line: 1,
                            start_column: 1,
                            end_line: 1,
                            end_column: 1,
                            new_text: "import { thing } from './thing';\n".to_string(),
                        }],
                    }],
                },
                SourceLspCodeAction {
                    title: "Organize Imports".to_string(),
                    kind: "source.organizeImports".to_string(),
                    is_preferred: false,
                    disabled_reason: Some(
                        "Command-only code action is not supported yet".to_string()
                    ),
                    files: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn extracts_document_highlights() {
        let result = json!([
            {
                "range": {
                    "start": { "line": 4, "character": 8 },
                    "end": { "line": 4, "character": 14 }
                },
                "kind": 2
            },
            {
                "range": {
                    "start": { "line": 8, "character": 2 },
                    "end": { "line": 8, "character": 8 }
                },
                "kind": 3
            }
        ]);

        assert_eq!(
            lsp_document_highlights_from_result(&result),
            vec![
                SourceLspDocumentHighlight {
                    start_line: 5,
                    start_column: 9,
                    end_line: 5,
                    end_column: 15,
                    kind: "read".to_string(),
                },
                SourceLspDocumentHighlight {
                    start_line: 9,
                    start_column: 3,
                    end_line: 9,
                    end_column: 9,
                    kind: "write".to_string(),
                },
            ]
        );
    }

    #[test]
    fn extracts_signature_help() {
        let result = json!({
            "activeSignature": 0,
            "activeParameter": 1,
            "signatures": [
                {
                    "label": "Format(value: string, uppercase: bool)",
                    "documentation": { "kind": "markdown", "value": "Formats a value." },
                    "parameters": [
                        { "label": "value: string", "documentation": "Input value." },
                        { "label": "uppercase: bool" }
                    ]
                }
            ]
        });

        assert_eq!(
            lsp_signature_help_from_result(&result),
            Some(SourceLspSignatureHelp {
                active_signature: 0,
                active_parameter: 1,
                signatures: vec![SourceLspSignature {
                    label: "Format(value: string, uppercase: bool)".to_string(),
                    documentation: "Formats a value.".to_string(),
                    parameters: vec![
                        SourceLspSignatureParameter {
                            label: "value: string".to_string(),
                            documentation: "Input value.".to_string(),
                        },
                        SourceLspSignatureParameter {
                            label: "uppercase: bool".to_string(),
                            documentation: String::new(),
                        },
                    ],
                }],
            })
        );
    }

    #[test]
    fn extracts_inlay_hints() {
        let result = json!([
            {
                "position": { "line": 8, "character": 16 },
                "label": ": string",
                "kind": 1,
                "tooltip": { "kind": "markdown", "value": "Inferred type" },
                "paddingLeft": true
            },
            {
                "position": { "line": 10, "character": 22 },
                "label": [
                    { "value": "value", "tooltip": "Parameter name" },
                    { "value": ": " }
                ],
                "kind": 2,
                "paddingRight": true
            }
        ]);

        assert_eq!(
            lsp_inlay_hints_from_result(&result, 10),
            vec![
                SourceLspInlayHint {
                    label: ": string".to_string(),
                    tooltip: "Inferred type".to_string(),
                    kind: "type".to_string(),
                    line: 9,
                    column: 17,
                    padding_left: true,
                    padding_right: false,
                },
                SourceLspInlayHint {
                    label: "value: ".to_string(),
                    tooltip: "Parameter name".to_string(),
                    kind: "parameter".to_string(),
                    line: 11,
                    column: 23,
                    padding_left: false,
                    padding_right: true,
                },
            ]
        );
    }

    #[test]
    fn extracts_semantic_token_legend_from_initialize() {
        let response = json!({
            "result": {
                "capabilities": {
                    "semanticTokensProvider": {
                        "legend": {
                            "tokenTypes": ["namespace", "class", "property", "modifier"],
                            "tokenModifiers": ["static"]
                        }
                    }
                }
            }
        });

        assert_eq!(
            lsp_semantic_token_types_from_initialize(&response),
            vec![
                "namespace".to_string(),
                "class".to_string(),
                "property".to_string(),
                "modifier".to_string(),
            ]
        );
    }

    #[test]
    fn extracts_semantic_tokens_from_relative_data() {
        let legend = vec![
            "namespace".to_string(),
            "class".to_string(),
            "property".to_string(),
            "method".to_string(),
            "modifier".to_string(),
            "struct".to_string(),
            "local".to_string(),
            "unmapped".to_string(),
        ];
        let result = json!({
            "data": [
                0, 0, 5, 0, 0,
                0, 10, 6, 1, 0,
                2, 4, 8, 2, 0,
                0, 12, 6, 3, 0,
                1, 2, 7, 4, 0,
                1, 1, 6, 5, 0,
                0, 8, 5, 6, 0,
                0, 8, 4, 99, 0,
                0, 12, 4, 7, 0
            ]
        });

        assert_eq!(
            lsp_semantic_tokens_from_result(&result, &legend, 10),
            vec![
                SourceLspSemanticToken {
                    token_type: "namespace".to_string(),
                    line: 1,
                    start_column: 1,
                    length: 5,
                },
                SourceLspSemanticToken {
                    token_type: "class".to_string(),
                    line: 1,
                    start_column: 11,
                    length: 6,
                },
                SourceLspSemanticToken {
                    token_type: "property".to_string(),
                    line: 3,
                    start_column: 5,
                    length: 8,
                },
                SourceLspSemanticToken {
                    token_type: "method".to_string(),
                    line: 3,
                    start_column: 17,
                    length: 6,
                },
                SourceLspSemanticToken {
                    token_type: "keyword".to_string(),
                    line: 4,
                    start_column: 3,
                    length: 7,
                },
                SourceLspSemanticToken {
                    token_type: "class".to_string(),
                    line: 5,
                    start_column: 2,
                    length: 6,
                },
                SourceLspSemanticToken {
                    token_type: "variable".to_string(),
                    line: 5,
                    start_column: 10,
                    length: 5,
                },
            ]
        );
    }

    #[test]
    fn limits_semantic_tokens_after_normalization() {
        let legend = vec!["class".to_string()];
        let result = json!({
            "data": [
                0, 0, 5, 0, 0,
                1, 2, 6, 0, 0,
                1, 2, 7, 0, 0
            ]
        });

        assert_eq!(
            lsp_semantic_tokens_from_result(&result, &legend, 2),
            vec![
                SourceLspSemanticToken {
                    token_type: "class".to_string(),
                    line: 1,
                    start_column: 1,
                    length: 5,
                },
                SourceLspSemanticToken {
                    token_type: "class".to_string(),
                    line: 2,
                    start_column: 3,
                    length: 6,
                },
            ]
        );
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
    fn extracts_pull_diagnostics() {
        let result = json!({
            "kind": "full",
            "items": [
                {
                    "range": {
                        "start": { "line": 2, "character": 12 },
                        "end": { "line": 2, "character": 20 }
                    },
                    "severity": 2,
                    "source": "typescript",
                    "message": "Type mismatch."
                }
            ]
        });

        assert_eq!(
            diagnostics_from_pull_result(&result),
            vec![SourceLspDiagnostic {
                severity: "warning".to_string(),
                message: "Type mismatch.".to_string(),
                line: 3,
                column: 13,
                source: Some("typescript".to_string()),
            }]
        );
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
    fn extracts_workspace_symbols_from_locations() {
        let root = env::temp_dir().join("mcb-workspace-symbol-root");
        let source_path = root.join("src/FormatResolver.cs");
        let result = json!([
            {
                "name": "FormatResolver",
                "kind": 5,
                "containerName": "EdiPlatform.Core.Services",
                "location": {
                    "uri": path_to_file_uri(&source_path),
                    "range": {
                        "start": { "line": 7, "character": 20 },
                        "end": { "line": 7, "character": 34 }
                    }
                }
            }
        ]);

        assert_eq!(
            lsp_workspace_symbols_from_result(&result, &root, "csharp", 20),
            vec![SourceLspWorkspaceSymbol {
                path: source_path.display().to_string(),
                relative_path: "src/FormatResolver.cs".to_string(),
                file_name: "FormatResolver.cs".to_string(),
                language: "csharp".to_string(),
                byte_count: 0,
                symbol_name: "FormatResolver".to_string(),
                kind: "class".to_string(),
                line: 8,
                column: 21,
                detail: "EdiPlatform.Core.Services - src/FormatResolver.cs:8:21".to_string(),
                container_name: Some("EdiPlatform.Core.Services".to_string()),
            }]
        );
    }

    #[test]
    fn extracts_completion_items_from_list_and_array_results() {
        let list_result = json!({
            "isIncomplete": false,
            "items": [
                {
                    "label": "Format",
                    "kind": 2,
                    "detail": "string Format(string value)",
                    "insertText": "Format"
                },
                {
                    "label": "Widget",
                    "kind": 7
                }
            ]
        });
        assert_eq!(
            lsp_completion_items_from_result(&list_result, 20),
            vec![
                SourceLspCompletionItem {
                    label: "Format".to_string(),
                    kind: "method".to_string(),
                    detail: "string Format(string value)".to_string(),
                    insert_text: "Format".to_string(),
                },
                SourceLspCompletionItem {
                    label: "Widget".to_string(),
                    kind: "class".to_string(),
                    detail: String::new(),
                    insert_text: "Widget".to_string(),
                }
            ]
        );

        let array_result = json!([
            {
                "label": "Console",
                "kind": 7,
                "detail": "class Console"
            }
        ]);
        assert_eq!(
            lsp_completion_items_from_result(&array_result, 1),
            vec![SourceLspCompletionItem {
                label: "Console".to_string(),
                kind: "class".to_string(),
                detail: "class Console".to_string(),
                insert_text: "Console".to_string(),
            }]
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
    fn typescript_language_server_smoke_reads_intelligence_actions() {
        if resolve_server_for_language("typescript").is_none() {
            eprintln!("skipping TypeScript LSP smoke: typescript-language-server not found");
            return;
        }

        let root = unique_lsp_temp_root("mcb-ts-lsp-smoke");
        std::fs::write(
            root.join("tsconfig.json"),
            r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"ESNext"}}"#,
        )
        .unwrap();
        let content = [
            "export function greet(name: string): string {",
            "  return `Hello ${name}`;",
            "}",
            "",
            "const value = greet(\"Mac\");",
            "const broken: number = \"oops\";",
        ]
        .join("\n");
        let file_path = root.join("App.ts");
        std::fs::write(&file_path, &content).unwrap();

        let preview = SourceLspPreview {
            path: file_path.display().to_string(),
            relative_path: "App.ts".to_string(),
            file_name: "App.ts".to_string(),
            language: "typescript".to_string(),
            byte_count: content.len() as u64,
            content,
            line_count: 6,
        };
        let request = SourceLspLookupRequest {
            root: root.display().to_string(),
            line: 5,
            column: 16,
            limit: Some(20),
        };
        let registry = SourceLspRegistry::default();

        let symbols = registry
            .find_symbols(preview.clone(), request.clone())
            .expect("document symbols");
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "greet" && symbol.kind == "function"),
            "expected TypeScript document symbols to include greet; got {symbols:?}"
        );

        let hover = registry
            .find_hover(preview.clone(), request.clone())
            .expect("hover")
            .expect("hover contents");
        assert!(
            hover.contents.join("\n").contains("greet"),
            "expected hover to describe greet; got {hover:?}"
        );

        let references = registry
            .find_references(preview.clone(), request.clone())
            .expect("references");
        assert!(
            references
                .iter()
                .any(|target| target.path == file_path.display().to_string() && target.line == 1),
            "expected references to include greet declaration; got {references:?}"
        );
        assert!(
            references
                .iter()
                .any(|target| target.path == file_path.display().to_string() && target.line == 5),
            "expected references to include greet call site; got {references:?}"
        );

        let definitions = registry
            .find_definitions(preview.clone(), request.clone())
            .expect("definitions");
        assert!(
            definitions
                .iter()
                .any(|target| target.path == file_path.display().to_string() && target.line == 1),
            "expected definition to resolve to App.ts line 1; got {definitions:?}"
        );

        let diagnostics = registry
            .read_diagnostics(preview, request)
            .expect("diagnostics");
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == "error"
                    && diagnostic.message.contains("string")
                    && diagnostic.message.contains("number")
            }),
            "expected TypeScript diagnostics to include the broken number assignment; got {diagnostics:?}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn csharp_language_server_smoke_reads_intelligence_actions() {
        if env::var_os("MCB_RUN_CSHARP_LSP_SMOKE").is_none() {
            eprintln!("skipping C# LSP smoke: set MCB_RUN_CSHARP_LSP_SMOKE=1 to enable");
            return;
        }
        if resolve_server_for_language("csharp").is_none() {
            eprintln!("skipping C# LSP smoke: csharp-ls not found");
            return;
        }

        let root = unique_lsp_temp_root("mcb-cs-lsp-smoke");
        let outcome = run_csharp_language_server_smoke(&root);
        let _ = std::fs::remove_dir_all(root);
        outcome.expect("C# LSP smoke");
    }

    fn run_csharp_language_server_smoke(root: &Path) -> Result<(), String> {
        std::fs::write(
            root.join("Smoke.csproj"),
            [
                r#"<Project Sdk="Microsoft.NET.Sdk">"#,
                "  <PropertyGroup>",
                "    <TargetFramework>net9.0</TargetFramework>",
                "    <Nullable>enable</Nullable>",
                "    <ImplicitUsings>enable</ImplicitUsings>",
                "  </PropertyGroup>",
                "</Project>",
            ]
            .join("\n"),
        )
        .map_err(|error| format!("write csproj: {error}"))?;

        let content = [
            "namespace Smoke;",
            "",
            "public sealed class Widget",
            "{",
            "    public string Format(string value) => value.ToUpperInvariant();",
            "}",
            "",
            "public sealed class Runner",
            "{",
            "    public string Run()",
            "    {",
            "        var widget = new Widget();",
            "        int broken = \"oops\";",
            "        return widget.Format(\"mac\");",
            "    }",
            "}",
        ]
        .join("\n");
        let file_path = root.join("Widget.cs");
        std::fs::write(&file_path, &content).map_err(|error| format!("write source: {error}"))?;

        let restore = Command::new("dotnet")
            .arg("restore")
            .arg("--nologo")
            .current_dir(root)
            .output()
            .map_err(|error| format!("dotnet restore failed to start: {error}"))?;
        if !restore.status.success() {
            return Err(format!(
                "dotnet restore failed: {}",
                String::from_utf8_lossy(&restore.stderr)
            ));
        }

        let status = read_source_lsp_status_sync(root.to_path_buf(), "csharp".to_string())?;
        if !status.available {
            return Err(format!("C# LSP status unavailable: {:?}", status.reason));
        }

        let preview = SourceLspPreview {
            path: file_path.display().to_string(),
            relative_path: "Widget.cs".to_string(),
            file_name: "Widget.cs".to_string(),
            language: "csharp".to_string(),
            byte_count: content.len() as u64,
            content,
            line_count: 16,
        };
        let request = SourceLspLookupRequest {
            root: root.display().to_string(),
            line: 12,
            column: 28,
            limit: Some(20),
        };
        let registry = SourceLspRegistry::default();

        let symbols = registry.find_symbols(preview.clone(), request.clone())?;
        if !symbols
            .iter()
            .any(|symbol| symbol.name == "Widget" && symbol.kind == "class")
        {
            return Err(format!(
                "expected C# document symbols to include Widget; got {symbols:?}"
            ));
        }

        let hover = registry
            .find_hover(preview.clone(), request.clone())?
            .ok_or_else(|| "expected C# hover contents".to_string())?;
        if !hover.contents.join("\n").contains("Widget") {
            return Err(format!(
                "expected C# hover to describe Widget; got {hover:?}"
            ));
        }

        let definitions = registry.find_definitions(preview.clone(), request.clone())?;
        if !definitions
            .iter()
            .any(|target| target.path == file_path.display().to_string() && target.line == 3)
        {
            return Err(format!(
                "expected C# definition to resolve to Widget.cs line 3; got {definitions:?}"
            ));
        }

        let references = registry.find_references(preview.clone(), request.clone())?;
        if !references
            .iter()
            .any(|target| target.path == file_path.display().to_string() && target.line == 12)
        {
            return Err(format!(
                "expected C# references to include Widget call site; got {references:?}"
            ));
        }

        let diagnostics = registry.read_diagnostics(preview, request)?;
        if !diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == "error"
                && diagnostic.message.contains("string")
                && diagnostic.message.contains("int")
        }) {
            return Err(format!(
                "expected C# diagnostics to include the broken int assignment; got {diagnostics:?}"
            ));
        }

        Ok(())
    }

    #[test]
    fn round_trips_file_uri_encoding() {
        let path = PathBuf::from("/tmp/source file #1.cs");
        let uri = path_to_file_uri(&path);
        assert_eq!(uri, "file:///tmp/source%20file%20%231.cs");
        assert_eq!(file_uri_to_path(&uri), Some(path));
    }

    fn unique_lsp_temp_root(prefix: &str) -> PathBuf {
        let mut root = env::temp_dir();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.push(format!("{prefix}-{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}
