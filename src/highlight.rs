use ratatui::style::{Color, Style};
use tree_sitter::{Parser, Query, QueryCursor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

pub trait Highlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan>;
}

#[derive(Debug, Default)]
pub struct PlainHighlighter;

impl Highlighter for PlainHighlighter {
    fn highlight_line(&mut self, _language_hint: Option<&str>, _line: &str) -> Vec<HighlightSpan> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct SimpleTreeSitterHighlighter {
    fallback: PlainHighlighter,
}

impl Highlighter for SimpleTreeSitterHighlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan> {
        match language_hint {
            Some("rs") | Some("rust") => rust_highlights(line).unwrap_or_else(|| self.fallback.highlight_line(language_hint, line)),
            _ => self.fallback.highlight_line(language_hint, line),
        }
    }
}

fn rust_highlights(line: &str) -> Option<Vec<HighlightSpan>> {
    let language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(line, None)?;
    let query = Query::new(
        &language,
        r#"
        "fn" @keyword
        "let" @keyword
        (function_item name: (identifier) @function)
        (call_expression function: (identifier) @function)
        (line_comment) @comment
        (string_literal) @string
        (integer_literal) @number
        "#,
    ).ok()?;
    let mut cursor = QueryCursor::new();
    let mut spans = Vec::new();
    let captures = cursor.captures(&query, tree.root_node(), line.as_bytes());
    use streaming_iterator::StreamingIterator;
    let names = query.capture_names();
    let mut captures = captures;
    while let Some((mat, capture_index)) = captures.next() {
        let capture = mat.captures[*capture_index];
        let name = names[capture.index as usize];
        let style = style_for_capture(name);
        spans.push(HighlightSpan {
            start: capture.node.start_byte(),
            end: capture.node.end_byte(),
            style,
        });
    }
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup_by_key(|span| (span.start, span.end));
    Some(spans)
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "keyword" => Style::default().fg(Color::Magenta),
        "function" => Style::default().fg(Color::Cyan),
        "comment" => Style::default().fg(Color::DarkGray),
        "string" => Style::default().fg(Color::Green),
        "number" => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    }
}
