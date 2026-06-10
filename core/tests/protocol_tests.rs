use mcb_core::dispatcher::dispatch;
use mcb_core::protocol::CoreRequest;
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
