use ideot::app::App;
use ideot::buffer::Position;
use ideot::lsp::{HoverInfo, LspMsg};
use tempfile::tempdir;

#[test]
fn mouse_hover_preview_does_not_focus_panel_after_response() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    app.lsp_hover_preview_for_test(Position { line: 0, column: 0 });
    app.apply_lsp_message_for_test(LspMsg::Hover(Some(HoverInfo {
        contents: "preview".to_string(),
        range: None,
    })));

    assert!(app.hover_panel_open());
    assert!(!app.hover_panel_focused());
}

#[test]
fn keyboard_hover_focuses_panel_after_response() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    app.lsp_hover_keyboard_for_test(Position { line: 0, column: 0 });
    app.apply_lsp_message_for_test(LspMsg::Hover(Some(HoverInfo {
        contents: "pinned".to_string(),
        range: None,
    })));

    assert!(app.hover_panel_open());
    assert!(app.hover_panel_focused());
}

#[test]
fn mouse_hover_preview_can_be_closed_when_mouse_moves_away() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.lsp_hover_preview_for_test(Position { line: 0, column: 0 });
    app.apply_lsp_message_for_test(LspMsg::Hover(Some(HoverInfo {
        contents: "preview".to_string(),
        range: None,
    })));

    app.close_hover_panel();

    assert!(!app.hover_panel_open());
}
