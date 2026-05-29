use crate::app::{App, GitView};
use crate::git::DiffKind;
use crate::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
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
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(text).block(Block::default().title("search").borders(Borders::ALL)),
            area,
        );
    }

    if app.git_view().is_some() {
        render_git_overlay(frame, app);
    }

    if app.help_open() {
        render_help_overlay(frame);
    }

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(root[1]);
    let shortcuts = if app.git_view().is_some() {
        "Git: Enter Select · Esc Back · Up/Down Move · F1 Help"
    } else {
        "Ctrl-P Search · Enter/Space Open/Expand · Ctrl-S Save · Ctrl-G Git · F1 Help · Ctrl-Q Quit"
    };
    frame.render_widget(Paragraph::new(shortcuts), footer[0]);
    let status = app.current_relative().unwrap_or("no file");
    frame.render_widget(Paragraph::new(status.to_string()), footer[1]);
}

fn render_git_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(90, 80, frame.area());
    frame.render_widget(Clear, area);
    match app.git_view() {
        Some(GitView::Commits) => {
            let rows: Vec<ListItem> = app
                .git_commits()
                .iter()
                .enumerate()
                .map(|(index, commit)| {
                    let prefix = if index == app.git_selected_index() {
                        "> "
                    } else {
                        "  "
                    };
                    ListItem::new(format!("{prefix}{} {}", commit.hash, commit.summary))
                })
                .collect();
            frame.render_widget(
                List::new(rows).block(
                    Block::default()
                        .title("git commits")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                ),
                area,
            );
        }
        Some(GitView::Files) => {
            let rows: Vec<ListItem> = app
                .git_files()
                .iter()
                .enumerate()
                .map(|(index, file)| {
                    let prefix = if index == app.git_selected_index() {
                        "> "
                    } else {
                        "  "
                    };
                    ListItem::new(format!("{prefix}{file}"))
                })
                .collect();
            frame.render_widget(
                List::new(rows).block(
                    Block::default()
                        .title("changed files")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                ),
                area,
            );
        }
        Some(GitView::Diff) => render_git_diff(frame, app, area),
        None => {}
    }
}

fn render_git_diff(frame: &mut Frame, app: &App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    frame.render_widget(Clear, area);
    let height = area.height.saturating_sub(2) as usize;
    let before: Vec<Line> = app
        .git_diff_rows()
        .iter()
        .enumerate()
        .skip(app.git_diff_scroll())
        .take(height)
        .map(|(index, row)| {
            diff_line(
                row.before.as_deref().unwrap_or(""),
                row.kind,
                true,
                app.git_diff_selected_row() == Some(index)
                    && app.git_diff_selected_side() == Some(true),
            )
        })
        .collect();
    let after: Vec<Line> = app
        .git_diff_rows()
        .iter()
        .enumerate()
        .skip(app.git_diff_scroll())
        .take(height)
        .map(|(index, row)| {
            diff_line(
                row.after.as_deref().unwrap_or(""),
                row.kind,
                false,
                app.git_diff_selected_row() == Some(index)
                    && app.git_diff_selected_side() == Some(false),
            )
        })
        .collect();
    let border = Style::default().fg(Color::Blue);
    frame.render_widget(
        Paragraph::new(before).block(
            Block::default()
                .title("before")
                .borders(Borders::ALL)
                .border_style(border),
        ),
        panes[0],
    );
    frame.render_widget(
        Paragraph::new(after).block(
            Block::default()
                .title("after")
                .borders(Borders::ALL)
                .border_style(border),
        ),
        panes[1],
    );
}

fn diff_line(text: &str, kind: DiffKind, before: bool, selected: bool) -> Line<'static> {
    let mut style = match (kind, before) {
        (DiffKind::Delete, true) => Style::default().fg(Color::Red),
        (DiffKind::Add, false) => Style::default().fg(Color::Green),
        (DiffKind::Equal, _) => Style::default(),
        _ => Style::default().fg(Color::DarkGray),
    };
    if selected {
        style = style.bg(Color::Blue);
    }
    Line::from(Span::styled(text.to_string(), style))
}

fn render_help_overlay(frame: &mut Frame) {
    let area = centered_rect(70, 55, frame.area());
    let help = vec![
        Line::from("Keyboard"),
        Line::from("  Ctrl-P       Search files"),
        Line::from("  Enter/Space  Open file or expand/collapse folder"),
        Line::from("  Ctrl-S       Save current file"),
        Line::from("  Ctrl-M       Mark current file"),
        Line::from("  Ctrl-1..9    Jump to mark"),
        Line::from("  Ctrl-G       Git commit browser"),
        Line::from("  F1           Toggle this help"),
        Line::from("  Ctrl-Q       Quit"),
        Line::from(""),
        Line::from("Mouse"),
        Line::from("  Click tree row     Open file or toggle folder"),
        Line::from("  Click editor pane  Move cursor"),
        Line::from("  Wheel              Scroll hovered pane"),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(help).block(Block::default().title("help").borders(Borders::ALL)),
        area,
    );
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
