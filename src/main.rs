use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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
        if let Event::Key(key) = event::read()? {
            match (key.modifiers, key.code) {
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
                (_, KeyCode::Enter) => {
                    let _ = app.open_selected();
                }
                (_, KeyCode::Backspace) if app.search_open() => app.pop_search_char(),
                (_, KeyCode::Backspace) => app.backspace(),
                (_, KeyCode::Char(ch)) if app.search_open() => app.push_search_char(ch),
                (_, KeyCode::Char(ch)) => app.insert_char(ch),
                _ => {}
            }
        }
    }
    Ok(())
}
