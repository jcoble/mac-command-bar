use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Emitter;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 2_000;
const MAX_SOURCE_LIST_LIMIT: usize = 5_000;
const SOURCE_SCAN_PROGRESS_EVENT: &str = "source_scan_progress";
const SOURCE_SCAN_PROGRESS_INTERVAL: usize = 64;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRecord {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceScanResult {
    records: Vec<SourceRecord>,
    limit: usize,
    truncated: bool,
}

#[derive(Clone, Copy)]
struct SourceScanProgressSnapshot {
    visited_entries: usize,
    matched_files: usize,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceScanProgressEvent {
    scan_id: String,
    visited_entries: usize,
    matched_files: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcePreview {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    content: String,
    line_count: usize,
}

#[derive(Clone, Copy)]
enum SourceFileAction {
    Open,
    Reveal,
}

#[derive(Debug, PartialEq, Eq)]
struct SourceFileActionCommand {
    program: String,
    args: Vec<String>,
}

#[derive(Default)]
struct SourceScanRegistry {
    scans: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl SourceScanRegistry {
    fn register(&self, scan_id: &str) -> Arc<AtomicBool> {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.scans
            .lock()
            .expect("source scan registry lock poisoned")
            .insert(scan_id.to_string(), Arc::clone(&cancelled));
        cancelled
    }

    fn cancel(&self, scan_id: &str) -> bool {
        let Some(cancelled) = self
            .scans
            .lock()
            .expect("source scan registry lock poisoned")
            .get(scan_id)
            .cloned()
        else {
            return false;
        };
        cancelled.store(true, Ordering::Relaxed);
        true
    }

    fn unregister(&self, scan_id: &str) {
        self.scans
            .lock()
            .expect("source scan registry lock poisoned")
            .remove(scan_id);
    }
}

struct SourceScanCancellation {
    cancelled: Arc<AtomicBool>,
    progress: Option<Arc<dyn Fn(SourceScanProgressSnapshot) + Send + Sync>>,
}

impl SourceScanCancellation {
    fn new(
        cancelled: Arc<AtomicBool>,
        progress: Option<Arc<dyn Fn(SourceScanProgressSnapshot) + Send + Sync>>,
    ) -> Self {
        Self {
            cancelled,
            progress,
        }
    }

    fn none() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), None)
    }

    #[cfg(test)]
    fn cancelled_for_test() -> Self {
        Self::new(Arc::new(AtomicBool::new(true)), None)
    }

    #[cfg(test)]
    fn active_for_test(
        progress: impl Fn(SourceScanProgressSnapshot) + Send + Sync + 'static,
    ) -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), Some(Arc::new(progress)))
    }

    fn ensure_active(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Source scan cancelled".to_string())
        } else {
            Ok(())
        }
    }

    fn report_progress(&self, progress: SourceScanProgressSnapshot) {
        if let Some(callback) = &self.progress {
            callback(progress);
        }
    }
}

#[derive(Default)]
struct SourceScanWalkProgress {
    visited_entries: usize,
    matched_files: usize,
    next_report_at: usize,
}

impl SourceScanWalkProgress {
    fn visit_entry(&mut self, cancellation: &SourceScanCancellation) {
        self.visited_entries += 1;
        if self.visited_entries == 1 || self.visited_entries >= self.next_report_at {
            self.next_report_at = self.visited_entries + SOURCE_SCAN_PROGRESS_INTERVAL;
            self.report(cancellation);
        }
    }

    fn match_file(&mut self, cancellation: &SourceScanCancellation) {
        self.matched_files += 1;
        self.report(cancellation);
    }

    fn report(&self, cancellation: &SourceScanCancellation) {
        cancellation.report_progress(SourceScanProgressSnapshot {
            visited_entries: self.visited_entries,
            matched_files: self.matched_files,
        });
    }
}

#[tauri::command]
async fn list_source_files(
    app: tauri::AppHandle,
    scan_registry: tauri::State<'_, SourceScanRegistry>,
    root: String,
    limit: Option<usize>,
    query: Option<String>,
    scan_id: Option<String>,
) -> Result<SourceScanResult, String> {
    let cancellation = source_scan_cancellation_for_command(&app, &scan_registry, scan_id.as_deref());
    let scan_id_for_cleanup = scan_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        list_source_files_sync_with_cancellation(
            PathBuf::from(root),
            limit.unwrap_or(DEFAULT_SOURCE_LIST_LIMIT),
            query,
            cancellation,
        )
    })
    .await
    .map_err(|error| format!("Source scan task failed: {error}"))?;

    if let Some(scan_id) = &scan_id_for_cleanup {
        scan_registry.unregister(scan_id);
    }

    result
}

