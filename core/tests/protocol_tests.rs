use mcb_core::dispatcher::dispatch;
use mcb_core::protocol::CoreRequest;
use mcb_core::source::list_source_files;
use serde_json::json;

#[test]
fn unknown_action_returns_error_with_same_id() {
    let response = dispatch(CoreRequest {
        id: "req-unknown".to_string(),
        action: "unknown.action".to_string(),
        dry_run: true,
        payload: json!({}),
    });

    assert_eq!(response.id, "req-unknown");
    assert!(!response.ok);
    assert!(response.summary.contains("unknown"));
}

#[test]
fn core_request_uses_camel_case_wire_keys() {
    let request = CoreRequest {
        id: "req-1".to_string(),
        action: "scan.worktrees".to_string(),
        dry_run: true,
        payload: json!({ "repoPath": "/tmp/repo" }),
    };

    let encoded = serde_json::to_string(&request).unwrap();

    assert!(encoded.contains("\"dryRun\":true"));
    assert!(encoded.contains("\"repoPath\""));
}

#[test]
fn source_preview_reads_csharp_file_and_returns_highlight_spans() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("Example.cs");
    std::fs::write(
        &source_path,
        "public class Example\n{\n    public string Name { get; set; }\n}\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source".to_string(),
        action: "source.preview".to_string(),
        dry_run: true,
        payload: json!({ "path": source_path }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.id, "req-source");
    assert_eq!(response.data["language"], "csharp");
    assert_eq!(response.data["fileName"], "Example.cs");
    assert!(response.data["content"]
        .as_str()
        .unwrap()
        .contains("public class Example"));
    assert_eq!(response.data["lineCount"], 4);

    let spans = response.data["spans"].as_array().unwrap();
    assert!(
        spans.iter().any(|span| span["role"] == "keyword"),
        "expected keyword highlight span, got {spans:?}"
    );
    assert!(
        spans.iter().any(|span| span["role"] == "type"),
        "expected type highlight span, got {spans:?}"
    );
}

#[test]
fn source_preview_returns_richer_csharp_highlight_roles() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("Example.cs");
    std::fs::write(
        &source_path,
        "using System;\nnamespace Demo.App;\n[Fact]\npublic class Example\n{\n    public void Run(string input)\n    {\n        var count = 1 + input.Length;\n        Console.WriteLine(input);\n    }\n}\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-rich".to_string(),
        action: "source.preview".to_string(),
        dry_run: true,
        payload: json!({ "path": source_path }),
    });

    assert!(response.ok, "{response:?}");
    let spans = response.data["spans"].as_array().unwrap();
    let roles: std::collections::BTreeSet<&str> = spans
        .iter()
        .filter_map(|span| span["role"].as_str())
        .collect();

    for expected in [
        "keyword",
        "type",
        "function",
        "parameter",
        "module",
        "attribute",
        "variable",
        "operator",
        "punctuation",
    ] {
        assert!(
            roles.contains(expected),
            "expected role {expected}, got {roles:?}"
        );
    }
}

#[test]
fn source_preview_reads_typescript_file_and_returns_highlight_spans() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("sourcePreview.ts");
    std::fs::write(
        &source_path,
        "import type { SourcePreview } from './sourceData';\n\
         export class PreviewStore {\n\
         \tconstructor(private readonly title: string) {}\n\
         \tload(path: string): SourcePreview {\n\
         \t\treturn { fileName: path, content: `loaded ${path}` } as SourcePreview;\n\
         \t}\n\
         }\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-ts".to_string(),
        action: "source.preview".to_string(),
        dry_run: true,
        payload: json!({ "path": source_path }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["language"], "typescript");

    let spans = response.data["spans"].as_array().unwrap();
    let roles: std::collections::BTreeSet<&str> = spans
        .iter()
        .filter_map(|span| span["role"].as_str())
        .collect();

    for expected in ["keyword", "type", "function", "parameter", "string"] {
        assert!(
            roles.contains(expected),
            "expected role {expected}, got {roles:?}"
        );
    }
}

