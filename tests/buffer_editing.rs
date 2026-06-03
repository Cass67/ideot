use ideot::buffer::{Buffer, Position};
use ideot::editor::Editor;
use tempfile::tempdir;

#[test]
fn insert_and_delete_update_text_and_dirty_state() {
    let mut buffer = Buffer::from_text("hello\nworld".to_string());

    assert_eq!(buffer.line_count(), 2);
    assert!(!buffer.is_dirty());

    buffer.insert_char(Position { line: 0, column: 5 }, '!');
    assert_eq!(buffer.line(0), Some("hello!"));
    assert!(buffer.is_dirty());

    buffer.delete_char_before(Position { line: 0, column: 6 });
    assert_eq!(buffer.line(0), Some("hello"));
}

#[test]
fn newline_splits_line_and_backspace_joins_lines() {
    let mut buffer = Buffer::from_text("abcd".to_string());

    buffer.insert_newline(Position { line: 0, column: 2 });
    assert_eq!(buffer.line(0), Some("ab"));
    assert_eq!(buffer.line(1), Some("cd"));

    buffer.delete_char_before(Position { line: 1, column: 0 });
    assert_eq!(buffer.line_count(), 1);
    assert_eq!(buffer.line(0), Some("abcd"));
}

#[test]
fn load_and_save_round_trip_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "one\ntwo").unwrap();

    let mut buffer = Buffer::load(&path).unwrap();
    buffer.insert_char(Position { line: 1, column: 3 }, '!');
    buffer.save().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\ntwo!");
    assert!(!buffer.is_dirty());
}

#[test]
fn editor_tracks_cursor_while_editing() {
    let mut editor = Editor::new(Buffer::from_text("abc".to_string()));

    editor.move_right();
    editor.move_right();
    editor.insert_char('X');
    assert_eq!(editor.buffer().line(0), Some("abXc"));
    assert_eq!(editor.cursor(), Position { line: 0, column: 3 });

    editor.insert_newline();
    assert_eq!(editor.buffer().line(0), Some("abX"));
    assert_eq!(editor.buffer().line(1), Some("c"));
    assert_eq!(editor.cursor(), Position { line: 1, column: 0 });

    editor.backspace();
    assert_eq!(editor.buffer().line(0), Some("abXc"));
    assert_eq!(editor.cursor(), Position { line: 0, column: 3 });
}

#[test]
fn replace_text_replaces_entire_buffer_and_marks_dirty() {
    let mut buffer = Buffer::from_text("old\ntext".to_string());

    buffer.replace_text("new\nfile".to_string());

    assert_eq!(buffer.line_count(), 2);
    assert_eq!(buffer.line(0), Some("new"));
    assert_eq!(buffer.line(1), Some("file"));
    assert_eq!(buffer.text(), "new\nfile");
    assert!(buffer.is_dirty());
}

#[test]
fn insert_text_at_position_handles_multiline_text() {
    let mut buffer = Buffer::from_text("hello world".to_string());

    let end = buffer.insert_text(Position { line: 0, column: 5 }, "\nsmall\n".to_string());

    assert_eq!(buffer.text(), "hello\nsmall\n world");
    assert_eq!(end, Position { line: 2, column: 0 });
    assert!(buffer.is_dirty());
}

#[test]
fn insert_text_normalizes_carriage_return_line_endings() {
    let mut buffer = Buffer::from_text("start end".to_string());

    let end = buffer.insert_text(Position { line: 0, column: 5 }, "a\rb\r\nc".to_string());

    assert_eq!(buffer.text(), "starta\nb\nc end");
    assert_eq!(end, Position { line: 2, column: 1 });
}

#[test]
fn buffer_editing_snaps_non_char_boundary_columns() {
    let mut buffer = Buffer::from_text("√Clone".to_string());

    let end = buffer.insert_text(Position { line: 0, column: 2 }, "X".to_string());
    assert_eq!(buffer.text(), "√XClone");
    assert_eq!(end, Position { line: 0, column: 4 });

    buffer.insert_newline(Position { line: 0, column: 4 });
    assert_eq!(buffer.text(), "√X\nClone");

    buffer.delete_char_before(Position { line: 0, column: 2 });
    assert_eq!(buffer.text(), "X\nClone");
}
