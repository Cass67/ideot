use crate::app::{App, FocusPane, GitDiffLayout, GitView};
use crate::git::DiffKind;
use crate::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct HighlightCacheKey {
    relative: Option<String>,
    text_hash: u64,
    scroll: usize,
    height: usize,
    selection: Option<(usize, usize, usize, usize)>,
    line_numbers_visible: bool,
    diagnostics_visible: bool,
    diagnostic_hash: u64,
}

thread_local! {
    static HIGHLIGHTER: RefCell<SimpleTreeSitterHighlighter> = RefCell::new(SimpleTreeSitterHighlighter::default());
    static HIGHLIGHT_CACHE: RefCell<Option<(HighlightCacheKey, Vec<Line<'static>>)>> = const { RefCell::new(None) };
}

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(frame.area());
    let panes = if app.file_pane_visible() {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(root[0])
    } else {
        vec![Rect::default(), root[0]].into()
    };

    if app.file_pane_visible() {
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
        let file_border = if app.focus_pane() == FocusPane::Explorer {
            Color::Blue
        } else {
            Color::White
        };
        frame.render_widget(
            List::new(files).block(
                Block::default()
                    .title("files")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(file_border)),
            ),
            panes[0],
        );
    }

    let editor_lines =
        highlighted_editor_lines_for_height(app, panes[1].height.saturating_sub(2) as usize);
    let editor_border = if app.focus_pane() == FocusPane::Editor {
        Color::Blue
    } else {
        Color::White
    };
    frame.render_widget(
        Paragraph::new(editor_lines).block(
            Block::default()
                .title("editor")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(editor_border)),
        ),
        panes[1],
    );
    if let Some((x, y)) = editor_cursor_screen_position(app, panes[1]) {
        frame.set_cursor_position((x, y));
    }

    if app.search_open() {
        let area = search_popup_area(frame.area());
        frame.render_widget(Clear, area);
        let visible_rows = area.height.saturating_sub(3) as usize;
        let selected = app.selected_file();
        let start = selected.saturating_sub(visible_rows.saturating_sub(1));
        let results: Vec<ListItem> = app
            .search(app.search_query())
            .into_iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
            .map(|(index, file)| {
                search_result_item(
                    app.search_query(),
                    index == app.selected_file(),
                    &file.relative,
                )
            })
            .collect();
        let total = app.search(app.search_query()).len();
        frame.render_widget(
            List::new(results).block(
                Block::default()
                    .title(format!(
                        "Find file: {} · {total} results · Esc close",
                        app.search_query()
                    ))
                    .borders(Borders::ALL),
            ),
            area,
        );
    }

    if app.file_prompt().is_some() {
        render_file_prompt(frame, app);
    }

    if app.file_search_open() {
        render_file_search_popup(frame, app);
    }

    if app.completion_popup.is_some() {
        render_completion_popup(frame, app);
    }

    if app.diagnostics_panel_open() {
        render_diagnostics_panel(frame, app);
    }

    if app.git_view().is_some() {
        render_git_overlay(frame, app);
    }

    if app.hover_popup.is_some() && !app.help_open() {
        render_hover_popup(frame, app);
    }

    if app.help_open() {
        render_help_overlay(frame, app);
    }

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(root[1]);
    frame.render_widget(
        Paragraph::new(footer_shortcuts(app.git_view().is_some())),
        footer[0],
    );
    frame.render_widget(Paragraph::new(app.status_line()), footer[1]);
}

