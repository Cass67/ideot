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
    go: Option<TreeSitterLineHighlighter>,
    python: Option<TreeSitterLineHighlighter>,
    javascript: Option<TreeSitterLineHighlighter>,
    markdown: Option<TreeSitterLineHighlighter>,
    json: Option<TreeSitterLineHighlighter>,
    bash: Option<TreeSitterLineHighlighter>,
    html: Option<TreeSitterLineHighlighter>,
    css: Option<TreeSitterLineHighlighter>,
    c: Option<TreeSitterLineHighlighter>,
    cpp: Option<TreeSitterLineHighlighter>,
    lua: Option<TreeSitterLineHighlighter>,
    fallback: PlainHighlighter,
}

impl Default for SimpleTreeSitterHighlighter {
    fn default() -> Self {
        Self {
            rust: TreeSitterLineHighlighter::new(
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY,
            ),
            go: TreeSitterLineHighlighter::new(
                tree_sitter_go::LANGUAGE.into(),
                tree_sitter_go::HIGHLIGHTS_QUERY,
            ),
            python: TreeSitterLineHighlighter::new(
                tree_sitter_python::LANGUAGE.into(),
                tree_sitter_python::HIGHLIGHTS_QUERY,
            ),
            javascript: TreeSitterLineHighlighter::new(
                tree_sitter_javascript::LANGUAGE.into(),
                tree_sitter_javascript::HIGHLIGHT_QUERY,
            ),
            markdown: TreeSitterLineHighlighter::new(
                tree_sitter_md::LANGUAGE.into(),
                tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            ),
            json: TreeSitterLineHighlighter::new(
                tree_sitter_json::LANGUAGE.into(),
                tree_sitter_json::HIGHLIGHTS_QUERY,
            ),
            bash: TreeSitterLineHighlighter::new(
                tree_sitter_bash::LANGUAGE.into(),
                tree_sitter_bash::HIGHLIGHT_QUERY,
            ),
            html: TreeSitterLineHighlighter::new(
                tree_sitter_html::LANGUAGE.into(),
                tree_sitter_html::HIGHLIGHTS_QUERY,
            ),
            css: TreeSitterLineHighlighter::new(
                tree_sitter_css::LANGUAGE.into(),
                tree_sitter_css::HIGHLIGHTS_QUERY,
            ),
            c: TreeSitterLineHighlighter::new(
                tree_sitter_c::LANGUAGE.into(),
                tree_sitter_c::HIGHLIGHT_QUERY,
            ),
            cpp: TreeSitterLineHighlighter::new(
                tree_sitter_cpp::LANGUAGE.into(),
                tree_sitter_cpp::HIGHLIGHT_QUERY,
            ),
            lua: TreeSitterLineHighlighter::new(
                tree_sitter_lua::LANGUAGE.into(),
                tree_sitter_lua::HIGHLIGHTS_QUERY,
            ),
            fallback: PlainHighlighter,
        }
    }
}

impl std::fmt::Debug for SimpleTreeSitterHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleTreeSitterHighlighter")
            .finish_non_exhaustive()
    }
}

impl Highlighter for SimpleTreeSitterHighlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan> {
        match language_hint.map(str::to_ascii_lowercase).as_deref() {
            Some("rs" | "rust") => {
                run_tree_sitter(&mut self.rust, line, language_hint).unwrap_or_default()
            }
            Some("go") => with_function_name_heuristic(
                run_tree_sitter(&mut self.go, line, language_hint).unwrap_or_default(),
                line,
                "func",
            ),
            Some("py" | "python" | "pyw") => with_function_name_heuristic(
                run_tree_sitter(&mut self.python, line, language_hint).unwrap_or_default(),
                line,
                "def",
            ),
            Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx") => with_function_name_heuristic(
                run_tree_sitter(&mut self.javascript, line, language_hint).unwrap_or_default(),
                line,
                "function",
            ),
            Some("md" | "markdown") => markdown_heuristic(line)
                .or_else(|| run_tree_sitter(&mut self.markdown, line, language_hint))
                .unwrap_or_default(),
            Some("json") => {
                run_tree_sitter(&mut self.json, line, language_hint).unwrap_or_default()
            }
            Some("sh" | "bash" | "zsh") => run_tree_sitter(&mut self.bash, line, language_hint)
                .filter(|spans| !spans.is_empty())
                .unwrap_or_else(|| bash_heuristic(line)),
            Some("html" | "htm") => {
                run_tree_sitter(&mut self.html, line, language_hint).unwrap_or_default()
            }
            Some("css" | "scss" | "less") => run_tree_sitter(&mut self.css, line, language_hint)
                .filter(|spans| !spans.is_empty())
                .unwrap_or_else(|| css_heuristic(line)),
            Some("c" | "h") => run_tree_sitter(&mut self.c, line, language_hint)
                .filter(|spans| !spans.is_empty())
                .unwrap_or_else(|| c_like_heuristic(line)),
            Some("cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx") => {
                run_tree_sitter(&mut self.cpp, line, language_hint)
                    .filter(|spans| !spans.is_empty())
                    .unwrap_or_else(|| c_like_heuristic(line))
            }
            Some("lua") => run_tree_sitter(&mut self.lua, line, language_hint)
                .filter(|spans| !spans.is_empty())
                .unwrap_or_else(|| lua_heuristic(line)),
            Some("toml") => toml_heuristic(line),
            Some("yaml" | "yml") => yaml_heuristic(line),
            _ => self.fallback.highlight_line(language_hint, line),
        }
    }
}

