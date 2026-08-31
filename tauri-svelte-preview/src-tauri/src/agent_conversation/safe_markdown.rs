use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SafeInlinePart {
    #[serde(rename = "text")]
    Text { value: String },
    #[serde(rename = "code")]
    Code { value: String },
    #[serde(rename = "strong")]
    Strong { parts: Vec<SafeInlinePart> },
    #[serde(rename = "emphasis")]
    Emphasis { parts: Vec<SafeInlinePart> },
    #[serde(rename = "strike")]
    Strike { parts: Vec<SafeInlinePart> },
    #[serde(rename = "link")]
    Link {
        href: String,
        parts: Vec<SafeInlinePart>,
    },
    #[serde(rename = "file-link")]
    FileLink {
        path: String,
        parts: Vec<SafeInlinePart>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeListItem {
    pub task: bool,
    pub checked: bool,
    pub parts: Vec<SafeInlinePart>,
    pub blocks: Vec<SafeMarkdownBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SafeMarkdownBlock {
    #[serde(rename = "paragraph")]
    Paragraph { parts: Vec<SafeInlinePart> },
    #[serde(rename = "heading")]
    Heading {
        level: u8,
        parts: Vec<SafeInlinePart>,
    },
    #[serde(rename = "code")]
    Code {
        language: String,
        value: String,
        complete: bool,
    },
    #[serde(rename = "quote")]
    Quote { blocks: Vec<SafeMarkdownBlock> },
    #[serde(rename = "list")]
    List {
        ordered: bool,
        items: Vec<SafeListItem>,
    },
    #[serde(rename = "table")]
    Table {
        headers: Vec<Vec<SafeInlinePart>>,
        rows: Vec<Vec<Vec<SafeInlinePart>>>,
    },
    #[serde(rename = "rule")]
    Rule,
}

fn has_control_characters(s: &str) -> bool {
    s.chars().any(|c| c.is_control() && c != '\t' && c != '\n')
}

fn is_safe_protocol(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("mailto:")
        || lower.starts_with('#')
}

fn has_scheme_prefix(href: &str) -> bool {
    if let Some(pos) = href.find(':') {
        let prefix = &href[..pos];
        !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-')
    } else {
        false
    }
}

fn normalize_file_href(href: &str) -> &str {
    let trimmed = href.trim();
    if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("file:") {
        stripped
    } else {
        trimmed
    }
}

fn file_path(href: &str) -> Option<String> {
    let path = href.trim();
    if path.is_empty() || has_scheme_prefix(path) || has_control_characters(path) {
        None
    } else {
        Some(path.to_string())
    }
}

fn part_plain_text(part: &SafeInlinePart) -> String {
    match part {
        SafeInlinePart::Text { value } | SafeInlinePart::Code { value } => value.clone(),
        SafeInlinePart::Strong { parts }
        | SafeInlinePart::Emphasis { parts }
        | SafeInlinePart::Strike { parts }
        | SafeInlinePart::Link { parts, .. }
        | SafeInlinePart::FileLink { parts, .. } => {
            parts.iter().map(part_plain_text).collect::<Vec<_>>().join("")
        }
    }
}

fn create_link_part(href: &str, parts: Vec<SafeInlinePart>) -> SafeInlinePart {
    let trimmed = href.trim();
    if trimmed.to_ascii_lowercase().starts_with("file:") {
        let normalized = normalize_file_href(trimmed);
        if let Some(path) = file_path(normalized) {
            return SafeInlinePart::FileLink { path, parts };
        }
        return SafeInlinePart::Text {
            value: parts.iter().map(part_plain_text).collect::<Vec<_>>().join(""),
        };
    }

    if is_safe_protocol(trimmed) && !has_control_characters(trimmed) {
        return SafeInlinePart::Link {
            href: trimmed.to_string(),
            parts,
        };
    }

    if let Some(path) = file_path(trimmed) {
        return SafeInlinePart::FileLink { path, parts };
    }

    SafeInlinePart::Text {
        value: parts.iter().map(part_plain_text).collect::<Vec<_>>().join(""),
    }
}

fn collect_inline_parts<'a>(node: &'a AstNode<'a>) -> Vec<SafeInlinePart> {
    let mut parts = Vec::new();
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Text(text) => {
                parts.push(SafeInlinePart::Text {
                    value: text.clone(),
                });
            }
            NodeValue::Code(code) => {
                parts.push(SafeInlinePart::Code {
                    value: code.literal.clone(),
                });
            }
            NodeValue::Strong => {
                let inner = collect_inline_parts(child);
                parts.push(SafeInlinePart::Strong { parts: inner });
            }
            NodeValue::Emph => {
                let inner = collect_inline_parts(child);
                parts.push(SafeInlinePart::Emphasis { parts: inner });
            }
            NodeValue::Strikethrough => {
                let inner = collect_inline_parts(child);
                parts.push(SafeInlinePart::Strike { parts: inner });
            }
            NodeValue::Link(link) => {
                let inner = collect_inline_parts(child);
                parts.push(create_link_part(&link.url, inner));
            }
            NodeValue::Image(img) => {
                let inner = collect_inline_parts(child);
                let fallback = if inner.is_empty() {
                    vec![SafeInlinePart::Text {
                        value: img.url.clone(),
                    }]
                } else {
                    inner
                };
                parts.push(create_link_part(&img.url, fallback));
            }
            NodeValue::LineBreak | NodeValue::SoftBreak => {
                parts.push(SafeInlinePart::Text {
                    value: "\n".to_string(),
                });
            }
            NodeValue::HtmlInline(html) => {
                parts.push(SafeInlinePart::Text { value: html.clone() });
            }
            _ => {
                let inner = collect_inline_parts(child);
                parts.extend(inner);
            }
        }
    }
    if parts.is_empty() {
        vec![SafeInlinePart::Text {
            value: String::new(),
        }]
    } else {
        parts
    }
}