fn search_result_item(query: &str, selected: bool, path: &str) -> ListItem<'static> {
    let prefix = if selected { "> " } else { "  " };
    let query = query.trim();
    if query.is_empty() {
        return ListItem::new(format!("{prefix}{path}"));
    }
    let haystack = path.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    let Some(start) = haystack.find(&needle) else {
        return ListItem::new(format!("{prefix}{path}"));
    };
    let end = start + needle.len();
    let mut spans = vec![Span::raw(prefix.to_string())];
    if start > 0 {
        spans.push(Span::raw(path[..start].to_string()));
    }
    spans.push(Span::styled(
        path[start..end].to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    if end < path.len() {
        spans.push(Span::raw(path[end..].to_string()));
    }
    ListItem::new(Line::from(spans))
}

fn render_file_search_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 20, frame.area());
    frame.render_widget(Clear, area);
    let count = app.file_search_match_count();
    frame.render_widget(
        Paragraph::new(format!("Find in file: {}", app.file_search_query())).block(
            Block::default()
                .title(format!(
                    "file search · {count} matches · Enter next · Esc close"
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
    );
}

fn render_diagnostics_panel(frame: &mut Frame, app: &App) {
    let area = centered_rect(75, 55, frame.area());
    frame.render_widget(Clear, area);
    let rows: Vec<ListItem> = app
        .current_file_diagnostics()
        .iter()
        .enumerate()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(index, diagnostic)| {
            let prefix = if index == app.selected_diagnostic() {
                "> "
            } else {
                "  "
            };
            let severity = diagnostic
                .severity
                .map(|severity| format!("{severity:?}"))
                .unwrap_or_else(|| "Info".to_string());
            ListItem::new(format!(
                "{prefix}line {} · {severity} · {}",
                diagnostic.range.start.line + 1,
                diagnostic.message
            ))
        })
        .collect();
    frame.render_widget(
        List::new(rows).block(
            Block::default()
                .title("diagnostics · Enter jump · Esc close")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
    );
}

fn render_completion_popup(frame: &mut Frame, app: &App) {
    let Some(items) = &app.completion_popup else {
        return;
    };
    let area = centered_rect(50, 35, frame.area());
    frame.render_widget(Clear, area);
    let rows: Vec<ListItem> = items
        .iter()
        .enumerate()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(index, item)| {
            let prefix = if index == app.completion_selected() {
                "> "
            } else {
                "  "
            };
            let detail = item.detail.as_deref().unwrap_or("");
            if detail.is_empty() {
                ListItem::new(format!("{prefix}{}", item.label))
            } else {
                ListItem::new(format!("{prefix}{} — {detail}", item.label))
            }
        })
        .collect();
    frame.render_widget(
        List::new(rows).block(
            Block::default()
                .title("completion")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
    );
}

fn render_hover_popup(frame: &mut Frame, app: &App) {
    let Some(hover) = &app.hover_popup else {
        return;
    };
    let area = hover_popup_area(frame.area());
    frame.render_widget(Clear, area);
    let text = hover
        .contents
        .lines()
        .skip(app.hover_scroll)
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }).block(
            Block::default()
                .title("hover · wheel/↑↓ scroll · Esc close")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
    );
}

pub fn search_popup_area(area: Rect) -> Rect {
    centered_rect(70, 45, area)
}

pub fn hover_popup_area(area: Rect) -> Rect {
    centered_rect(80, 70, area)
}

pub fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

pub fn hover_popup_mouse_region(area: Rect) -> Rect {
    let popup = hover_popup_area(area);
    Rect::new(
        popup.x.saturating_sub(2),
        popup.y.saturating_sub(2),
        popup.width.saturating_add(4),
        popup.height.saturating_add(4),
    )
}

fn render_file_prompt(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);
    let text = match app.file_prompt() {
        Some(crate::app::FilePrompt::New) => format!("New file path: {}", app.file_prompt_input()),
        Some(crate::app::FilePrompt::Delete) => format!(
            "Delete {}? y/N",
            app.current_relative().unwrap_or("current file")
        ),
        None => String::new(),
    };
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title("file action")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
    );
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
    match app.git_diff_layout() {
        GitDiffLayout::Split => render_split_git_diff(frame, app, area),
        GitDiffLayout::Unified => render_unified_git_diff(frame, app, area),
    }
}