fn run_tree_sitter(
    highlighter: &mut Option<TreeSitterLineHighlighter>,
    line: &str,
    _language_hint: Option<&str>,
) -> Option<Vec<HighlightSpan>> {
    highlighter.as_mut()?.highlight_line(line)
}

struct TreeSitterLineHighlighter {
    parser: Parser,
    query: Query,
}

impl TreeSitterLineHighlighter {
    fn new(language: Language, query_source: &str) -> Option<Self> {
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;
        let query = Query::new(&language, query_source).ok()?;
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

fn with_function_name_heuristic(
    mut spans: Vec<HighlightSpan>,
    line: &str,
    keyword: &str,
) -> Vec<HighlightSpan> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let Some(after_keyword) = trimmed.strip_prefix(keyword) else {
        return spans;
    };
    let Some(name_offset) = after_keyword.find(|ch: char| !ch.is_whitespace()) else {
        return spans;
    };
    let start = leading + keyword.len() + name_offset;
    let end = line[start..]
        .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .map(|offset| start + offset)
        .unwrap_or(line.len());
    if start < end {
        spans.push(HighlightSpan {
            start,
            end,
            style: Style::default().fg(Color::Cyan),
        });
        spans.sort_by_key(|span| (span.start, span.end));
    }
    spans
}

fn lua_heuristic(line: &str) -> Vec<HighlightSpan> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    for keyword in ["local", "function", "return", "if", "then", "end"] {
        if trimmed.starts_with(keyword) {
            return vec![HighlightSpan {
                start: leading,
                end: leading + keyword.len(),
                style: style_for_capture("keyword"),
            }];
        }
    }
    Vec::new()
}

fn c_like_heuristic(line: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    for keyword in [
        "int", "void", "return", "class", "struct", "public", "private",
    ] {
        let mut search_start = 0;
        while let Some(offset) = line[search_start..].find(keyword) {
            let start = search_start + offset;
            let end = start + keyword.len();
            let before_ok = start == 0
                || !line[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric());
            let after_ok = end == line.len()
                || !line[end..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric());
            if before_ok && after_ok {
                spans.push(HighlightSpan {
                    start,
                    end,
                    style: style_for_capture(if ["int", "void"].contains(&keyword) {
                        "type.builtin"
                    } else {
                        "keyword"
                    }),
                });
            }
            search_start = end;
        }
    }
    spans.sort_by_key(|span| (span.start, span.end));
    spans
}

fn css_heuristic(line: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    if let Some(selector_end) = line.find('{') {
        let selector = line[..selector_end].trim();
        if !selector.is_empty() {
            let start = line[..selector_end].find(selector).unwrap_or(0);
            spans.push(HighlightSpan {
                start,
                end: start + selector.len(),
                style: style_for_capture("type"),
            });
        }
    }
    if let Some(colon) = line.find(':') {
        let property_start = line[..colon]
            .rfind(|ch: char| ch == '{' || ch.is_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(0);
        if property_start < colon {
            spans.push(HighlightSpan {
                start: property_start,
                end: colon,
                style: style_for_capture("property"),
            });
        }
    }
    spans
}

fn bash_heuristic(line: &str) -> Vec<HighlightSpan> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    if trimmed.starts_with('#') {
        return vec![HighlightSpan {
            start: leading,
            end: line.len(),
            style: style_for_capture("comment"),
        }];
    }
    for keyword in [
        "if", "then", "else", "elif", "fi", "for", "while", "do", "done",
    ] {
        if trimmed.starts_with(keyword) {
            return vec![HighlightSpan {
                start: leading,
                end: leading + keyword.len(),
                style: style_for_capture("keyword"),
            }];
        }
    }
    Vec::new()
}