#[tauri::command]
async fn cancel_source_scan(
    scan_registry: tauri::State<'_, SourceScanRegistry>,
    scan_id: String,
) -> Result<bool, String> {
    Ok(scan_registry.cancel(&scan_id))
}

fn source_scan_cancellation_for_command(
    app: &tauri::AppHandle,
    scan_registry: &SourceScanRegistry,
    scan_id: Option<&str>,
) -> SourceScanCancellation {
    let Some(scan_id) = scan_id.filter(|value| !value.trim().is_empty()) else {
        return SourceScanCancellation::none();
    };

    let cancelled = scan_registry.register(scan_id);
    let app = app.clone();
    let scan_id = scan_id.to_string();
    let progress = Arc::new(move |snapshot: SourceScanProgressSnapshot| {
        let _ = app.emit(
            SOURCE_SCAN_PROGRESS_EVENT,
            SourceScanProgressEvent {
                scan_id: scan_id.clone(),
                visited_entries: snapshot.visited_entries,
                matched_files: snapshot.matched_files,
            },
        );
    });

    SourceScanCancellation::new(cancelled, Some(progress))
}

#[tauri::command]
async fn read_source_file(path: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || read_source_file_sync(PathBuf::from(path)))
        .await
        .map_err(|error| format!("Source preview task failed: {error}"))?
}

#[tauri::command]
async fn write_source_file(path: String, content: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        write_source_file_sync(PathBuf::from(path), content)
    })
    .await
    .map_err(|error| format!("Source write task failed: {error}"))?
}

#[tauri::command]
async fn open_source_file(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_source_file_action(PathBuf::from(path), SourceFileAction::Open)
    })
    .await
    .map_err(|error| format!("Source open task failed: {error}"))?
}

#[tauri::command]
async fn reveal_source_file(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_source_file_action(PathBuf::from(path), SourceFileAction::Reveal)
    })
    .await
    .map_err(|error| format!("Source reveal task failed: {error}"))?
}

fn list_source_files_sync(
    root: PathBuf,
    limit: usize,
    query: Option<String>,
) -> Result<SourceScanResult, String> {
    list_source_files_sync_with_cancellation(root, limit, query, SourceScanCancellation::none())
}

fn list_source_files_sync_with_cancellation(
    root: PathBuf,
    limit: usize,
    query: Option<String>,
    cancellation: SourceScanCancellation,
) -> Result<SourceScanResult, String> {
    cancellation.ensure_active()?;
    let metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read source root metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Source root is not a directory".to_string());
    }

    let limit = if limit == 0 {
        DEFAULT_SOURCE_LIST_LIMIT
    } else {
        limit.min(MAX_SOURCE_LIST_LIMIT)
    };
    let normalized_query = query
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let collect_limit = limit.saturating_add(1);
    let mut records = Vec::new();
    let mut progress = SourceScanWalkProgress::default();
    collect_source_files(
        &root,
        &root,
        collect_limit,
        normalized_query.as_deref(),
        &mut records,
        &cancellation,
        &mut progress,
    )?;
    progress.report(&cancellation);
    cancellation.ensure_active()?;
    records.sort_by(|left, right| {
        left.relative_path
            .to_lowercase()
            .cmp(&right.relative_path.to_lowercase())
    });
    let truncated = records.len() > limit;
    records.truncate(limit);
    #[cfg(debug_assertions)]
    eprintln!(
        "mcb tauri source.list root={} count={} truncated={}",
        root.display(),
        records.len(),
        truncated
    );
    Ok(SourceScanResult {
        records,
        limit,
        truncated,
    })
}

