use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 10_000;
const MAX_SOURCE_LIST_LIMIT: usize = 25_000;
const MAX_SKIPPED_DIRECTORY_SAMPLES: usize = 16;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourcePreview {
    pub path: String,
    pub file_name: String,
    pub language: String,
    pub content: String,
    pub spans: Vec<SyntaxSpan>,
    pub line_count: usize,
    pub byte_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileRecord {
    pub path: String,
    pub relative_path: String,
    pub file_name: String,
    pub language: String,
    pub byte_count: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileList {
    pub files: Vec<SourceFileRecord>,
    pub limit: usize,
    pub truncated: bool,
    pub diagnostics: SourceScanDiagnostics,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceScanDiagnostics {
    pub effective_limit: usize,
    pub returned_count: usize,
    pub truncated: bool,
    pub skipped_directory_count: usize,
    pub unsupported_file_count: usize,
    pub unreadable_entry_count: usize,
    pub skipped_directories: Vec<SourceSkippedDirectory>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSkippedDirectory {
    pub path: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Default)]
struct SourceScanStats {
    skipped_directory_count: usize,
    unsupported_file_count: usize,
    unreadable_entry_count: usize,
    skipped_directories: Vec<SourceSkippedDirectory>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxSpan {
    pub start: usize,
    pub end: usize,
    pub role: SyntaxRole,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SyntaxRole {
    Keyword,
    Type,
    Function,
    Parameter,
    Module,
    Attribute,
    Variable,
    Property,
    Constructor,
    Constant,
    String,
    Comment,
    Number,
    Operator,
    Punctuation,
}

pub fn preview_source_file(path: &Path) -> Result<SourcePreview> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("could not read metadata for {}", path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!("source preview path is not a file"));
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(anyhow!(
            "source file is too large for preview: {} bytes",
            metadata.len()
        ));
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("could not read UTF-8 source file {}", path.display()))?;
    let language = detect_language(path);
    let spans = match language.as_str() {
        "csharp" => csharp_spans(&content)?,
        "typescript" | "javascript" => typescript_spans(&content, false)?,
        "tsx" | "jsx" => typescript_spans(&content, true)?,
        _ => Vec::new(),
    };

    Ok(SourcePreview {
        path: path.display().to_string(),
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("source")
            .to_string(),
        language,
        line_count: content.lines().count(),
        byte_count: content.len(),
        content,
        spans,
    })
}

pub fn list_source_files(root: &Path, limit: usize, query: Option<&str>) -> Result<SourceFileList> {
    let metadata = std::fs::metadata(root)
        .with_context(|| format!("could not read metadata for {}", root.display()))?;
    if !metadata.is_dir() {
        return Err(anyhow!("source list root is not a directory"));
    }

    let limit = if limit == 0 {
        DEFAULT_SOURCE_LIST_LIMIT
    } else {
        limit.min(MAX_SOURCE_LIST_LIMIT)
    };
    let normalized_query = query
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let mut records = Vec::new();
    let mut stats = SourceScanStats::default();
    let collect_limit = limit.saturating_add(1);
    collect_source_files(
        root,
        root,
        collect_limit,
        normalized_query.as_deref(),
        &mut records,
        &mut stats,
    )?;
    records.sort_by(|left, right| {
        left.relative_path
            .to_lowercase()
            .cmp(&right.relative_path.to_lowercase())
    });
    let truncated = records.len() > limit;
    records.truncate(limit);
    let returned_count = records.len();
    Ok(SourceFileList {
        files: records,
        limit,
        truncated,
        diagnostics: SourceScanDiagnostics {
            effective_limit: limit,
            returned_count,
            truncated,
            skipped_directory_count: stats.skipped_directory_count,
            unsupported_file_count: stats.unsupported_file_count,
            unreadable_entry_count: stats.unreadable_entry_count,
            skipped_directories: stats.skipped_directories,
        },
    })
}

pub fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|value| value.to_str()) {
        Some("cs") => "csharp".to_string(),
        Some("swift") => "swift".to_string(),
        Some("rs") => "rust".to_string(),
        Some("ts") => "typescript".to_string(),
        Some("tsx") => "tsx".to_string(),
        Some("js") => "javascript".to_string(),
        Some("jsx") => "jsx".to_string(),
        Some("svelte") => "svelte".to_string(),
        Some("json") => "json".to_string(),
        Some("md") => "markdown".to_string(),
        _ => "plain".to_string(),
    }
}

fn collect_source_files(
    root: &Path,
    current: &Path,
    limit: usize,
    query: Option<&str>,
    records: &mut Vec<SourceFileRecord>,
    stats: &mut SourceScanStats,
) -> Result<()> {
    if records.len() >= limit {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(current)
        .with_context(|| format!("could not read directory {}", current.display()))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if records.len() >= limit {
            break;
        }
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                stats.unreadable_entry_count += 1;
                continue;
            }
        };

        if metadata.is_dir() {
            if let Some(reason) = skip_dir_reason(&file_name) {
                record_skipped_directory(root, &path, &file_name, reason, stats);
                continue;
            }
            collect_source_files(root, &path, limit, query, records, stats)?;
            continue;
        }

        if !metadata.is_file() || !is_source_file(&path) {
            stats.unsupported_file_count += 1;
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
        records.push(SourceFileRecord {
            path: path.display().to_string(),
            relative_path,
            file_name,
            language: detect_language(&path),
            byte_count: metadata.len(),
        });
    }

    Ok(())
}