fn render_split_git_diff(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_unified_git_diff(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let height = area.height.saturating_sub(2) as usize;
    let rows: Vec<Line> = app
        .git_unified_diff_rows()
        .into_iter()
        .skip(app.git_diff_scroll())
        .take(height)
        .map(|row| {
            let style = match row.prefix {
                '-' => Style::default().fg(Color::Red),
                '+' => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::DarkGray),
            };
            let old = row
                .old_line
                .map(|line| line.to_string())
                .unwrap_or_else(|| " ".to_string());
            let new = row
                .new_line
                .map(|line| line.to_string())
                .unwrap_or_else(|| " ".to_string());
            Line::from(vec![
                Span::styled(format!("{old:>4} "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{new:>4} "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} {}", row.prefix, row.text), style),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(rows).block(
            Block::default()
                .title("unified diff")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        ),
        area,
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

pub fn footer_shortcuts(_git_open: bool) -> &'static str {
    "F1 Help · Ctrl-Q Quit"
}

pub fn help_text_lines() -> Vec<&'static str> {
    vec![
        "Keyboard",
        "  Help is modal; press F1 or Esc to return to the editor",
        "  Ctrl-W       Toggle focus between explorer and editor",
        "  Tab          Editor: insert indent",
        "  Arrows       Navigate focused pane / move cursor in editor",
        "  Page Up/Down Scroll focused pane",
        "  Enter        New line (editor) / Open file (explorer/git)",
        "  Space        Open file or toggle folder (explorer)",
        "  Ctrl-S       Save current file",
        "  Ctrl-Q       Quit",
        "",
        "Editing",
        "  Home/End     Move to start/end of line",
        "  Delete       Delete character after cursor",
        "  Alt←/Alt→    Move by word",
        "  Shift+Arrows Select text",
        "  Ctrl-A       Select all text in current file",
        "  Y            Copy selection",
        "  Ctrl+Shift+C Copy selection",
        "  Ctrl+V       Paste",
        "  Ctrl-F       Search in current file",
        "  U            Undo",
        "  Ctrl-R       Redo",
        "  Esc          Clear selection / close overlay",
        "",
        "Project",
        "  Ctrl-P       Search files",
        "  Ctrl-B       Toggle file pane on/off (remembered)",
        "  Ctrl-T       Toggle line numbers on/off (remembered)",
        "  Ctrl-N       New file",
        "  Ctrl-D       Delete file",
        "  Ctrl-M       Mark current file",
        "  Ctrl-1..9    Jump to mark",
        "  Ctrl-G       Git commit browser",
        "",
        "Git Browser",
        "  Enter        Select commit/file",
        "  Tab          Toggle split/unified diff",
        "  Esc          Back/close git browser",
        "",
        "LSP",
        "  Ctrl-L       Toggle LSP on/off (remembered)",
        "  Ctrl-O       Toggle LSP hover on/off (remembered)",
        "  Ctrl-U       Toggle LSP diagnostics on/off (remembered)",
        "  Ctrl-H       LSP hover",
        "  Ctrl-/       LSP completion",
        "  Ctrl-]       LSP go to definition",
        "  F8/Shift-F8 Next/previous diagnostic",
        "  Ctrl-E       Toggle diagnostics panel",
        "",
        "Mouse",
        "  Click tree row     Open file or toggle folder",
        "  Click editor pane  Move cursor",
        "  Hover editor text LSP hover",
        "  Drag editor text  Select text in ideot",
        "  Modifier-drag     Terminal-native selection escape hatch",
        "  Wheel              Scroll hovered pane",
        "",
        "Footer",
        "  F1 Help      Show this command reference",
    ]
}

fn render_help_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, frame.area());
    let help: Vec<Line> = help_text_lines()
        .into_iter()
        .skip(app.help_scroll())
        .map(Line::from)
        .collect();
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(help).block(
            Block::default()
                .title("help · F1/Esc close · ↑↓ scroll")
                .borders(Borders::ALL),
        ),
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
    let gutter_width = app.editor_gutter_width() as usize;
    let text_width = inner_width.saturating_sub(gutter_width);
    let column = cursor.column.min(text_width);
    Some((
        inner_x + gutter_width as u16 + column as u16,
        inner_y + visible_line as u16,
    ))
}

pub fn highlighted_editor_lines_for_height(app: &App, height: usize) -> Vec<Line<'static>> {
    let Some(editor) = app.editor() else {
        return vec![Line::from(
            "Open a file with Ctrl-P or select from explorer",
        )];
    };
    let selection = editor.selection();
    let scroll = app.editor_scroll();
    let key = highlight_cache_key(app, height);
    if let Some(cached) = HIGHLIGHT_CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .and_then(|(cached_key, lines)| (cached_key == &key).then(|| lines.clone()))
    }) {
        return cached;
    }
    let lines: Vec<Line<'static>> = HIGHLIGHTER.with(|highlighter| {
        let mut highlighter = highlighter.borrow_mut();
        editor
            .buffer()
            .lines()
            .iter()
            .enumerate()
            .skip(app.editor_scroll())
            .take(height)
            .map(|(line_idx, line)| {
                let visible_line = line_idx - scroll;
                let mut rendered = highlighted_line_with_selection(
                    &mut *highlighter,
                    app.language_hint(),
                    line,
                    line_idx,
                    visible_line,
                    selection,
                );
                let prefix = if app.line_numbers_visible() {
                    app.diagnostic_gutter_for_line(line_idx)
                } else {
                    app.compact_diagnostic_gutter_for_line(line_idx)
                };
                rendered.spans.insert(0, Span::raw(prefix));
                rendered
            })
            .collect()
    });
    HIGHLIGHT_CACHE.with(|cache| *cache.borrow_mut() = Some((key, lines.clone())));
    lines
}

