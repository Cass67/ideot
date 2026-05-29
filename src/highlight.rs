use ratatui::style::{Color, Modifier, Style};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

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

pub struct SimpleTreeSitterHighlighter {
    rust: Option<TreeSitterLineHighlighter>,
    fallback: PlainHighlighter,
}

impl Default for SimpleTreeSitterHighlighter {
    fn default() -> Self {
        Self { rust: TreeSitterLineHighlighter::rust(), fallback: PlainHighlighter }
    }
}

impl std::fmt::Debug for SimpleTreeSitterHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleTreeSitterHighlighter").finish_non_exhaustive()
    }
}

impl Highlighter for SimpleTreeSitterHighlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan> {
        match language_hint {
            Some("rs") | Some("rust") => self
                .rust
                .as_mut()
                .and_then(|highlighter| highlighter.highlight_line(line))
                .unwrap_or_else(|| self.fallback.highlight_line(language_hint, line)),
            _ => self.fallback.highlight_line(language_hint, line),
        }
    }
}

struct TreeSitterLineHighlighter {
    parser: Parser,
    query: Query,
}

impl TreeSitterLineHighlighter {
    fn rust() -> Option<Self> {
        let language: Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let query = Query::new(&language, tree_sitter_rust::HIGHLIGHTS_QUERY).ok()?;
        Some(Self { parser, query })
    }

    fn highlight_line(&mut self, line: &str) -> Option<Vec<HighlightSpan>> {
        let tree = self.parser.parse(line, None)?;
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&self.query, tree.root_node(), line.as_bytes());
        let names = self.query.capture_names();
        let mut spans = Vec::new();

        while let Some((mat, capture_index)) = captures.next() {
            let capture = mat.captures[*capture_index];
            let name = names[capture.index as usize];
            let style = style_for_capture(name);
            if style == Style::default() {
                continue;
            }
            let start = capture.node.start_byte();
            let end = capture.node.end_byte();
            if start < end && end <= line.len() {
                spans.push(HighlightSpan { start, end, style });
            }
        }

        spans.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));
        let mut non_overlapping = Vec::new();
        let mut cursor = 0;
        for span in spans {
            if span.start >= cursor {
                cursor = span.end;
                non_overlapping.push(span);
            }
        }
        Some(non_overlapping)
    }
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "attribute" | "constructor" | "module" | "namespace" => Style::default().fg(Color::Yellow),
        "comment" => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        "constant" | "constant.builtin" | "number" => Style::default().fg(Color::Yellow),
        "function" | "function.method" | "function.builtin" | "function.macro" => Style::default().fg(Color::Cyan),
        "keyword" | "keyword.function" | "keyword.operator" | "keyword.storage" | "operator" => Style::default().fg(Color::Magenta),
        "property" | "variable" | "variable.parameter" => Style::default().fg(Color::White),
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => Style::default().fg(Color::DarkGray),
        "string" | "string.special" | "character" => Style::default().fg(Color::Green),
        "type" | "type.builtin" => Style::default().fg(Color::Blue),
        _ => Style::default(),
    }
}
