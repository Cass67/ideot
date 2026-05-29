use crate::app::App;
use crate::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(root[0]);

    let files: Vec<ListItem> = if app.search_open() {
        app.search(app.search_query())
            .into_iter()
            .enumerate()
            .skip(app.explorer_scroll())
            .take(200)
            .map(|(index, file)| {
                let prefix = if index == app.selected_file() {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(format!("{prefix}{}", file.relative))
            })
            .collect()
    } else {
        app.explorer_entries()
            .into_iter()
            .enumerate()
            .skip(app.explorer_scroll())
            .take(200)
            .map(|(index, entry)| {
                let prefix = if index == app.selected_file() {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(format!("{prefix}{}", entry.label))
            })
            .collect()
    };
    frame.render_widget(
        List::new(files).block(Block::default().title("files").borders(Borders::ALL)),
        panes[0],
    );

    let editor_lines =
        highlighted_editor_lines_for_height(app, panes[1].height.saturating_sub(2) as usize);
    frame.render_widget(
        Paragraph::new(editor_lines).block(Block::default().title("editor").borders(Borders::ALL)),
        panes[1],
    );
    if let Some((x, y)) = editor_cursor_screen_position(app, panes[1]) {
        frame.set_cursor_position((x, y));
    }

    if app.search_open() {
        let area = centered_rect(60, 20, frame.area());
        let text = format!("Find file: {}", app.search_query());
        frame.render_widget(
            Paragraph::new(text).block(Block::default().title("search").borders(Borders::ALL)),
            area,
        );
    }

    let status = app.current_relative().unwrap_or("no file");
    frame.render_widget(Paragraph::new(status.to_string()), root[1]);
}

pub fn editor_cursor_screen_position(app: &App, editor_area: Rect) -> Option<(u16, u16)> {
    let cursor = app.editor()?.cursor();
    let visible_line = cursor.line.checked_sub(app.editor_scroll())?;
    let inner_x = editor_area.x.saturating_add(1);
    let inner_y = editor_area.y.saturating_add(1);
    let inner_height = editor_area.height.saturating_sub(2) as usize;
    let inner_width = editor_area.width.saturating_sub(2) as usize;
    if visible_line >= inner_height {
        return None;
    }
    let column = cursor.column.min(inner_width.saturating_sub(1));
    Some((inner_x + column as u16, inner_y + visible_line as u16))
}

pub fn highlighted_editor_lines_for_height(app: &App, height: usize) -> Vec<Line<'static>> {
    let Some(editor) = app.editor() else {
        return vec![Line::from(
            "Open a file with Ctrl-P or select from explorer",
        )];
    };
    let mut highlighter = SimpleTreeSitterHighlighter::default();
    editor
        .buffer()
        .lines()
        .iter()
        .skip(app.editor_scroll())
        .take(height)
        .map(|line| highlighted_line(&mut highlighter, app.language_hint(), line))
        .collect()
}

fn highlighted_line(
    highlighter: &mut dyn Highlighter,
    language_hint: Option<&str>,
    line: &str,
) -> Line<'static> {
    let spans = highlighter.highlight_line(language_hint, line);
    if spans.is_empty() {
        return Line::from(line.to_string());
    }

    let mut rendered = Vec::new();
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor || span.end > line.len() || span.start >= span.end {
            continue;
        }
        if span.start > cursor {
            rendered.push(Span::raw(line[cursor..span.start].to_string()));
        }
        rendered.push(Span::styled(
            line[span.start..span.end].to_string(),
            span.style,
        ));
        cursor = span.end;
    }
    if cursor < line.len() {
        rendered.push(Span::raw(line[cursor..].to_string()));
    }
    Line::from(rendered)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
