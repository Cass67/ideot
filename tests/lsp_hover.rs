use ideot::app::App;
use ideot::lsp::{HoverInfo, LspMsg};
use ideot::ui;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::tempdir;

#[test]
fn poll_lsp_hover_none_sets_visible_status() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    app.apply_lsp_message_for_test(LspMsg::Hover(None));

    assert!(app.status_line().contains("no hover info"));
}

#[test]
fn hover_panel_scrolls_rendered_text() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.apply_lsp_message_for_test(LspMsg::Hover(Some(HoverInfo {
        contents: "first\nsecond".to_string(),
        range: None,
    })));
    app.scroll_hover_down();

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let rendered = format!("{:?}", terminal.backend().buffer());

    assert!(rendered.contains("second"));
}

#[test]
fn hover_popup_text_is_rendered_when_present() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.apply_lsp_message_for_test(LspMsg::Hover(Some(HoverInfo {
        contents: "fn main() -> ()".to_string(),
        range: None,
    })));

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let rendered = format!("{:?}", terminal.backend().buffer());

    assert!(rendered.contains("hover"));
    assert!(rendered.contains("fn main() -> ()"));
}
