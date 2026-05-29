use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
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
        .search("")
        .into_iter()
        .take(200)
        .map(|file| ListItem::new(file.relative))
        .collect();
    frame.render_widget(List::new(files).block(Block::default().title("files").borders(Borders::ALL)), panes[0]);

    let editor_text = app
        .editor()
        .map(|editor| editor.buffer().text())
        .unwrap_or_else(|| "Open a file with Ctrl-P or select from explorer".to_string());
    frame.render_widget(Paragraph::new(editor_text).block(Block::default().title("editor").borders(Borders::ALL)), panes[1]);

    let status = app.current_relative().unwrap_or("no file");
    frame.render_widget(Paragraph::new(status.to_string()), root[1]);
}