fn record_skipped_directory(
    root: &Path,
    path: &Path,
    name: &str,
    reason: &str,
    stats: &mut SourceScanStats,
) {
    stats.skipped_directory_count += 1;
    if stats.skipped_directories.len() >= MAX_SKIPPED_DIRECTORY_SAMPLES {
        return;
    }

    stats.skipped_directories.push(SourceSkippedDirectory {
        path: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        name: name.to_string(),
        reason: reason.to_string(),
    });
}

fn skip_dir_reason(name: &str) -> Option<&'static str> {
    let normalized = name.to_ascii_lowercase();
    if normalized.ends_with("_files") {
        return Some("saved web page asset directory");
    }

    match normalized.as_str() {
        ".git" | ".hg" | ".svn" => Some("version-control metadata directory"),
        ".agents" | ".claude" | ".codex" => Some("agent/tool state directory"),
        ".build" | ".next" | ".svelte-kit" | "bin" | "build" | "dist" | "obj" | "target" => {
            Some("build output/cache directory")
        }
        "node_modules" | "packages" | "vendor" => Some("dependency directory"),
        "worktrees" => Some("session worktree directory"),
        _ => None,
    }
}

fn source_file_matches_query(relative_path: &str, file_name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    relative_path.to_lowercase().contains(query) || file_name.to_lowercase().contains(query)
}

fn is_source_file(path: &Path) -> bool {
    !matches!(
        detect_language(path).as_str(),
        "plain" | "json" | "markdown"
    )
}

fn csharp_spans(content: &str) -> Result<Vec<SyntaxSpan>> {
    let mut parser = Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE.into();
    parser
        .set_language(&language)
        .context("failed to load C# grammar")?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| anyhow!("failed to parse C# source"))?;

    let mut spans = Vec::new();
    collect_csharp_spans(tree.root_node(), &mut spans);
    collect_csharp_query_spans(&language, tree.root_node(), content, &mut spans)?;
    Ok(normalize_spans(spans))
}

fn typescript_spans(content: &str, tsx: bool) -> Result<Vec<SyntaxSpan>> {
    let mut parser = Parser::new();
    let language = if tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    };
    parser
        .set_language(&language)
        .context("failed to load TypeScript grammar")?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| anyhow!("failed to parse TypeScript source"))?;

    let mut spans = Vec::new();
    collect_typescript_spans(tree.root_node(), &mut spans);
    collect_typescript_query_spans(&language, tree.root_node(), content, &mut spans)?;
    Ok(normalize_spans(spans))
}

const CSHARP_HIGHLIGHT_QUERY: &str = r#"
(identifier) @variable

(method_declaration name: (identifier) @function)
(local_function_statement name: (identifier) @function)
(invocation_expression (member_access_expression name: (identifier) @function))
(invocation_expression function: (identifier) @function)

