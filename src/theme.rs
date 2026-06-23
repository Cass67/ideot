use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders},
};

// ponytail: hardcoded palette; add settings-driven theme only if a user asks.
const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;

/// Cursor glyph for the focused list row (replaces the legacy `>` marker).
pub fn selection_prefix(selected: bool) -> &'static str {
    if selected {
        "❯ "
    } else {
        "  "
    }
}

/// Pane chrome: rounded border, accent when focused, dim when not.
pub fn pane_block(title: &str, focused: bool) -> Block<'static> {
    styled_block(title, if focused { ACCENT } else { DIM })
}

/// Modal/overlay chrome: rounded accent border.
pub fn overlay_block(title: &str) -> Block<'static> {
    styled_block(title, ACCENT)
}

fn styled_block(title: &str, fg: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(format!(" {title} ")).style(Style::default().fg(fg)))
        .border_style(Style::default().fg(fg))
}
