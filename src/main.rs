use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ideot::{app::App, ui};
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
        SetCursorStyle::SteadyBar
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape
    )?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if app.should_quit {
            break;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => app.should_quit = true,
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                    let _ = app.save_current();
                }
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
                (_, KeyCode::PageDown) if terminal.size()?.width > 0 => {
                    app.page_editor_down(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::PageUp) if terminal.size()?.width > 0 => {
                    app.page_editor_up(terminal.size()?.height.saturating_sub(4) as usize)
                }
                (_, KeyCode::Down) => app.move_selection_down(),
                (_, KeyCode::Up) => app.move_selection_up(),
                (_, KeyCode::Enter) | (_, KeyCode::Char(' ')) => {
                    let _ = app.activate_selected();
                }
                (_, KeyCode::Backspace) if app.search_open() => app.pop_search_char(),
                (_, KeyCode::Backspace) => app.backspace(),
                (_, KeyCode::Char(ch)) if app.search_open() => app.push_search_char(ch),
                (_, KeyCode::Char(ch)) => app.insert_char(ch),
                _ => {}
            },
            Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let explorer_width = size.width * 30 / 100;
                let content_height = size.height.saturating_sub(1);
                let in_content_rows = mouse.row > 0 && mouse.row < content_height.saturating_sub(1);
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
                        let row = mouse.row.saturating_sub(1) as usize;
                        let column = mouse.column.saturating_sub(explorer_width + 1) as usize;
                        app.place_editor_cursor(row, column);
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
    }
    Ok(())
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