fn collect_source_files(
    root: &Path,
    current: &Path,
    limit: usize,
    query: Option<&str>,
    records: &mut Vec<SourceRecord>,
    cancellation: &SourceScanCancellation,
    progress: &mut SourceScanWalkProgress,
) -> Result<(), String> {
    cancellation.ensure_active()?;
    if records.len() >= limit {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(current)
        .map_err(|error| format!("Could not read source directory: {error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        cancellation.ensure_active()?;
        progress.visit_entry(cancellation);

        if records.len() >= limit {
            break;
        }

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            collect_source_files(root, &path, limit, query, records, cancellation, progress)?;
            continue;
        }

        if !metadata.is_file() || !is_source_file(&path) {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if !source_file_matches_query(&relative_path, &file_name, query) {
            continue;
        }

        records.push(SourceRecord {
            path: path.display().to_string(),
            relative_path,
            file_name,
            language: detect_language(&path),
            byte_count: metadata.len(),
        });
        progress.match_file(cancellation);
    }

    Ok(())
}

fn read_source_file_sync(path: PathBuf) -> Result<SourcePreview, String> {
    let path_ref = path.as_path();
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(format!(
            "Source file is too large: {} bytes",
            metadata.len()
        ));
    }

    let content = std::fs::read_to_string(path_ref)
        .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();

    let preview = SourcePreview {
        path: path_ref.display().to_string(),
        relative_path: file_name.clone(),
        file_name,
        language: detect_language(path_ref),
        byte_count: metadata.len(),
        line_count: content.lines().count(),
        content,
    };
    #[cfg(debug_assertions)]
    eprintln!(
        "mcb tauri source.preview path={} language={} lines={}",
        preview.path, preview.language, preview.line_count
    );
    Ok(preview)
}

fn write_source_file_sync(path: PathBuf, content: String) -> Result<SourcePreview, String> {
    let path_ref = path.as_path();
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if !is_source_file(path_ref) {
        return Err("Source path is not a supported source file".to_string());
    }

    let byte_count = content.as_bytes().len() as u64;
    if byte_count > MAX_PREVIEW_BYTES {
        return Err(format!("Source content is too large: {byte_count} bytes"));
    }

    let parent = path_ref
        .parent()
        .ok_or_else(|| "Source path has no parent directory".to_string())?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(
        ".{file_name}.mcb-write-{}-{unique}",
        std::process::id()
    ));

    std::fs::write(&temp_path, content.as_bytes())
        .map_err(|error| format!("Could not write source temp file: {error}"))?;
    if let Err(error) = std::fs::rename(&temp_path, path_ref) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Could not replace source file: {error}"));
    }

    read_source_file_sync(path)
}

fn run_source_file_action(path: PathBuf, action: SourceFileAction) -> Result<(), String> {
    let command = source_file_action_command(&path, action)?;
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|error| format!("Could not run source file action: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Source file action exited with {status}"))
    }
}

fn source_file_action_command(
    path: &Path,
    action: SourceFileAction,
) -> Result<SourceFileActionCommand, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }

    let path_arg = path.display().to_string();
    let args = match action {
        SourceFileAction::Open => vec![path_arg],
        SourceFileAction::Reveal => vec!["-R".to_string(), path_arg],
    };

    Ok(SourceFileActionCommand {
        program: "open".to_string(),
        args,
    })
}

