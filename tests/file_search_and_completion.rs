use ideot::app::App;
use ideot::lsp::{CompletionItem, CompletionKind, LspMsg};
use tempfile::tempdir;

#[test]
fn current_file_search_moves_to_matching_text() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "alpha\nbeta\ngamma beta").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    app.open_file_search();
    app.push_file_search_char('b');
    app.push_file_search_char('e');
    app.push_file_search_char('t');
    app.push_file_search_char('a');
    app.next_file_search_match();

    assert_eq!(app.editor().unwrap().cursor().line, 1);
    assert_eq!(app.editor().unwrap().cursor().column, 0);
}

#[test]
fn accepting_completion_inserts_selected_label() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() { }").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.editor_mut()
        .unwrap()
        .set_cursor(ideot::buffer::Position {
            line: 0,
            column: 12,
        });
    app.apply_lsp_message_for_test(LspMsg::Completion(Some(vec![CompletionItem {
        label: "println!".to_string(),
        kind: CompletionKind::Function,
        detail: None,
    }])));

    app.accept_completion();

    assert!(app.editor().unwrap().buffer().text().contains("println!"));
    assert!(app.completion_popup.is_none());
}
