use ideot::buffer::{Buffer, Position};
use ideot::editor::Editor;

#[test]
fn selection_bounds_single_line() {
    let buffer = Buffer::from_text("hello world".into());
    let mut editor = Editor::new(buffer);

    // Start selection at column 0
    editor.set_cursor(Position { line: 0, column: 0 });
    editor.start_selection();

    // Extend to column 5
    editor.set_cursor(Position { line: 0, column: 5 });
    editor.extend_selection_to();

    let sel = editor.selection().expect("selection should exist");
    let (start, end) = sel.bounds();
    assert_eq!(start.column, 0);
    assert_eq!(end.column, 5);

    let text = editor.selected_text();
    assert_eq!(text, "hello");
}

#[test]
fn selection_bounds_multi_line() {
    let buffer = Buffer::from_text("first\nsecond\nthird".into());
    let mut editor = Editor::new(buffer);

    // Start at beginning of first line
    editor.set_cursor(Position { line: 0, column: 0 });
    editor.start_selection();

    // Extend to end of second line
    editor.set_cursor(Position { line: 1, column: 6 });
    editor.extend_selection_to();

    let text = editor.selected_text();
    assert_eq!(text, "first\nsecond");
}

#[test]
fn selection_bounds_reverse_direction() {
    let buffer = Buffer::from_text("hello world".into());
    let mut editor = Editor::new(buffer);

    // Start at column 5
    editor.set_cursor(Position { line: 0, column: 5 });
    editor.start_selection();

    // Extend back to column 0
    editor.set_cursor(Position { line: 0, column: 0 });
    editor.extend_selection_to();

    let text = editor.selected_text();
    assert_eq!(text, "hello");
}

#[test]
fn delete_selection_single_line() {
    let buffer = Buffer::from_text("hello world".into());
    let mut editor = Editor::new(buffer);

    // Select "hello "
    editor.set_cursor(Position { line: 0, column: 0 });
    editor.start_selection();
    editor.set_cursor(Position { line: 0, column: 6 });
    editor.extend_selection_to();

    editor.delete_selection();

    assert_eq!(editor.buffer().line(0), Some("world"));
    assert_eq!(editor.selection(), None);
}

#[test]
fn delete_selection_multi_line() {
    let buffer = Buffer::from_text("first\nsecond\nthird".into());
    let mut editor = Editor::new(buffer);

    // Select from start of first line to end of second line
    editor.set_cursor(Position { line: 0, column: 0 });
    editor.start_selection();
    editor.set_cursor(Position { line: 1, column: 6 });
    editor.extend_selection_to();

    editor.delete_selection();

    // After deleting "first\nsecond", we get "\nthird" = ["", "third"]
    assert_eq!(editor.buffer().line_count(), 2);
    assert_eq!(editor.buffer().line(0), Some(""));
    assert_eq!(editor.buffer().line(1), Some("third"));
}

#[test]
fn clear_selection_removes_selection() {
    let buffer = Buffer::from_text("hello".into());
    let mut editor = Editor::new(buffer);

    editor.set_cursor(Position { line: 0, column: 0 });
    editor.start_selection();
    editor.set_cursor(Position { line: 0, column: 3 });
    editor.extend_selection_to();

    assert!(editor.selection().is_some());

    editor.clear_selection();
    assert!(editor.selection().is_none());
}

#[test]
fn select_all_selects_entire_buffer() {
    let buffer = Buffer::from_text("one\ntwo\nthree".into());
    let mut editor = Editor::new(buffer);

    editor.select_all();

    assert_eq!(editor.selected_text(), "one\ntwo\nthree");
    let selection = editor.selection().expect("selection should exist");
    assert_eq!(selection.start, Position { line: 0, column: 0 });
    assert_eq!(selection.end, Position { line: 2, column: 5 });
}

#[test]
fn set_selection_uses_clamped_positions() {
    let buffer = Buffer::from_text("abc\ndef".into());
    let mut editor = Editor::new(buffer);

    editor.set_selection(
        Position { line: 0, column: 1 },
        Position {
            line: 99,
            column: 99,
        },
    );

    assert_eq!(editor.selected_text(), "bc\ndef");
    assert_eq!(editor.cursor(), Position { line: 1, column: 3 });
}

#[test]
fn replace_selection_replaces_multiline_range_and_clears_selection() {
    let buffer = Buffer::from_text("alpha\nbeta\ngamma".into());
    let mut editor = Editor::new(buffer);
    editor.set_selection(
        Position { line: 0, column: 2 },
        Position { line: 1, column: 2 },
    );

    editor.replace_selection("XYZ".to_string());

    assert_eq!(editor.buffer().text(), "alXYZta\ngamma");
    assert_eq!(editor.cursor(), Position { line: 0, column: 5 });
    assert_eq!(editor.selection(), None);
}

#[test]
fn insert_text_in_editor_moves_cursor_to_end_of_inserted_text() {
    let buffer = Buffer::from_text("hello world".into());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(Position { line: 0, column: 5 });

    editor.insert_text("\nsmall".to_string());

    assert_eq!(editor.buffer().text(), "hello\nsmall world");
    assert_eq!(editor.cursor(), Position { line: 1, column: 5 });
}
