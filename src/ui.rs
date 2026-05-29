use crate::app::App;
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

    let files: Vec<ListItem> = app
        .search(if app.search_open() { app.search_query() } else { "" })
        .into_iter()
        .enumerate()
        .take(200)
        .map(|(index, file)| {
            let prefix = if index == app.selected_file() { "> " } else { "  " };
            ListItem::new(format!("{prefix}{}", file.relative))
        })
        .collect();
    frame.render_widget(List::new(files).block(Block::default().title("files").borders(Borders::ALL)), panes[0]);

    let editor_text = app
        .editor()
        .map(|editor| editor.buffer().text())
        .unwrap_or_else(|| "Open a file with Ctrl-P or select from explorer".to_string());
    frame.render_widget(Paragraph::new(editor_text).block(Block::default().title("editor").borders(Borders::ALL)), panes[1]);

    if app.search_open() {
        let area = centered_rect(60, 20, frame.area());
        let text = format!("Find file: {}", app.search_query());
        frame.render_widget(Paragraph::new(text).block(Block::default().title("search").borders(Borders::ALL)), area);
    }

    let status = app.current_relative().unwrap_or("no file");
    frame.render_widget(Paragraph::new(status.to_string()), root[1]);
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
