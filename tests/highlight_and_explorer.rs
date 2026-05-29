use ideot::app::App;
use ideot::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use ratatui::style::Color;
use tempfile::tempdir;

#[test]
fn rust_highlighter_styles_keywords_and_function_names() {
    let mut highlighter = SimpleTreeSitterHighlighter::default();

    let spans = highlighter.highlight_line(Some("rs"), "fn main() { let x = 1; }");

    assert!(spans.iter().any(|span| span.start == 0 && span.end == 2 && span.style.fg == Some(Color::Magenta)));
    assert!(spans.iter().any(|span| span.start == 3 && span.end == 7 && span.style.fg == Some(Color::Cyan)));
    assert!(spans.iter().any(|span| span.start == 12 && span.end == 15 && span.style.fg == Some(Color::Magenta)));
}

#[test]
fn explorer_entries_default_to_collapsed_directories() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/nested")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("src/nested/lib.rs"), "").unwrap();
    std::fs::write(dir.path().join("README.md"), "readme").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    let labels: Vec<_> = app.explorer_entries().into_iter().map(|entry| entry.label).collect();

    assert!(labels.contains(&"  README.md".to_string()));
    assert!(labels.contains(&"▸ src".to_string()));
    assert!(!labels.iter().any(|label| label.contains("main.rs")));
    assert!(!labels.iter().any(|label| label.contains("nested")));
}

#[test]
fn scroll_offsets_move_independently_for_explorer_and_editor() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\nline2\nline3").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    assert_eq!(app.explorer_scroll(), 0);
    assert_eq!(app.editor_scroll(), 0);

    app.scroll_explorer_down();
    app.scroll_editor_down();
    app.scroll_editor_down();

    assert_eq!(app.explorer_scroll(), 1);
    assert_eq!(app.editor_scroll(), 2);

    app.scroll_explorer_up();
    app.scroll_editor_up();

    assert_eq!(app.explorer_scroll(), 0);
    assert_eq!(app.editor_scroll(), 1);
}
