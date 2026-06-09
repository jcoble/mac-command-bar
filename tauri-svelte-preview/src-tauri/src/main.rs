use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 300;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRecord {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
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

#[tauri::command]
async fn list_source_files(
    root: String,
    limit: Option<usize>,
    query: Option<String>,
) -> Result<Vec<SourceRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_source_files_sync(
            PathBuf::from(root),
            limit.unwrap_or(DEFAULT_SOURCE_LIST_LIMIT),
            query,
        )
    })
    .await
    .map_err(|error| format!("Source scan task failed: {error}"))?
}

#[tauri::command]
async fn read_source_file(path: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || read_source_file_sync(PathBuf::from(path)))
        .await
        .map_err(|error| format!("Source preview task failed: {error}"))?
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
) -> Result<Vec<SourceRecord>, String> {
    let metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read source root metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Source root is not a directory".to_string());
    }

    let limit = if limit == 0 {
        DEFAULT_SOURCE_LIST_LIMIT
    } else {
        limit.min(1_000)
    };
    let normalized_query = query
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let mut records = Vec::new();
    collect_source_files(
        &root,
        &root,
        limit,
        normalized_query.as_deref(),
        &mut records,
    )?;
    records.sort_by(|left, right| {
        left.relative_path
            .to_lowercase()
            .cmp(&right.relative_path.to_lowercase())
    });
    records.truncate(limit);
    #[cfg(debug_assertions)]
    eprintln!(
        "mcb tauri source.list root={} count={}",
        root.display(),
        records.len()
    );
    Ok(records)
}

fn collect_source_files(
    root: &Path,
    current: &Path,
    limit: usize,
    query: Option<&str>,
    records: &mut Vec<SourceRecord>,
) -> Result<(), String> {
    if records.len() >= limit {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(current)
        .map_err(|error| format!("Could not read source directory: {error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
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
            collect_source_files(root, &path, limit, query, records)?;
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
        "html" | "htm" => "html",
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
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx" => "cpp",
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
    if name.ends_with("_files") {
        return true;
    }

    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".agents"
            | ".build"
            | ".claude"
            | ".codex"
            | ".next"
            | ".svelte-kit"
            | "bin"
            | "build"
            | "dist"
            | "node_modules"
            | "obj"
            | "target"
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_source_files,
            read_source_file,
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

        let files = list_source_files_sync(root.clone(), 20, None).unwrap();
        let relative_paths = files
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
    fn source_scan_filters_by_query() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src/Workers")).unwrap();
        std::fs::write(root.join("src/App.svelte"), "<script></script>").unwrap();
        std::fs::write(root.join("src/Workers/Worker.cs"), "public class Worker {}").unwrap();

        let files = list_source_files_sync(root.clone(), 20, Some("worker".to_string())).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "src/Workers/Worker.cs");

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
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mac-command-bar-tauri-source-test-{}-{unique}",
            std::process::id()
        ))
    }
}
