use super::super::*;

#[tauri::command]
pub(crate) async fn find_source_lsp_definitions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_definitions(preview, request))
        .await
        .map_err(|error| format!("Source LSP definition task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_completions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspCompletionItem>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_completions(preview, request))
        .await
        .map_err(|error| format!("Source LSP completion task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_implementations(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_implementations(preview, request))
        .await
        .map_err(|error| format!("Source LSP implementation task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_type_definitions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_type_definitions(preview, request))
        .await
        .map_err(|error| format!("Source LSP type-definition task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_document_highlights(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDocumentHighlight>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.find_document_highlights(preview, request)
    })
    .await
    .map_err(|error| format!("Source LSP document-highlight task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_signature_help(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Option<lsp::SourceLspSignatureHelp>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_signature_help(preview, request))
        .await
        .map_err(|error| format!("Source LSP signature-help task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_inlay_hints(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspInlayHint>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_inlay_hints(preview, request))
        .await
        .map_err(|error| format!("Source LSP inlay-hint task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_semantic_tokens(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspSemanticToken>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_semantic_tokens(preview, request))
        .await
        .map_err(|error| format!("Source LSP semantic-token task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_workspace_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspWorkspaceSymbolRequest,
) -> Result<Vec<lsp::SourceLspWorkspaceSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_workspace_symbols(preview, request))
        .await
        .map_err(|error| format!("Source LSP workspace-symbol task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn format_source_with_lsp(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspTextEdit>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.format_document(preview, request))
        .await
        .map_err(|error| format!("Source LSP formatting task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn rename_source_with_lsp(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspRenameRequest,
) -> Result<lsp::SourceLspRenameResult, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.rename(preview, request))
        .await
        .map_err(|error| format!("Source LSP rename task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_code_actions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspCodeActionRequest,
) -> Result<Vec<lsp::SourceLspCodeAction>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_code_actions(preview, request))
        .await
        .map_err(|error| format!("Source LSP code action task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_references(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspReferenceTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_references(preview, request))
        .await
        .map_err(|error| format!("Source LSP reference task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_hover(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Option<lsp::SourceLspHover>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_hover(preview, request))
        .await
        .map_err(|error| format!("Source LSP hover task failed: {error}"))?
}

/// Every symbol in one file, as the editor's margin counts want them: flattened, with
/// both numbers counted from zero.
///
/// This is what switches the margin counts on for Rust and Svelte, where the app's own
/// pattern-based symbol reader has never worked. Unlike the other language-server
/// lookups it is handed a path rather than the editor's copy of the text, and reads the
/// file from disk itself.
#[tauri::command]
pub(crate) async fn find_source_lsp_document_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
    language: String,
    path: String,
) -> Result<Vec<lsp::SourceLspDocumentSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.find_document_symbols(root, language, path)
    })
    .await
    .map_err(|error| format!("Source LSP document symbol task failed: {error}"))?
}

/// The last few hundred lines a running language server printed to its own error output.
///
/// When a server refuses to answer, this is usually the only explanation there is. Empty
/// when no server for that language is running.
#[tauri::command]
pub(crate) async fn read_source_lsp_log(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
    language: String,
) -> Result<Vec<String>, String> {
    // The project folder is accepted but not used: one server per language serves every
    // project, so there is only ever one log to hand back.
    let _ = root;
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.read_server_log(&language))
        .await
        .map_err(|error| format!("Source LSP log task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn find_source_lsp_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_symbols(preview, request))
        .await
        .map_err(|error| format!("Source LSP symbol task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn read_source_lsp_diagnostics(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDiagnostic>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.read_diagnostics(preview, request))
        .await
        .map_err(|error| format!("Source LSP diagnostics task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn list_source_lsp_diagnostics_for_root(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
) -> Result<Vec<lsp::SourceLspDiagnostic>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.list_diagnostics_for_root(PathBuf::from(root))
    })
    .await
    .map_err(|error| format!("Source LSP project diagnostics task failed: {error}"))?
}
