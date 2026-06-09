use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 200;

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

pub fn list_source_files(root: &Path, limit: usize) -> Result<Vec<SourceFileRecord>> {
    let metadata = std::fs::metadata(root)
        .with_context(|| format!("could not read metadata for {}", root.display()))?;
    if !metadata.is_dir() {
        return Err(anyhow!("source list root is not a directory"));
    }

    let limit = if limit == 0 {
        DEFAULT_SOURCE_LIST_LIMIT
    } else {
        limit.min(1_000)
    };
    let mut records = Vec::new();
    collect_source_files(root, root, limit, &mut records)?;
    records.sort_by(|left, right| {
        left.relative_path
            .to_lowercase()
            .cmp(&right.relative_path.to_lowercase())
    });
    records.truncate(limit);
    Ok(records)
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
    records: &mut Vec<SourceFileRecord>,
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
            Err(_) => continue,
        };

        if metadata.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            collect_source_files(root, &path, limit, records)?;
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

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".build"
            | ".next"
            | ".svelte-kit"
            | "bin"
            | "build"
            | "dist"
            | "node_modules"
            | "obj"
            | "packages"
            | "target"
            | "vendor"
    )
}

fn is_source_file(path: &Path) -> bool {
    !matches!(detect_language(path).as_str(), "plain" | "json" | "markdown")
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

fn collect_csharp_query_spans(
    language: &tree_sitter::Language,
    root: Node<'_>,
    content: &str,
    spans: &mut Vec<SyntaxSpan>,
) -> Result<()> {
    let query = Query::new(language, CSHARP_HIGHLIGHT_QUERY).context("invalid C# highlight query")?;
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

fn csharp_role_for_node(node: Node<'_>) -> Option<SyntaxRole> {
    match node.kind() {
        "abstract" | "as" | "base" | "break" | "case" | "catch" | "class" | "const"
        | "continue" | "default" | "delegate" | "do" | "else" | "enum" | "event" | "explicit"
        | "extern" | "finally" | "fixed" | "for" | "foreach" | "get" | "if" | "implicit"
        | "in" | "interface" | "internal" | "is" | "lock" | "namespace" | "new" | "operator"
        | "out" | "override" | "params" | "private" | "protected" | "public" | "readonly"
        | "ref" | "return" | "sealed" | "set" | "sizeof" | "stackalloc" | "static" | "struct"
        | "switch" | "this" | "throw" | "try" | "typeof" | "unchecked" | "unsafe" | "using"
        | "virtual" | "void" | "volatile" | "while" => Some(SyntaxRole::Keyword),
        "predefined_type" => Some(SyntaxRole::Type),
        "string_literal" | "raw_string_literal" | "verbatim_string_literal" | "character_literal" => {
            Some(SyntaxRole::String)
        }
        "boolean_literal" | "null_literal" => Some(SyntaxRole::Constant),
        "comment" => Some(SyntaxRole::Comment),
        "integer_literal" | "real_literal" => Some(SyntaxRole::Number),
        "--" | "-" | "-=" | "&" | "&=" | "&&" | "+" | "++" | "+=" | "<" | "<=" | "<<"
        | "<<=" | "=" | "==" | "!" | "!=" | "=>" | ">" | ">=" | ">>" | ">>=" | ">>>"
        | ">>>=" | "|" | "|=" | "||" | "?" | "??" | "??=" | "^" | "^=" | "~" | "*"
        | "*=" | "/" | "/=" | "%" | "%=" | ":" | ".." => Some(SyntaxRole::Operator),
        ";" | "." | "," | "(" | ")" | "[" | "]" | "{" | "}" => Some(SyntaxRole::Punctuation),
        _ => None,
    }
}
