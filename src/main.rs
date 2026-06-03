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
    ui,
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let mut app = App::new(root);
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
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if app.should_quit {
            break;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            Event::Key(key)
                if app.help_open()
                    && help_modal_key_action(key.modifiers, key.code) == HelpModalAction::Close =>
            {
                app.toggle_help();
            }
            Event::Key(_) if app.help_open() => {}
            Event::Mouse(mouse) if app.help_open() => match help_modal_mouse_action(mouse.kind) {
                HelpModalAction::Close => app.toggle_help(),
                HelpModalAction::Ignore => {}
            },
            Event::Key(key) => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => app.should_quit = true,
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                    let _ = app.save_current();
                }
                (KeyModifiers::CONTROL, KeyCode::Char('n')) => app.start_new_file_prompt(),
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => app.start_delete_file_prompt(),
                (KeyModifiers::CONTROL, KeyCode::Char('p')) => app.toggle_search(),
                (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                    let _ = app.open_git_browser();
                }
                (_, KeyCode::F(1)) => app.toggle_help(),
                (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
                    let _ = app.mark_current_file();
                }
                (KeyModifiers::CONTROL, KeyCode::Char(ch)) if ('1'..='9').contains(&ch) => {
                    let slot = ch.to_digit(10).unwrap() as usize;
                    let _ = app.jump_to_mark(slot);
                }
                (_, KeyCode::Esc) if app.git_view().is_some() => app.git_back(),
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
                (_, KeyCode::Esc) if app.file_prompt().is_some() => app.cancel_file_prompt(),
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
                    if let Some(editor) = app.editor() {
                        if let Some(path) = editor.buffer().path() {
                            let pos = editor.cursor();
                            app.lsp_hover(path, pos);
                        }
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
                let explorer_width = size.width * 30 / 100;
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
                        if let Some(action) = mouse_input.left_down(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left)
                        if mouse.column > explorer_width && in_content_rows =>
                    {
                        if let Some(action) = mouse_input.drag(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(action) = mouse_input.left_up() {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
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

        // Poll LSP messages (non-blocking)
        app.poll_lsp();
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
