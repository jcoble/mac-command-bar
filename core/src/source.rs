use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::Path;
use tree_sitter::{Node, Parser};

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;

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
    String,
    Comment,
    Number,
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

fn csharp_spans(content: &str) -> Result<Vec<SyntaxSpan>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .context("failed to load C# grammar")?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| anyhow!("failed to parse C# source"))?;

    let mut spans = Vec::new();
    collect_csharp_spans(tree.root_node(), &mut spans);
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup();
    Ok(spans)
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
        "comment" => Some(SyntaxRole::Comment),
        "integer_literal" | "real_literal" => Some(SyntaxRole::Number),
        _ => None,
    }
}
