use ratatui::style::{Color, Style};

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
            Some("rs") | Some("rust") if line.trim_start().starts_with("fn ") => {
                let start = line.find("fn").unwrap_or(0);
                vec![HighlightSpan { start, end: start + 2, style: Style::default().fg(Color::Magenta) }]
            }
            _ => self.fallback.highlight_line(language_hint, line),
        }
    }
}