#[test]
fn source_preview_reads_tsx_file_and_returns_highlight_spans() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("SourcePreview.tsx");
    std::fs::write(
        &source_path,
        "type Props = { title: string };\n\
         export function SourcePreview({ title }: Props) {\n\
         \treturn <section>{title}</section>;\n\
         }\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-tsx".to_string(),
        action: "source.preview".to_string(),
        dry_run: true,
        payload: json!({ "path": source_path }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["language"], "tsx");

    let spans = response.data["spans"].as_array().unwrap();
    let roles: std::collections::BTreeSet<&str> = spans
        .iter()
        .filter_map(|span| span["role"].as_str())
        .collect();

    for expected in ["keyword", "type", "function"] {
        assert!(
            roles.contains(expected),
            "expected role {expected}, got {roles:?}"
        );
    }
}

#[test]
fn source_preview_rejects_missing_path_payload() {
    let response = dispatch(CoreRequest {
        id: "req-source-missing".to_string(),
        action: "source.preview".to_string(),
        dry_run: true,
        payload: json!({}),
    });

    assert!(!response.ok);
    assert!(response.summary.contains("missing path"));
}

#[test]
fn source_list_scans_repo_and_skips_build_dependency_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("obj")).unwrap();
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::write(
        root.join("src/ExternalLogin.cs"),
        "public class ExternalLogin {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/App.svelte"),
        "<script>let count = 0;</script>\n",
    )
    .unwrap();
    std::fs::write(root.join("obj/Generated.cs"), "public class Generated {}\n").unwrap();
    std::fs::write(
        root.join("node_modules/pkg/index.ts"),
        "export const x = 1;\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 20 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 2);

    let files = response.data["files"].as_array().unwrap();
    let relative_paths: Vec<&str> = files
        .iter()
        .map(|file| file["relativePath"].as_str().unwrap())
        .collect();
    assert_eq!(
        relative_paths,
        vec!["src/App.svelte", "src/ExternalLogin.cs"]
    );
    assert!(files.iter().any(|file| file["language"] == "csharp"));
    assert!(files.iter().any(|file| file["language"] == "svelte"));
}

#[test]
fn source_list_skips_agent_and_worktree_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/hooks")).unwrap();
    std::fs::create_dir_all(root.join(".claude/worktrees/agent/EdiPlatform.Data")).unwrap();
    std::fs::create_dir_all(root.join("worktrees/session/EdiPlatform.Engine")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join(".claude/hooks/guard.js"),
        "console.log('guard');\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".claude/worktrees/agent/EdiPlatform.Data/Stale.cs"),
        "public class Stale {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("worktrees/session/EdiPlatform.Engine/Hidden.cs"),
        "public class Hidden {}\n",
    )
    .unwrap();
    std::fs::write(root.join("src/Real.cs"), "public class Real {}\n").unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list-skip-agents".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 20 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1);
    assert_eq!(
        response.data["files"][0]["relativePath"].as_str().unwrap(),
        "src/Real.cs"
    );
}

