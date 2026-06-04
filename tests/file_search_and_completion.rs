use ideot::app::App;
use ideot::lsp::{CompletionItem, CompletionKind, LspMsg};
use tempfile::tempdir;

#[test]
fn current_file_search_is_case_insensitive() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "use std::path::PathBuf;").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    app.open_file_search();
    for ch in "Pathbuf".chars() {
        app.push_file_search_char(ch);
    }
    app.next_file_search_match();

    assert_eq!(app.editor().unwrap().cursor().column, 15);
}

#[test]
fn current_file_search_moves_to_matching_text_and_reveals_it() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.rs"),
        (0..80)
            .map(|line| {
                if line == 70 {
                    "needle".to_string()
                } else {
                    format!("line {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    app.open_file_search();
    for ch in "needle".chars() {
        app.push_file_search_char(ch);
    }
    app.next_file_search_match();

    assert_eq!(app.editor().unwrap().cursor().line, 70);
    assert_eq!(app.editor().unwrap().cursor().column, 0);
    assert_eq!(app.editor_scroll(), 70);
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