fn toml_heuristic(line: &str) -> Vec<HighlightSpan> {
    if line.trim_start().starts_with('#') {
        return vec![HighlightSpan {
            start: 0,
            end: line.len(),
            style: style_for_capture("comment"),
        }];
    }
    let mut spans = Vec::new();
    if let Some(eq) = line.find('=') {
        if let Some(key_start) = line[..eq].find(|ch: char| !ch.is_whitespace()) {
            let key_end = line[..eq].trim_end().len();
            spans.push(HighlightSpan {
                start: key_start,
                end: key_end,
                style: Style::default().fg(Color::Cyan),
            });
        }
        add_value_span(line, eq + 1, &mut spans);
    } else if line.trim_start().starts_with('[') {
        spans.push(HighlightSpan {
            start: 0,
            end: line.len(),
            style: Style::default().fg(Color::Yellow),
        });
    }
    spans
}

fn yaml_heuristic(line: &str) -> Vec<HighlightSpan> {
    if line.trim_start().starts_with('#') {
        return vec![HighlightSpan {
            start: 0,
            end: line.len(),
            style: style_for_capture("comment"),
        }];
    }
    let mut spans = Vec::new();
    if let Some(colon) = line.find(':') {
        if let Some(key_start) = line[..colon].find(|ch: char| !ch.is_whitespace() && ch != '-') {
            let key_end = line[..colon].trim_end().len();
            spans.push(HighlightSpan {
                start: key_start,
                end: key_end,
                style: Style::default().fg(Color::Cyan),
            });
        }
        add_value_span(line, colon + 1, &mut spans);
    }
    spans
}

fn markdown_heuristic(line: &str) -> Option<Vec<HighlightSpan>> {
    let heading_marks = line.chars().take_while(|ch| *ch == '#').count();
    if heading_marks == 0 {
        return None;
    }
    let mut spans = vec![HighlightSpan {
        start: 0,
        end: heading_marks,
        style: Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    }];
    if line.len() > heading_marks + 1 {
        spans.push(HighlightSpan {
            start: heading_marks + 1,
            end: line.len(),
            style: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        });
    }
    Some(spans)
}

fn add_value_span(line: &str, after_separator: usize, spans: &mut Vec<HighlightSpan>) {
    if let Some(offset) = line[after_separator..].find(|ch: char| !ch.is_whitespace()) {
        let start = after_separator + offset;
        let end = line[start..]
            .find('#')
            .map(|end| start + end)
            .unwrap_or(line.len());
        if start < end {
            spans.push(HighlightSpan {
                start,
                end: end.trim_ascii_end_index(line),
                style: Style::default().fg(Color::Green),
            });
        }
    }
}

trait TrimAsciiEndIndex {
    fn trim_ascii_end_index(self, line: &str) -> usize;
}

impl TrimAsciiEndIndex for usize {
    fn trim_ascii_end_index(self, line: &str) -> usize {
        let mut end = self;
        while end > 0 && line.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        end
    }
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "attribute" | "constructor" | "module" | "namespace" => Style::default().fg(Color::Yellow),
        "comment" => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
        "constant" | "constant.builtin" | "number" => Style::default().fg(Color::Yellow),
        "function" | "function.method" | "function.builtin" | "function.macro" | "method" => {
            Style::default().fg(Color::Cyan)
        }
        "keyword" | "keyword.function" | "keyword.operator" | "keyword.storage" | "operator"
        | "repeat" | "conditional" => Style::default().fg(Color::Magenta),
        "property" | "variable" | "variable.parameter" => Style::default().fg(Color::White),
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => {
            Style::default().fg(Color::DarkGray)
        }
        "string" | "string.special" | "character" => Style::default().fg(Color::Green),
        "type" | "type.builtin" | "type.definition" | "type.qualifier" => {
            Style::default().fg(Color::Blue)
        }
        "markup.heading" | "markup.heading.marker" => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        _ => Style::default(),
    }
}