#[test]
fn source_list_matches_preview_scanner_file_surface_and_tool_skips() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("Docs")).unwrap();
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::create_dir_all(root.join(".history")).unwrap();
    std::fs::create_dir_all(root.join(".pytest_cache")).unwrap();
    std::fs::create_dir_all(root.join(".vscode")).unwrap();
    std::fs::write(root.join("src/App.tsx"), "export const App = () => null;\n").unwrap();
    std::fs::write(root.join("src/config.json"), "{}\n").unwrap();
    std::fs::write(root.join("Docs/README.md"), "# docs\n").unwrap();
    std::fs::write(root.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(root.join("Makefile"), "test:\n\techo ok\n").unwrap();
    std::fs::write(root.join("scripts/run.sh"), "#!/usr/bin/env bash\n").unwrap();
    std::fs::write(root.join(".history/Old.cs"), "public class Old {}\n").unwrap();
    std::fs::write(root.join(".pytest_cache/cache.py"), "print('cache')\n").unwrap();
    std::fs::write(root.join(".vscode/settings.json"), "{}\n").unwrap();

    let listed = list_source_files(root, 40, None).unwrap();
    let relative_paths = listed
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();
    assert!(relative_paths.contains(&"src/App.tsx"));
    assert!(relative_paths.contains(&"src/config.json"));
    assert!(relative_paths.contains(&"Docs/README.md"));
    assert!(relative_paths.contains(&"Dockerfile"));
    assert!(relative_paths.contains(&"Makefile"));
    assert!(relative_paths.contains(&"scripts/run.sh"));
    assert!(!relative_paths
        .iter()
        .any(|path| path.starts_with(".history/")));
    assert!(!relative_paths
        .iter()
        .any(|path| path.starts_with(".pytest_cache/")));
    assert!(!relative_paths
        .iter()
        .any(|path| path.starts_with(".vscode/")));

    let languages = listed
        .files
        .iter()
        .map(|file| file.language.as_str())
        .collect::<Vec<_>>();
    assert!(languages.contains(&"tsx"));
    assert!(languages.contains(&"json"));
    assert!(languages.contains(&"markdown"));
    assert!(languages.contains(&"dockerfile"));
    assert!(languages.contains(&"makefile"));
    assert!(languages.contains(&"shell"));

    let skipped_names = listed
        .diagnostics
        .skipped_directories
        .iter()
        .map(|directory| directory.name.as_str())
        .collect::<Vec<_>>();
    assert!(skipped_names.contains(&".history"));
    assert!(skipped_names.contains(&".pytest_cache"));
    assert!(skipped_names.contains(&".vscode"));
}

#[test]
fn source_list_prioritizes_app_source_before_docs_when_truncated() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("Docs")).unwrap();
    std::fs::create_dir_all(root.join("EdiPlatform.Core/Services")).unwrap();
    std::fs::create_dir_all(root.join("EdiPlatform.Core/Models")).unwrap();
    std::fs::write(root.join("Docs/A.md"), "# docs\n").unwrap();
    std::fs::write(root.join("Docs/B.md"), "# docs\n").unwrap();
    std::fs::write(
        root.join("EdiPlatform.Core/Services/RuntimeService.cs"),
        "namespace Demo;\npublic sealed class RuntimeService {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("EdiPlatform.Core/Models/RuntimeModel.cs"),
        "namespace Demo;\npublic sealed class RuntimeModel {}\n",
    )
    .unwrap();

    let listed = list_source_files(root, 2, None).unwrap();

    assert_eq!(
        listed
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "EdiPlatform.Core/Models/RuntimeModel.cs",
            "EdiPlatform.Core/Services/RuntimeService.cs",
        ]
    );
    assert!(listed.truncated);
}

#[test]
fn source_list_final_sort_keeps_src_before_project_named_shared_dirs_when_truncated() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Project");
    std::fs::create_dir_all(root.join("Project.Shared")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Project.Shared/Alpha.cs"),
        "namespace Project.Shared;\npublic sealed class Alpha {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Project.Shared/Beta.cs"),
        "namespace Project.Shared;\npublic sealed class Beta {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/App.cs"),
        "namespace Project;\npublic sealed class App {}\n",
    )
    .unwrap();

    let listed = list_source_files(&root, 2, None).unwrap();

    assert_eq!(
        listed
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/App.cs", "Project.Shared/Alpha.cs"]
    );
    assert!(listed.truncated);
}

#[test]
fn source_list_low_requested_limit_still_walks_beyond_returned_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::write(root.join("src/App.ts"), "export const app = true;\n").unwrap();
    std::fs::write(root.join("src/Worker.ts"), "export const worker = true;\n").unwrap();
    std::fs::write(root.join("src/Widget.ts"), "export const widget = true;\n").unwrap();
    std::fs::write(
        root.join("node_modules/pkg/index.ts"),
        "export const dependency = true;\n",
    )
    .unwrap();

    let listed = list_source_files(root, 2, None).unwrap();

    assert_eq!(listed.files.len(), 2);
    assert!(listed.truncated);
    assert_eq!(listed.diagnostics.skipped_directory_count, 1);
    assert_eq!(
        listed.diagnostics.skipped_directories[0].name,
        "node_modules"
    );
}

