use ideot::app::App;
use ideot::lsp::{CompletionItem, CompletionKind, Location, LspMsg, Position, Range};
use ideot::ui;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::tempdir;

#[test]
fn definition_response_jumps_to_local_file_location() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn target() {}\nfn main() {}").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    let uri = format!("file://{}", dir.path().join("main.rs").display());

    app.apply_lsp_message_for_test(LspMsg::Definition(Some(vec![Location {
        uri,
        range: Range {
            start: Position {
                line: 1,
                character: 3,
            },
            end: Position {
                line: 1,
                character: 7,
            },
        },
    }])));

    assert_eq!(app.editor().unwrap().cursor().line, 1);
    assert_eq!(app.editor().unwrap().cursor().column, 3);
    assert!(app.status_line().contains("definition"));
}

#[test]
fn completion_response_is_rendered() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.apply_lsp_message_for_test(LspMsg::Completion(Some(vec![CompletionItem {
        label: "println!".to_string(),
        kind: CompletionKind::Function,
        detail: Some("macro".to_string()),
    }])));

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let rendered = format!("{:?}", terminal.backend().buffer());

    assert!(rendered.contains("completion"));
    assert!(rendered.contains("println!"));
    assert!(rendered.contains("macro"));
}
