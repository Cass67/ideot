use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ideot::{
    app::{App, FocusPane},
    input::{
        help_modal_key_action, help_modal_mouse_action, EditorViewport, HelpModalAction,
        MouseAction, MouseInputController,
    },
    runtime::{RenderScheduler, TickAction},
    settings::Settings,
    ui,
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let settings = Settings::load()?;
    let mut app = App::new_with_settings(root, settings);
    app.rebuild_index()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        SetCursorStyle::SteadyBar
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape
    )?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut mouse_input = MouseInputController::default();
    let mut scheduler = RenderScheduler::new();
    let mut last_mouse_hover: Option<(std::path::PathBuf, ideot::buffer::Position)> = None;
    let mut pending_mouse_hover: Option<(Instant, std::path::PathBuf, ideot::buffer::Position)> =
        None;
    let mut mouse_entered_hover_panel = false;
    loop {
        if scheduler.tick() == TickAction::Render {
            terminal.draw(|frame| ui::render(frame, app))?;
        }
        if app.should_quit {
            break;
        }
        if let Some((due_at, path, pos)) = pending_mouse_hover.as_ref() {
            if Instant::now() >= *due_at {
                let path = path.clone();
                let pos = *pos;
                pending_mouse_hover = None;
                app.lsp_hover_preview(&path, pos);
                mouse_entered_hover_panel = false;
                scheduler.mark_dirty();
            }
        }
        if !event::poll(Duration::from_millis(16))? {
            if scheduler.should_poll_lsp_after_idle() && app.poll_lsp() {
                scheduler.mark_dirty();
            }
            continue;
        }
        match event::read()? {
            Event::Key(key)
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('q') =>
            {
                app.should_quit = true;
            }
            Event::Key(key)
                if app.help_open()
                    && help_modal_key_action(key.modifiers, key.code) == HelpModalAction::Close =>
            {
                app.toggle_help();
            }
            Event::Key(key) if app.help_open() => match key.code {
                KeyCode::Down => app.scroll_help_down(),
                KeyCode::Up => app.scroll_help_up(),
                KeyCode::PageDown => app.page_help_down(),
                KeyCode::PageUp => app.page_help_up(),
                _ => {}
            },
            Event::Mouse(mouse) if app.help_open() => match mouse.kind {
                MouseEventKind::ScrollDown => app.scroll_help_down(),
                MouseEventKind::ScrollUp => app.scroll_help_up(),
                _ => match help_modal_mouse_action(mouse.kind) {
                    HelpModalAction::Close => app.toggle_help(),
                    HelpModalAction::Ignore => {}
                },
            },
            Event::Key(key) if app.hover_panel_focused() => match key.code {
                KeyCode::Esc => app.close_hover_panel(),
                KeyCode::Down => app.scroll_hover_down(),
                KeyCode::Up => app.scroll_hover_up(),
                KeyCode::PageDown => app.page_hover_down(),
                KeyCode::PageUp => app.page_hover_up(),
                _ => {}
            },
            Event::Mouse(mouse)
                if app.search_open() && mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
            {
                let size = terminal.size()?;
                let area = ui::search_popup_area(Rect::new(0, 0, size.width, size.height));
                if ui::rect_contains(area, mouse.column, mouse.row) {
                    let row = mouse.row.saturating_sub(area.y + 1) as usize;
                    let visible_rows = area.height.saturating_sub(3) as usize;
                    let _ = app.activate_search_visible_row(row, visible_rows);
                }
            }
            Event::Mouse(mouse) if app.hover_panel_focused() => match mouse.kind {
                MouseEventKind::ScrollDown => app.scroll_hover_down(),
                MouseEventKind::ScrollUp => app.scroll_hover_up(),
                _ => {}
            },
            Event::Mouse(mouse) if app.hover_panel_open() => {
                let size = terminal.size()?;
                let hover_area =
                    ui::hover_popup_mouse_region(Rect::new(0, 0, size.width, size.height));
                let over_hover = ui::rect_contains(hover_area, mouse.column, mouse.row);
                if over_hover {
                    mouse_entered_hover_panel = true;
                }
                match mouse.kind {
                    MouseEventKind::ScrollDown if over_hover => app.scroll_hover_down(),
                    MouseEventKind::ScrollUp if over_hover => app.scroll_hover_up(),
                    MouseEventKind::Moved if !over_hover && mouse_entered_hover_panel => {
                        app.close_hover_panel();
                        mouse_entered_hover_panel = false;
                    }
                    _ => {}
                }
            }
            Event::Key(key) => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                    let _ = app.save_current();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('n')) => app.start_new_file_prompt(),
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => app.start_delete_file_prompt(),
                (KeyModifiers::CONTROL, KeyCode::Char('p')) => app.toggle_search(),
                (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                    let _ = app.open_git_browser();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                    let _ = app.toggle_file_pane_visible();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                    let _ = app.toggle_line_numbers_visible();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    let _ = app.toggle_lsp_enabled();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
                    let _ = app.toggle_lsp_hover_enabled();
                }
                (_, KeyCode::F(1)) => app.toggle_help(),
                (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
                    let _ = app.mark_current_file();
                }
                (KeyModifiers::CONTROL, KeyCode::Char(ch)) if ('1'..='9').contains(&ch) => {
                    let slot = ch.to_digit(10).unwrap() as usize;
                    let _ = app.jump_to_mark(slot);
                }
                (_, KeyCode::Esc) if app.search_open() => app.close_search(),
                (_, KeyCode::Esc) if app.file_prompt().is_some() => app.cancel_file_prompt(),
                (_, KeyCode::Esc) if app.git_view().is_some() => app.git_back(),
                (_, KeyCode::Esc) if app.focus_pane() == FocusPane::Explorer => {
                    app.collapse_selected_directory();
                }
                (_, KeyCode::Tab) if app.git_view() == Some(ideot::app::GitView::Diff) => {
                    app.toggle_git_diff_layout()
                }
                (_, KeyCode::Tab) if app.git_view().is_none() => app.toggle_focus_pane(),
                (_, KeyCode::PageDown) if app.git_view() == Some(ideot::app::GitView::Diff) => {
                    app.page_git_diff_down(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::PageUp) if app.git_view() == Some(ideot::app::GitView::Diff) => {
                    app.page_git_diff_up(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::Down) if app.git_view().is_some() => app.git_move_down(),
                (_, KeyCode::Up) if app.git_view().is_some() => app.git_move_up(),
                (_, KeyCode::Enter) if app.git_view().is_some() => {
                    let _ = app.activate_git_selection();
                }
                (_, KeyCode::Down) if app.search_open() => app.move_selection_down(),
                (_, KeyCode::Up) if app.search_open() => app.move_selection_up(),
                (_, KeyCode::PageDown) if app.search_open() => {
                    let amount = terminal.size()?.height.saturating_sub(8) as usize;
                    for _ in 0..amount.max(1) {
                        app.move_selection_down();
                    }
                }
                (_, KeyCode::PageUp) if app.search_open() => {
                    let amount = terminal.size()?.height.saturating_sub(8) as usize;
                    for _ in 0..amount.max(1) {
                        app.move_selection_up();
                    }
                }
                (_, KeyCode::Enter) if app.search_open() => {
                    let _ = app.activate_selected();
                }
                (_, KeyCode::Enter) if app.file_prompt().is_some() => {
                    let _ = app.submit_file_prompt();
                }
                (_, KeyCode::Char('y'))
                    if app.file_prompt() == Some(ideot::app::FilePrompt::Delete) =>
                {
                    let _ = app.confirm_delete_current_file();
                }
                (_, KeyCode::Backspace) if app.file_prompt().is_some() => {
                    app.pop_file_prompt_char()
                }
                (_, KeyCode::Char(ch))
                    if app.file_prompt() == Some(ideot::app::FilePrompt::New) =>
                {
                    app.push_file_prompt_char(ch)
                }
                (_, KeyCode::PageDown) if app.focus_pane() == FocusPane::Explorer => {
                    app.page_explorer_down(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::PageUp) if app.focus_pane() == FocusPane::Explorer => {
                    app.page_explorer_up(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::PageDown) => {
                    let vh = terminal
                        .size()
                        .map(|s| s.height.saturating_sub(4) as usize)
                        .unwrap_or(20);
                    app.page_editor_down(vh);
                    app.scroll_to_cursor(vh);
                }
                (_, KeyCode::PageUp) => {
                    let vh = terminal
                        .size()
                        .map(|s| s.height.saturating_sub(4) as usize)
                        .unwrap_or(20);
                    app.page_editor_up(vh);
                    app.scroll_to_cursor(vh);
                }
                (_, KeyCode::Home) => app.editor_move_line_start(),
                (_, KeyCode::End) => app.editor_move_line_end(),
                (_, KeyCode::Delete) => app.delete_forward(),
                (KeyModifiers::ALT, KeyCode::Left) => app.editor_move_word_left(),
                (KeyModifiers::ALT, KeyCode::Right) => app.editor_move_word_right(),
                // Selection: Shift + arrows (must be before plain arrows)
                (KeyModifiers::SHIFT, KeyCode::Left) => app.extend_selection_left(),
                (KeyModifiers::SHIFT, KeyCode::Right) => app.extend_selection_right(),
                (KeyModifiers::SHIFT, KeyCode::Up) => app.extend_selection_up(),
                (KeyModifiers::SHIFT, KeyCode::Down) => app.extend_selection_down(),
                // Copy selection (Ctrl+Shift+C)
                (KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Char('c')) => {
                    let _ = app.copy_selection();
                }
                // Paste (Ctrl+V or Shift+Insert)
                (KeyModifiers::CONTROL, KeyCode::Char('v'))
                | (KeyModifiers::SHIFT, KeyCode::Insert) => {
                    let _ = app.paste();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('a')) => app.select_all(),
                (_, KeyCode::Char('U')) => app.undo(),
                (KeyModifiers::CONTROL, KeyCode::Char('r')) => app.redo(),
                // Escape clears selection
                (_, KeyCode::Esc) if app.editor().and_then(|e| e.selection()).is_some() => {
                    app.clear_selection();
                }
                (_, KeyCode::Down) if app.focus_pane() == FocusPane::Editor => {
                    let vh = terminal
                        .size()
                        .map(|s| s.height.saturating_sub(4) as usize)
                        .unwrap_or(20);
                    app.editor_move_down();
                    app.scroll_to_cursor(vh);
                }
                (_, KeyCode::Up) if app.focus_pane() == FocusPane::Editor => {
                    let vh = terminal
                        .size()
                        .map(|s| s.height.saturating_sub(4) as usize)
                        .unwrap_or(20);
                    app.editor_move_up();
                    app.scroll_to_cursor(vh);
                }
                (_, KeyCode::Left) if app.focus_pane() == FocusPane::Editor => {
                    app.editor_move_left();
                }
                (_, KeyCode::Right) if app.focus_pane() == FocusPane::Editor => {
                    app.editor_move_right();
                }
                (_, KeyCode::Down) => app.focused_arrow_down(),
                (_, KeyCode::Up) => app.focused_arrow_up(),
                (_, KeyCode::Enter) if app.focus_pane() == FocusPane::Editor => {
                    let vh = terminal
                        .size()
                        .map(|s| s.height.saturating_sub(4) as usize)
                        .unwrap_or(20);
                    app.insert_newline();
                    app.scroll_to_cursor(vh);
                }
                (_, KeyCode::Enter) => {
                    let _ = app.activate_selected();
                }
                (_, KeyCode::Char(' ')) if app.focus_pane() == FocusPane::Explorer => {
                    let _ = app.activate_selected();
                }
                (_, KeyCode::Backspace) if app.search_open() => app.pop_search_char(),
                (_, KeyCode::Backspace) => app.backspace(),
                (_, KeyCode::Char(ch)) if app.search_open() => app.push_search_char(ch),
                (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                    let hover_target = app.editor().and_then(|editor| {
                        editor
                            .buffer()
                            .path()
                            .map(|path| (path.to_path_buf(), editor.cursor()))
                    });
                    if let Some((path, pos)) = hover_target {
                        app.lsp_hover(&path, pos);
                    }
                }
                (KeyModifiers::CONTROL, KeyCode::Char('/')) => {
                    if let Some(editor) = app.editor() {
                        if let Some(path) = editor.buffer().path() {
                            let pos = editor.cursor();
                            app.lsp_completion(path, pos);
                        }
                    }
                }
                (KeyModifiers::CONTROL, KeyCode::Char(']')) => {
                    if let Some(editor) = app.editor() {
                        if let Some(path) = editor.buffer().path() {
                            let pos = editor.cursor();
                            app.lsp_definition(path, pos);
                        }
                    }
                }
                (_, KeyCode::Char('Y')) => {
                    let _ = app.copy_selection();
                }
                (_, KeyCode::Char(ch)) => app.insert_char(ch),
                _ => {}
            },
            Event::Paste(text) => {
                app.insert_pasted_text(text);
            }
            Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let explorer_width = if app.file_pane_visible() {
                    size.width * 30 / 100
                } else {
                    0
                };
                let content_height = size.height.saturating_sub(1);
                let in_content_rows = mouse.row > 0 && mouse.row < content_height.saturating_sub(1);
                let editor_x = explorer_width.saturating_add(1);
                let editor_height = content_height.saturating_sub(1);
                let editor_viewport = EditorViewport {
                    x: editor_x,
                    y: 1,
                    width: size.width.saturating_sub(editor_x),
                    height: editor_height,
                    scroll: app.editor_scroll(),
                };
                let visible_lines: Vec<String> = app
                    .editor()
                    .map(|editor| {
                        editor
                            .buffer()
                            .lines()
                            .iter()
                            .skip(app.editor_scroll())
                            .take(editor_height as usize)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left)
                        if app.git_view() == Some(ideot::app::GitView::Diff) =>
                    {
                        let area = git_overlay_area(size.width, size.height);
                        let before_side = mouse.column < area.x + area.width / 2;
                        let row = mouse.row.saturating_sub(area.y + 1) as usize;
                        app.click_git_diff_row(row, before_side);
                    }
                    MouseEventKind::Down(MouseButton::Left)
                        if mouse.column < explorer_width && in_content_rows =>
                    {
                        let row = mouse.row.saturating_sub(1) as usize;
                        let _ = app.activate_explorer_visible_row(row);
                    }
                    MouseEventKind::Down(MouseButton::Left)
                        if mouse.column > explorer_width && in_content_rows =>
                    {
                        if let Some(action) = mouse_input.left_down_with_gutter(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                            app.editor_gutter_width(),
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left)
                        if mouse.column > explorer_width && in_content_rows =>
                    {
                        if let Some(action) = mouse_input.drag_with_gutter(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                            app.editor_gutter_width(),
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(action) = mouse_input.left_up() {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Moved if mouse.column > explorer_width && in_content_rows => {
                        let hover_target = editor_viewport
                            .position_for_rendered_editor_cell_with_gutter(
                                mouse.column,
                                mouse.row,
                                &visible_lines,
                                app.editor_gutter_width(),
                            )
                            .and_then(|pos| {
                                app.editor().and_then(|editor| {
                                    editor.buffer().path().map(|path| (path.to_path_buf(), pos))
                                })
                            });
                        if let Some((path, pos)) = hover_target {
                            let target = (path.clone(), pos);
                            if last_mouse_hover.as_ref() != Some(&target) {
                                last_mouse_hover = Some(target);
                                pending_mouse_hover =
                                    Some((Instant::now() + Duration::from_millis(350), path, pos));
                            }
                        }
                    }
                    MouseEventKind::Moved if app.hover_panel_open() => {
                        app.close_hover_panel();
                    }
                    MouseEventKind::ScrollDown if app.hover_panel_open() => app.scroll_hover_down(),
                    MouseEventKind::ScrollUp if app.hover_panel_open() => app.scroll_hover_up(),
                    MouseEventKind::ScrollDown
                        if app.git_view() == Some(ideot::app::GitView::Diff) =>
                    {
                        app.scroll_git_diff_down()
                    }
                    MouseEventKind::ScrollUp
                        if app.git_view() == Some(ideot::app::GitView::Diff) =>
                    {
                        app.scroll_git_diff_up()
                    }
                    MouseEventKind::ScrollDown if mouse.column < explorer_width => {
                        app.scroll_explorer_down()
                    }
                    MouseEventKind::ScrollUp if mouse.column < explorer_width => {
                        app.scroll_explorer_up()
                    }
                    MouseEventKind::ScrollDown => app.scroll_editor_down(),
                    MouseEventKind::ScrollUp => app.scroll_editor_up(),
                    _ => {}
                }
            }
            _ => {}
        }

        if app.poll_lsp() {
            scheduler.mark_dirty();
        }
        scheduler.mark_dirty();
    }
    Ok(())
}

fn apply_mouse_action(app: &mut App, action: MouseAction, visible_height: usize) {
    match action {
        MouseAction::MoveCursor(position) => app.move_editor_cursor_to(position),
        MouseAction::UpdateSelection { anchor, end } => app.update_editor_selection(anchor, end),
        MouseAction::FinishSelection => app.finish_editor_selection(),
    }
    app.scroll_to_cursor(visible_height);
}

fn git_overlay_area(width: u16, height: u16) -> Rect {
    let area_width = width * 90 / 100;
    let area_height = height * 80 / 100;
    Rect::new(
        (width - area_width) / 2,
        (height - area_height) / 2,
        area_width,
        area_height,
    )
}