fn collect_blocks<'a>(root: &'a AstNode<'a>) -> Vec<SafeMarkdownBlock> {
    let mut blocks = Vec::new();

    for child in root.children() {
        match &child.data.borrow().value {
            NodeValue::Paragraph => {
                let parts = collect_inline_parts(child);
                blocks.push(SafeMarkdownBlock::Paragraph { parts });
            }
            NodeValue::Heading(heading) => {
                let parts = collect_inline_parts(child);
                blocks.push(SafeMarkdownBlock::Heading {
                    level: heading.level,
                    parts,
                });
            }
            NodeValue::CodeBlock(code) => {
                let language = code.info.trim().split_whitespace().next().unwrap_or("").to_string();
                let value = code.literal.clone();
                let complete = !code.fenced || value.ends_with('\n');
                blocks.push(SafeMarkdownBlock::Code {
                    language,
                    value,
                    complete,
                });
            }
            NodeValue::BlockQuote => {
                let inner = collect_blocks(child);
                blocks.push(SafeMarkdownBlock::Quote { blocks: inner });
            }
            NodeValue::List(list) => {
                let ordered = matches!(list.list_type, comrak::nodes::ListType::Ordered);
                let mut items = Vec::new();
                for item_node in child.children() {
                    let mut item_parts = Vec::new();
                    let mut item_blocks = Vec::new();
                    let mut task = false;
                    let mut checked = false;

                    for sub in item_node.children() {
                        match &sub.data.borrow().value {
                            NodeValue::TaskItem(status) => {
                                task = true;
                                checked = status.is_some();
                            }
                            NodeValue::Paragraph => {
                                if item_parts.is_empty() {
                                    item_parts = collect_inline_parts(sub);
                                } else {
                                    item_blocks.push(SafeMarkdownBlock::Paragraph {
                                        parts: collect_inline_parts(sub),
                                    });
                                }
                            }
                            _ => {
                                let sub_blocks = collect_blocks(item_node);
                                item_blocks.extend(sub_blocks);
                                break;
                            }
                        }
                    }
                    if item_parts.is_empty() && item_blocks.is_empty() {
                        item_parts = collect_inline_parts(item_node);
                    }
                    items.push(SafeListItem {
                        task,
                        checked,
                        parts: item_parts,
                        blocks: item_blocks,
                    });
                }
                blocks.push(SafeMarkdownBlock::List { ordered, items });
            }
            NodeValue::Table(_) => {
                let mut headers = Vec::new();
                let mut rows = Vec::new();
                for row_node in child.children() {
                    let is_header = match &row_node.data.borrow().value {
                        NodeValue::TableRow(header) => *header,
                        _ => false,
                    };
                    let mut cells = Vec::new();
                    for cell_node in row_node.children() {
                        cells.push(collect_inline_parts(cell_node));
                    }
                    if is_header {
                        headers = cells;
                    } else {
                        rows.push(cells);
                    }
                }
                blocks.push(SafeMarkdownBlock::Table { headers, rows });
            }
            NodeValue::ThematicBreak => {
                blocks.push(SafeMarkdownBlock::Rule);
            }
            NodeValue::HtmlBlock(html) => {
                blocks.push(SafeMarkdownBlock::Paragraph {
                    parts: vec![SafeInlinePart::Text {
                        value: html.literal.clone(),
                    }],
                });
            }
            _ => {}
        }
    }

    blocks
}

/// Parse raw markdown text into safe structured AST blocks.
pub fn parse_safe_markdown(markdown: &str) -> Vec<SafeMarkdownBlock> {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;

    let root = parse_document(&arena, markdown, &options);
    collect_blocks(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_paragraph_and_code() {
        let md = "Hello world `foo` **bold**";
        let blocks = parse_safe_markdown(md);
        assert_eq!(blocks.len(), 1);
        if let SafeMarkdownBlock::Paragraph { parts } = &blocks[0] {
            assert_eq!(parts.len(), 4);
            assert_eq!(
                parts[0],
                SafeInlinePart::Text {
                    value: "Hello world ".to_string()
                }
            );
            assert_eq!(
                parts[1],
                SafeInlinePart::Code {
                    value: "foo".to_string()
                }
            );
            assert_eq!(
                parts[2],
                SafeInlinePart::Text {
                    value: " ".to_string()
                }
            );
            assert_eq!(
                parts[3],
                SafeInlinePart::Strong {
                    parts: vec![SafeInlinePart::Text {
                        value: "bold".to_string()
                    }]
                }
            );
        } else {
            panic!("Expected paragraph block");
        }
    }

    #[test]
    fn parses_file_and_web_links() {
        let md = "[docs](https://example.com) and [code](file:///src/main.rs) and [rel](src/app.ts:42)";
        let blocks = parse_safe_markdown(md);
        assert_eq!(blocks.len(), 1);
        if let SafeMarkdownBlock::Paragraph { parts } = &blocks[0] {
            assert!(matches!(&parts[0], SafeInlinePart::Link { href, .. } if href == "https://example.com"));
            assert!(matches!(&parts[2], SafeInlinePart::FileLink { path, .. } if path == "/src/main.rs"));
            assert!(matches!(&parts[4], SafeInlinePart::FileLink { path, .. } if path == "src/app.ts:42"));
        } else {
            panic!("Expected paragraph block");
        }
    }
}
