use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ideot::{app::App, ui};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let mut app = App::new(root);
    app.rebuild_index()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
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
                (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
                    let _ = app.mark_current_file();
                }
                (KeyModifiers::CONTROL, KeyCode::Char(ch)) if ('1'..='9').contains(&ch) => {
                    let slot = ch.to_digit(10).unwrap() as usize;
                    let _ = app.jump_to_mark(slot);
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