#[test]
fn source_list_reports_diagnostics_for_skipped_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::create_dir_all(root.join("worktrees/session/src")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join(".git/objects/Hidden.ts"),
        "export const hidden = true;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("node_modules/pkg/index.ts"),
        "export const dependency = true;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("worktrees/session/src/Stale.cs"),
        "public class Stale {}\n",
    )
    .unwrap();
    std::fs::write(root.join("src/Real.ts"), "export const real = true;\n").unwrap();
    std::fs::write(root.join("notes.txt"), "plain notes\n").unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list-diagnostics".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 20 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1);
    assert_eq!(response.data["diagnostics"]["effectiveLimit"], 20);
    assert_eq!(response.data["diagnostics"]["returnedCount"], 1);
    assert_eq!(response.data["diagnostics"]["truncated"], false);
    assert_eq!(response.data["diagnostics"]["skippedDirectoryCount"], 3);
    assert_eq!(response.data["diagnostics"]["unsupportedFileCount"], 1);

    let skipped = response.data["diagnostics"]["skippedDirectories"]
        .as_array()
        .unwrap();
    let skipped_names: std::collections::BTreeSet<&str> = skipped
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(
        skipped_names,
        std::collections::BTreeSet::from([".git", "node_modules", "worktrees"])
    );
    assert!(skipped.iter().all(|entry| entry["reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty())));
}

#[test]
fn source_list_skips_saved_web_asset_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("Docs/Research/Guide_files")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Docs/Research/Guide_files/app.js"),
        "console.log('asset');\n",
    )
    .unwrap();
    std::fs::write(root.join("src/Real.cs"), "public class Real {}\n").unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list-skip-web-assets".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 20 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1);
    assert_eq!(
        response.data["files"][0]["relativePath"].as_str().unwrap(),
        "src/Real.cs"
    );
}

#[test]
fn source_list_applies_query_before_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/AFirst.cs"), "public class AFirst {}\n").unwrap();
    std::fs::write(
        root.join("src/TransactionProcessorWorker.cs"),
        "public class TransactionProcessorWorker {}\n",
    )
    .unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list-query".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 1, "query": "ProcessorWorker.cs" }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1);
    assert_eq!(
        response.data["files"][0]["relativePath"].as_str().unwrap(),
        "src/TransactionProcessorWorker.cs"
    );
}

#[test]
fn source_list_honors_default_limit_cap_and_explicit_lower_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/A.ts"), "export const a = 1;\n").unwrap();
    std::fs::write(root.join("src/B.ts"), "export const b = 1;\n").unwrap();

    let defaulted = list_source_files(root, 0, None).unwrap();
    assert_eq!(defaulted.limit, 10_000);
    assert_eq!(defaulted.files.len(), 2);
    assert!(!defaulted.truncated);

    let capped = list_source_files(root, 30_000, None).unwrap();
    assert_eq!(capped.limit, 25_000);
    assert_eq!(capped.files.len(), 2);
    assert!(!capped.truncated);

    let explicit_lower = list_source_files(root, 1, None).unwrap();
    assert_eq!(explicit_lower.limit, 1);
    assert_eq!(explicit_lower.files.len(), 1);
    assert!(explicit_lower.truncated);
}

#[test]
fn source_list_allows_explicit_limits_above_old_preview_cap() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();

    for index in 0..1_200 {
        std::fs::write(
            root.join(format!("src/File{index:04}.ts")),
            "export const value = 1;\n",
        )
        .unwrap();
    }

    let response = dispatch(CoreRequest {
        id: "req-source-list-large".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 1_200 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1_200);
    assert_eq!(response.data["limit"], 1_200);
    assert_eq!(response.data["truncated"], false);
}

#[test]
fn source_list_reports_truncation_when_limit_is_reached() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/A.ts"), "export const a = 1;\n").unwrap();
    std::fs::write(root.join("src/B.ts"), "export const b = 1;\n").unwrap();

    let response = dispatch(CoreRequest {
        id: "req-source-list-truncated".to_string(),
        action: "source.list".to_string(),
        dry_run: true,
        payload: json!({ "rootPath": root, "limit": 1 }),
    });

    assert!(response.ok, "{response:?}");
    assert_eq!(response.data["count"], 1);
    assert_eq!(response.data["limit"], 1);
    assert_eq!(response.data["truncated"], true);
}