fn detect_language(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if file_name == "dockerfile" || file_name.ends_with(".dockerfile") {
        return "dockerfile".to_string();
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match extension.as_str() {
        "cs" => "csharp",
        "csproj" | "fsproj" | "vbproj" | "props" | "targets" | "xaml" | "xml" => "xml",
        "swift" => "swift",
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "svelte" => "svelte",
        "astro" | "html" | "htm" | "vue" => "html",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "json" | "jsonc" => "json",
        "md" | "markdown" => "markdown",
        "mdx" => "mdx",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "ini" | "env" => "ini",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" | "psm1" => "powershell",
        "py" => "python",
        "rb" => "ruby",
        "php" => "php",
        "dart" => "dart",
        "lua" => "lua",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx" => "cpp",
        "tf" | "tfvars" => "hcl",
        "proto" => "protobuf",
        "sql" => "sql",
        "graphql" | "gql" => "graphql",
        "fs" | "fsx" => "fsharp",
        "razor" => "razor",
        _ => "plain",
    }
    .to_string()
}

fn is_source_file(path: &Path) -> bool {
    !matches!(detect_language(path).as_str(), "plain")
}

fn should_skip_dir(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    if normalized.ends_with("_files") {
        return true;
    }

    matches!(
        normalized.as_str(),
        "__pycache__"
            | ".cache"
            | ".git"
            | ".hg"
            | ".svn"
            | ".agents"
            | ".build"
            | ".claude"
            | ".codex"
            | ".gradle"
            | ".next"
            | ".nuxt"
            | ".parcel-cache"
            | ".svelte-kit"
            | ".turbo"
            | ".vite"
            | "bin"
            | "build"
            | "coverage"
            | "deriveddata"
            | "dist"
            | "node_modules"
            | "obj"
            | "pods"
            | "target"
            | "testresults"
            | "vendor"
            | "worktrees"
    )
}

fn source_file_matches_query(relative_path: &str, file_name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    relative_path.to_lowercase().contains(query) || file_name.to_lowercase().contains(query)
}

fn main() {
    tauri::Builder::default()
        .manage(SourceScanRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_source_files,
            cancel_source_scan,
            read_source_file,
            write_source_file,
            open_source_file,
            reveal_source_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run MacCommandBar webview preview");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn source_scan_groups_supported_files_and_skips_build_dirs() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("packages/ui")).unwrap();
        std::fs::create_dir_all(root.join("src/Workers")).unwrap();
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::write(root.join("packages/ui/Button.tsx"), "export function Button() {}").unwrap();
        std::fs::write(root.join("Package.swift"), "let package = 1").unwrap();
        std::fs::write(root.join("src/App.svelte"), "<script></script>").unwrap();
        std::fs::write(root.join("src/settings.json"), "{}").unwrap();
        std::fs::write(root.join("src/Workers/Worker.cs"), "public class Worker {}").unwrap();
        std::fs::write(root.join("target/debug/generated.rs"), "fn generated() {}").unwrap();
        std::fs::write(root.join("README.md"), "# docs").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, None).unwrap();
        let relative_paths = scan.records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec![
                "Package.swift",
                "packages/ui/Button.tsx",
                "README.md",
                "src/App.svelte",
                "src/settings.json",
                "src/Workers/Worker.cs"
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_skips_tool_cache_and_coverage_dirs() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".cache/generated")).unwrap();
        std::fs::create_dir_all(root.join(".turbo/cache")).unwrap();
        std::fs::create_dir_all(root.join(".parcel-cache")).unwrap();
        std::fs::create_dir_all(root.join(".nuxt")).unwrap();
        std::fs::create_dir_all(root.join(".vite/deps")).unwrap();
        std::fs::create_dir_all(root.join("coverage/lcov-report")).unwrap();
        std::fs::create_dir_all(root.join("DerivedData/Build")).unwrap();
        std::fs::create_dir_all(root.join("TestResults/run")).unwrap();
        std::fs::create_dir_all(root.join("Pods/SomeDependency")).unwrap();
        std::fs::write(root.join("src/Keep.ts"), "export const keep = true;").unwrap();
        std::fs::write(root.join(".cache/generated/Cache.ts"), "export const cache = true;").unwrap();
        std::fs::write(root.join(".turbo/cache/Turbo.ts"), "export const turbo = true;").unwrap();
        std::fs::write(root.join(".parcel-cache/Parcel.ts"), "export const parcel = true;").unwrap();
        std::fs::write(root.join(".nuxt/App.vue"), "<template></template>").unwrap();
        std::fs::write(root.join(".vite/deps/Vite.ts"), "export const vite = true;").unwrap();
        std::fs::write(root.join("coverage/lcov-report/Coverage.ts"), "export const covered = true;").unwrap();
        std::fs::write(root.join("DerivedData/Build/Generated.swift"), "let generated = true").unwrap();
        std::fs::write(root.join("TestResults/run/TestLog.cs"), "public class TestLog {}").unwrap();
        std::fs::write(root.join("Pods/SomeDependency/Dependency.swift"), "let dependency = true").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, None).unwrap();
        let relative_paths = scan.records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(relative_paths, vec!["src/Keep.ts"]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_language_detection_covers_common_project_files() {
        assert_eq!(detect_language(Path::new("Program.cs")), "csharp");
        assert_eq!(detect_language(Path::new("Package.swift")), "swift");
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("src/routes/+page.svelte")), "svelte");
        assert_eq!(detect_language(Path::new("src/main.tsx")), "tsx");
        assert_eq!(detect_language(Path::new("src/app.jsx")), "jsx");
        assert_eq!(detect_language(Path::new("README.md")), "markdown");
        assert_eq!(detect_language(Path::new("package.json")), "json");
        assert_eq!(detect_language(Path::new("pnpm-lock.yaml")), "yaml");
        assert_eq!(detect_language(Path::new("Cargo.toml")), "toml");
        assert_eq!(detect_language(Path::new("scripts/build.sh")), "shell");
        assert_eq!(detect_language(Path::new("tools/import.py")), "python");
        assert_eq!(detect_language(Path::new("Dockerfile")), "dockerfile");
        assert_eq!(detect_language(Path::new("EdiPlatform.Api.csproj")), "xml");
    }

    #[test]
    fn source_language_detection_covers_more_preview_formats() {
        assert_eq!(detect_language(Path::new("web/App.vue")), "html");
        assert_eq!(detect_language(Path::new("web/Page.astro")), "html");
        assert_eq!(detect_language(Path::new("index.php")), "php");
        assert_eq!(detect_language(Path::new("lib/main.dart")), "dart");
        assert_eq!(detect_language(Path::new("scripts/tool.lua")), "lua");
        assert_eq!(detect_language(Path::new("infra/main.tf")), "hcl");
        assert_eq!(detect_language(Path::new("infra/dev.tfvars")), "hcl");
        assert_eq!(detect_language(Path::new("schemas/service.proto")), "protobuf");
    }

    #[test]
    fn source_scan_filters_by_query() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src/Workers")).unwrap();
        std::fs::write(root.join("src/App.svelte"), "<script></script>").unwrap();
        std::fs::write(root.join("src/Workers/Worker.cs"), "public class Worker {}").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, Some("worker".to_string())).unwrap();

        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.records[0].relative_path, "src/Workers/Worker.cs");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_default_limit_covers_large_project_trees() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..350 {
            std::fs::write(root.join(format!("src/File{index:03}.ts")), "export const value = 1;")
                .unwrap();
        }

        let scan = list_source_files_sync(root.clone(), 0, None).unwrap();

        assert_eq!(scan.records.len(), 350);
        assert!(!scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_allows_explicit_limits_above_the_default_preview_cap() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..1_200 {
            std::fs::write(root.join(format!("src/File{index:04}.ts")), "export const value = 1;")
                .unwrap();
        }

        let scan = list_source_files_sync(root.clone(), 1_200, None).unwrap();

        assert_eq!(scan.records.len(), 1_200);
        assert!(!scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_reports_when_the_limit_was_reached() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();
        std::fs::write(root.join("src/B.ts"), "export const b = 1;").unwrap();

        let scan = list_source_files_sync(root.clone(), 1, None).unwrap();

        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.limit, 1);
        assert!(scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_can_be_cancelled_before_collection() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();

        let cancellation = SourceScanCancellation::cancelled_for_test();
        let error = list_source_files_sync_with_cancellation(root.clone(), 20, None, cancellation)
            .unwrap_err();

        assert!(error.contains("Source scan cancelled"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_reports_progress_during_collection() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();
        std::fs::write(root.join("src/B.ts"), "export const b = 1;").unwrap();

        let progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let progress_clone = std::sync::Arc::clone(&progress);
        let cancellation = SourceScanCancellation::active_for_test(move |_| {
            progress_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        let scan = list_source_files_sync_with_cancellation(root.clone(), 20, None, cancellation)
            .unwrap();

        assert_eq!(scan.records.len(), 2);
        assert!(progress.load(std::sync::atomic::Ordering::Relaxed) > 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_write_file_persists_utf8_content_and_returns_preview() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.ts");
        std::fs::write(&file_path, "export const oldValue = 1;\n").unwrap();

        let preview =
            write_source_file_sync(file_path.clone(), "export const newValue = 2;\n".to_string())
                .unwrap();

        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "export const newValue = 2;\n"
        );
        assert_eq!(preview.path, file_path.display().to_string());
        assert_eq!(preview.file_name, "App.ts");
        assert_eq!(preview.language, "typescript");
        assert_eq!(preview.line_count, 1);
        assert_eq!(preview.content, "export const newValue = 2;\n");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_file_action_builds_open_and_reveal_commands() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.svelte");
        std::fs::write(&file_path, "<script></script>").unwrap();

        let open_command = source_file_action_command(&file_path, SourceFileAction::Open).unwrap();
        assert_eq!(open_command.program, "open");
        assert_eq!(open_command.args, vec![file_path.display().to_string()]);

        let reveal_command =
            source_file_action_command(&file_path, SourceFileAction::Reveal).unwrap();
        assert_eq!(reveal_command.program, "open");
        assert_eq!(
            reveal_command.args,
            vec!["-R".to_string(), file_path.display().to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_file_action_rejects_directories() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();

        let error = source_file_action_command(&root, SourceFileAction::Open).unwrap_err();
        assert!(error.contains("Source path is not a file"));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_root() -> PathBuf {
        static TEMP_ROOT_COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mac-command-bar-tauri-source-test-{}-{unique}-{counter}",
            std::process::id()
        ))
    }
}