fn highlight_cache_key(app: &App, height: usize) -> HighlightCacheKey {
    let mut text_hasher = DefaultHasher::new();
    if let Some(editor) = app.editor() {
        editor.buffer().text().hash(&mut text_hasher);
    }
    let mut diagnostic_hasher = DefaultHasher::new();
    for diagnostic in app.displayed_file_diagnostics() {
        diagnostic.range.start.line.hash(&mut diagnostic_hasher);
        diagnostic
            .range
            .start
            .character
            .hash(&mut diagnostic_hasher);
        diagnostic.message.hash(&mut diagnostic_hasher);
    }
    let selection = app.editor().and_then(|editor| {
        editor.selection().map(|selection| {
            (
                selection.start.line,
                selection.start.column,
                selection.end.line,
                selection.end.column,
            )
        })
    });
    HighlightCacheKey {
        relative: app.current_relative().map(ToOwned::to_owned),
        text_hash: text_hasher.finish(),
        scroll: app.editor_scroll(),
        height,
        selection,
        line_numbers_visible: app.line_numbers_visible(),
        diagnostics_visible: app.lsp_diagnostics_visible(),
        diagnostic_hash: diagnostic_hasher.finish(),
    }
}

fn highlighted_line_with_selection(
    highlighter: &mut dyn Highlighter,
    language_hint: Option<&str>,
    line: &str,
    line_idx: usize,
    _visible_line: usize,
    selection: Option<crate::editor::Selection>,
) -> Line<'static> {
    let spans = highlighter.highlight_line(language_hint, line);
    if spans.is_empty() && selection.is_none() {
        return Line::from(line.to_string());
    }

    // Calculate selection range for this line
    let sel_start = selection.and_then(|s| {
        let (start, end) = s.bounds();
        if line_idx >= start.line && line_idx <= end.line {
            Some((
                if line_idx == start.line {
                    start.column
                } else {
                    0
                },
                if line_idx == end.line {
                    end.column
                } else {
                    line.len()
                },
            ))
        } else {
            None
        }
    });

    struct SpanInfo {
        start: usize,
        end: usize,
        style: Style,
    }

    let mut all_spans: Vec<SpanInfo> = spans
        .into_iter()
        .filter(|s| s.start < line.len() && s.end <= line.len() && s.start < s.end)
        .map(|s| SpanInfo {
            start: s.start,
            end: s.end,
            style: s.style,
        })
        .collect();
    all_spans.sort_by_key(|s| s.start);

    let sel_style = Style::default().bg(Color::Blue).fg(Color::White);
    let mut rendered = Vec::new();
    let mut pos = 0usize;

    for span in &all_spans {
        if span.start > pos {
            push_sel_split(
                &mut rendered,
                &line[pos..span.start],
                pos,
                Style::default(),
                sel_start,
                sel_style,
            );
        }
        push_sel_split(
            &mut rendered,
            &line[span.start..span.end],
            span.start,
            span.style,
            sel_start,
            sel_style,
        );
        pos = span.end;
    }
    if pos < line.len() {
        push_sel_split(
            &mut rendered,
            &line[pos..],
            pos,
            Style::default(),
            sel_start,
            sel_style,
        );
    }

    Line::from(rendered)
}

fn push_sel_split(
    out: &mut Vec<Span<'static>>,
    text: &str,
    offset: usize,
    base: Style,
    sel: Option<(usize, usize)>,
    sel_style: Style,
) {
    if text.is_empty() {
        return;
    }
    let end = offset + text.len();
    let Some((ss, se)) = sel else {
        out.push(Span::styled(text.to_string(), base));
        return;
    };
    let pre_end = ss.min(end);
    if offset < pre_end {
        out.push(Span::styled(text[..pre_end - offset].to_string(), base));
    }
    let in_start = ss.max(offset);
    let in_end = se.min(end);
    if in_start < in_end {
        out.push(Span::styled(
            text[in_start - offset..in_end - offset].to_string(),
            sel_style,
        ));
    }
    let post_start = se.max(offset);
    if post_start < end {
        out.push(Span::styled(text[post_start - offset..].to_string(), base));
    }
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