(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(generic_name (identifier) @type)
(type_parameter (identifier) @property.definition)
(parameter type: (identifier) @type)
(type_argument_list (identifier) @type)
(_ type: (identifier) @type)
(base_list (identifier) @type)

(namespace_declaration name: (identifier) @module)
(namespace_declaration name: (qualified_name) @module)
(file_scoped_namespace_declaration (identifier) @module)
(file_scoped_namespace_declaration (qualified_name) @module)
(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)
(attribute name: (identifier) @attribute)
(parameter name: (identifier) @variable.parameter)
(enum_member_declaration (identifier) @property.definition)
(type_parameter_constraints_clause (identifier) @property.definition)
"#;

const TYPESCRIPT_HIGHLIGHT_QUERY: &str = r#"
(function_declaration name: (identifier) @function)
(method_definition name: (property_identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function))

(class_declaration name: (type_identifier) @type)
(interface_declaration name: (type_identifier) @type)
(type_alias_declaration name: (type_identifier) @type)
(type_identifier) @type

(required_parameter pattern: (identifier) @variable.parameter)
(optional_parameter pattern: (identifier) @variable.parameter)
"#;

fn collect_csharp_query_spans(
    language: &tree_sitter::Language,
    root: Node<'_>,
    content: &str,
    spans: &mut Vec<SyntaxSpan>,
) -> Result<()> {
    let query =
        Query::new(language, CSHARP_HIGHLIGHT_QUERY).context("invalid C# highlight query")?;
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, root, content.as_bytes());

    loop {
        captures.advance();
        let Some((query_match, capture_index)) = captures.get() else {
            break;
        };
        let Some(capture) = query_match.captures.get(*capture_index) else {
            continue;
        };
        let Some(capture_name) = capture_names.get(capture.index as usize) else {
            continue;
        };
        let Some(role) = role_for_capture(capture_name) else {
            continue;
        };
        spans.push(SyntaxSpan {
            start: capture.node.start_byte(),
            end: capture.node.end_byte(),
            role,
        });
    }

    Ok(())
}

fn collect_typescript_query_spans(
    language: &tree_sitter::Language,
    root: Node<'_>,
    content: &str,
    spans: &mut Vec<SyntaxSpan>,
) -> Result<()> {
    let query = Query::new(language, TYPESCRIPT_HIGHLIGHT_QUERY)
        .context("invalid TypeScript highlight query")?;
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, root, content.as_bytes());

    loop {
        captures.advance();
        let Some((query_match, capture_index)) = captures.get() else {
            break;
        };
        let Some(capture) = query_match.captures.get(*capture_index) else {
            continue;
        };
        let Some(capture_name) = capture_names.get(capture.index as usize) else {
            continue;
        };
        let Some(role) = role_for_capture(capture_name) else {
            continue;
        };
        spans.push(SyntaxSpan {
            start: capture.node.start_byte(),
            end: capture.node.end_byte(),
            role,
        });
    }

    Ok(())
}

fn role_for_capture(capture_name: &str) -> Option<SyntaxRole> {
    match capture_name {
        "function" => Some(SyntaxRole::Function),
        "constructor" => Some(SyntaxRole::Constructor),
        "type" | "type.builtin" => Some(SyntaxRole::Type),
        "module" => Some(SyntaxRole::Module),
        "attribute" => Some(SyntaxRole::Attribute),
        "variable.parameter" => Some(SyntaxRole::Parameter),
        "property.definition" => Some(SyntaxRole::Property),
        "variable" => Some(SyntaxRole::Variable),
        "constant.builtin" => Some(SyntaxRole::Constant),
        "operator" => Some(SyntaxRole::Operator),
        "punctuation.delimiter" | "punctuation.bracket" => Some(SyntaxRole::Punctuation),
        _ => None,
    }
}

fn normalize_spans(spans: Vec<SyntaxSpan>) -> Vec<SyntaxSpan> {
    let mut by_range: BTreeMap<(usize, usize), SyntaxRole> = BTreeMap::new();
    for span in spans {
        if span.start >= span.end {
            continue;
        }
        by_range
            .entry((span.start, span.end))
            .and_modify(|existing| {
                if role_priority(&span.role) > role_priority(existing) {
                    *existing = span.role.clone();
                }
            })
            .or_insert(span.role);
    }

    by_range
        .into_iter()
        .map(|((start, end), role)| SyntaxSpan { start, end, role })
        .collect()
}

fn role_priority(role: &SyntaxRole) -> u8 {
    match role {
        SyntaxRole::Keyword => 90,
        SyntaxRole::Function | SyntaxRole::Constructor => 85,
        SyntaxRole::Type => 80,
        SyntaxRole::Module => 75,
        SyntaxRole::Attribute => 70,
        SyntaxRole::Parameter | SyntaxRole::Property => 65,
        SyntaxRole::Constant | SyntaxRole::String | SyntaxRole::Number => 60,
        SyntaxRole::Operator | SyntaxRole::Punctuation => 55,
        SyntaxRole::Variable => 40,
        SyntaxRole::Comment => 30,
    }
}

