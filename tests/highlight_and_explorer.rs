use ideot::app::App;
use ideot::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use ideot::ui;
use ratatui::style::Color;
use tempfile::tempdir;

#[test]
fn rust_highlighter_styles_richer_rust_syntax() {
    let mut highlighter = SimpleTreeSitterHighlighter::default();

    let line = "pub struct App { name: String, count: usize }";
    let spans = highlighter.highlight_line(Some("rs"), line);

    assert!(spans.iter().any(|span| &line[span.start..span.end] == "pub" && span.style.fg == Some(Color::Magenta)));
    assert!(spans.iter().any(|span| &line[span.start..span.end] == "struct" && span.style.fg == Some(Color::Magenta)));
    assert!(spans.iter().any(|span| &line[span.start..span.end] == "App" && span.style.fg == Some(Color::Blue)));
    assert!(spans.iter().any(|span| &line[span.start..span.end] == "String" && span.style.fg == Some(Color::Blue)));
    assert!(spans.iter().any(|span| &line[span.start..span.end] == "usize" && span.style.fg == Some(Color::Blue)));
}

#[test]
fn editor_highlighting_is_limited_to_visible_viewport() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("main.rs");
    let text = (0..100).map(|i| format!("fn line_{i}() {{}}" )).collect::<Vec<_>>().join("\n");
    std::fs::write(&path, text).unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    let lines = ui::highlighted_editor_lines_for_height(&app, 12);

    assert_eq!(lines.len(), 12);
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
fn toggling_selected_directory_expands_children_and_files_open() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/nested")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("src/nested/lib.rs"), "").unwrap();
    std::fs::write(dir.path().join("README.md"), "readme").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();

    app.activate_selected().unwrap();
    let labels: Vec<_> = app.explorer_entries().into_iter().map(|entry| entry.label).collect();
    assert!(labels.contains(&"▾ src".to_string()));
    assert!(labels.contains(&"    src/main.rs".to_string()));
    assert!(labels.contains(&"  ▸ src/nested".to_string()));

    app.move_selection_down();
    app.activate_selected().unwrap();
    assert_eq!(app.current_relative(), Some("src/main.rs"));
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
