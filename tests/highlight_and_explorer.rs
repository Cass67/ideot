use ideot::app::{App, FocusPane};
use ideot::highlight::{Highlighter, SimpleTreeSitterHighlighter};
use ideot::ui;
use ratatui::{layout::Rect, style::Color};
use tempfile::tempdir;

#[test]
fn rust_highlighter_styles_richer_rust_syntax() {
    let mut highlighter = SimpleTreeSitterHighlighter::default();

    let line = "pub struct App { name: String, count: usize }";
    let spans = highlighter.highlight_line(Some("rs"), line);

    assert_styled(&spans, line, "pub", Color::Magenta);
    assert_styled(&spans, line, "struct", Color::Magenta);
    assert_styled(&spans, line, "App", Color::Blue);
    assert_styled(&spans, line, "String", Color::Blue);
    assert_styled(&spans, line, "usize", Color::Blue);
}

#[test]
fn highlighter_supports_go_python_javascript_toml_yaml_and_markdown() {
    let mut highlighter = SimpleTreeSitterHighlighter::default();

    let go = "func main() { var name string }";
    assert_styled(
        &highlighter.highlight_line(Some("go"), go),
        go,
        "func",
        Color::Magenta,
    );
    assert_styled(
        &highlighter.highlight_line(Some("go"), go),
        go,
        "main",
        Color::Cyan,
    );

    let python = "def main(): return 42";
    assert_styled(
        &highlighter.highlight_line(Some("py"), python),
        python,
        "def",
        Color::Magenta,
    );
    assert_styled(
        &highlighter.highlight_line(Some("py"), python),
        python,
        "main",
        Color::Cyan,
    );

    let js = "function main() { const x = 1 }";
    assert_styled(
        &highlighter.highlight_line(Some("js"), js),
        js,
        "function",
        Color::Magenta,
    );
    assert_styled(
        &highlighter.highlight_line(Some("js"), js),
        js,
        "main",
        Color::Cyan,
    );

    let toml = "name = \"ideot\"";
    assert_styled(
        &highlighter.highlight_line(Some("toml"), toml),
        toml,
        "name",
        Color::Cyan,
    );
    assert_styled(
        &highlighter.highlight_line(Some("toml"), toml),
        toml,
        "\"ideot\"",
        Color::Green,
    );

    let yaml = "name: ideot";
    assert_styled(
        &highlighter.highlight_line(Some("yaml"), yaml),
        yaml,
        "name",
        Color::Cyan,
    );
    assert_styled(
        &highlighter.highlight_line(Some("yaml"), yaml),
        yaml,
        "ideot",
        Color::Green,
    );

    let markdown = "# Heading";
    assert_styled(
        &highlighter.highlight_line(Some("md"), markdown),
        markdown,
        "#",
        Color::Magenta,
    );
    assert_styled(
        &highlighter.highlight_line(Some("md"), markdown),
        markdown,
        "Heading",
        Color::Cyan,
    );
}

fn assert_styled(spans: &[ideot::highlight::HighlightSpan], line: &str, text: &str, color: Color) {
    assert!(
        spans
            .iter()
            .any(|span| &line[span.start..span.end] == text && span.style.fg == Some(color)),
        "expected {text:?} in {line:?} to be styled {color:?}; spans: {spans:?}"
    );
}

#[test]
fn editor_highlighting_is_limited_to_visible_viewport() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("main.rs");
    let text = (0..100)
        .map(|i| format!("fn line_{i}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
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
    let labels: Vec<_> = app
        .explorer_entries()
        .into_iter()
        .map(|entry| entry.label)
        .collect();

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
    let labels: Vec<_> = app
        .explorer_entries()
        .into_iter()
        .map(|entry| entry.label)
        .collect();
    assert!(labels.contains(&"▾ src".to_string()));
    assert!(labels.contains(&"    src/main.rs".to_string()));
    assert!(labels.contains(&"  ▸ src/nested".to_string()));

    app.move_selection_down();
    app.activate_selected().unwrap();
    assert_eq!(app.current_relative(), Some("src/main.rs"));
}

#[test]
fn help_overlay_toggles_without_question_mark_binding() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    assert!(!app.help_open());
    app.toggle_help();
    assert!(app.help_open());
    app.toggle_help();
    assert!(!app.help_open());
}

#[test]
fn editor_cursor_maps_to_screen_position_when_visible() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "abc\ndef").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.place_editor_cursor(1, 2);

    let area = Rect::new(30, 0, 70, 20);
    assert_eq!(ui::editor_cursor_screen_position(&app, area), Some((42, 2)));
}

#[test]
fn mouse_activation_opens_tree_files_and_places_editor_cursor() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "abc\ndef").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();

    app.activate_explorer_visible_row(0).unwrap();
    assert!(app
        .explorer_entries()
        .iter()
        .any(|entry| entry.label == "    src/main.rs"));

    app.activate_explorer_visible_row(1).unwrap();
    assert_eq!(app.current_relative(), Some("src/main.rs"));

    app.place_editor_cursor(1, 2);
    assert_eq!(app.editor().unwrap().cursor().line, 1);
    assert_eq!(app.editor().unwrap().cursor().column, 2);
}

#[test]
fn tab_toggles_focus_between_file_panel_and_editor_panel() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    assert_eq!(app.focus_pane(), FocusPane::Explorer);
    app.toggle_focus_pane();
    assert_eq!(app.focus_pane(), FocusPane::Editor);
    app.toggle_focus_pane();
    assert_eq!(app.focus_pane(), FocusPane::Explorer);
}

#[test]
fn arrow_scroll_respects_focused_pane() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\nline2\nline3").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    assert_eq!(app.focus_pane(), FocusPane::Explorer);
    app.focused_arrow_down();
    assert_eq!(app.selected_file(), 0);
    assert_eq!(app.editor_scroll(), 0);

    app.toggle_focus_pane();
    app.focused_arrow_down();
    app.focused_arrow_down();
    assert_eq!(app.editor_scroll(), 2);
    app.focused_arrow_up();
    assert_eq!(app.editor_scroll(), 1);
}

#[test]
fn page_scroll_moves_explorer_and_editor_by_requested_amount() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.rs"),
        "fn main() {}\nline2\nline3\nline4\nline5",
    )
    .unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    app.page_editor_down(3);
    assert_eq!(app.editor_scroll(), 3);
    app.page_editor_up(2);
    assert_eq!(app.editor_scroll(), 1);

    app.page_explorer_down(4);
    assert_eq!(app.explorer_scroll(), 4);
    app.page_explorer_up(3);
    assert_eq!(app.explorer_scroll(), 1);
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