fn collect_csharp_spans(node: Node<'_>, spans: &mut Vec<SyntaxSpan>) {
    if let Some(role) = csharp_role_for_node(node) {
        spans.push(SyntaxSpan {
            start: node.start_byte(),
            end: node.end_byte(),
            role,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_csharp_spans(child, spans);
    }
}

fn collect_typescript_spans(node: Node<'_>, spans: &mut Vec<SyntaxSpan>) {
    if let Some(role) = typescript_role_for_node(node) {
        spans.push(SyntaxSpan {
            start: node.start_byte(),
            end: node.end_byte(),
            role,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_typescript_spans(child, spans);
    }
}

fn csharp_role_for_node(node: Node<'_>) -> Option<SyntaxRole> {
    match node.kind() {
        "abstract" | "as" | "base" | "break" | "case" | "catch" | "class" | "const"
        | "continue" | "default" | "delegate" | "do" | "else" | "enum" | "event" | "explicit"
        | "extern" | "finally" | "fixed" | "for" | "foreach" | "get" | "if" | "implicit" | "in"
        | "interface" | "internal" | "is" | "lock" | "namespace" | "new" | "operator" | "out"
        | "override" | "params" | "private" | "protected" | "public" | "readonly" | "ref"
        | "return" | "sealed" | "set" | "sizeof" | "stackalloc" | "static" | "struct"
        | "switch" | "this" | "throw" | "try" | "typeof" | "unchecked" | "unsafe" | "using"
        | "virtual" | "void" | "volatile" | "while" => Some(SyntaxRole::Keyword),
        "predefined_type" => Some(SyntaxRole::Type),
        "string_literal"
        | "raw_string_literal"
        | "verbatim_string_literal"
        | "character_literal" => Some(SyntaxRole::String),
        "boolean_literal" | "null_literal" => Some(SyntaxRole::Constant),
        "comment" => Some(SyntaxRole::Comment),
        "integer_literal" | "real_literal" => Some(SyntaxRole::Number),
        "--" | "-" | "-=" | "&" | "&=" | "&&" | "+" | "++" | "+=" | "<" | "<=" | "<<" | "<<="
        | "=" | "==" | "!" | "!=" | "=>" | ">" | ">=" | ">>" | ">>=" | ">>>" | ">>>=" | "|"
        | "|=" | "||" | "?" | "??" | "??=" | "^" | "^=" | "~" | "*" | "*=" | "/" | "/=" | "%"
        | "%=" | ":" | ".." => Some(SyntaxRole::Operator),
        ";" | "." | "," | "(" | ")" | "[" | "]" | "{" | "}" => Some(SyntaxRole::Punctuation),
        _ => None,
    }
}

fn typescript_role_for_node(node: Node<'_>) -> Option<SyntaxRole> {
    match node.kind() {
        "abstract" | "as" | "async" | "await" | "break" | "case" | "catch" | "class" | "const"
        | "continue" | "debugger" | "declare" | "default" | "delete" | "do" | "else" | "enum"
        | "export" | "extends" | "finally" | "for" | "from" | "function" | "get" | "if"
        | "implements" | "import" | "in" | "infer" | "instanceof" | "interface" | "keyof"
        | "let" | "module" | "namespace" | "new" | "of" | "private" | "protected" | "public"
        | "readonly" | "require" | "return" | "satisfies" | "set" | "static" | "switch"
        | "this" | "throw" | "try" | "type" | "typeof" | "var" | "void" | "while" | "with"
        | "yield" => Some(SyntaxRole::Keyword),
        "string" | "template_string" | "string_fragment" => Some(SyntaxRole::String),
        "comment" => Some(SyntaxRole::Comment),
        "number" => Some(SyntaxRole::Number),
        "true" | "false" | "null" | "undefined" => Some(SyntaxRole::Constant),
        "=>" | "=" | "==" | "===" | "!" | "!=" | "!==" | "+" | "++" | "+=" | "-" | "--" | "-="
        | "*" | "**" | "*=" | "/" | "/=" | "%" | "%=" | "<" | "<=" | ">" | ">=" | "&&" | "||"
        | "??" | "?" | ":" | "." | "..." | "|" | "|=" | "&" | "&=" | "^" | "^=" | "~" => {
            Some(SyntaxRole::Operator)
        }
        ";" | "," | "(" | ")" | "[" | "]" | "{" | "}" => Some(SyntaxRole::Punctuation),
        _ => None,
    }
}
