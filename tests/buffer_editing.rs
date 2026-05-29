use ideot::buffer::{Buffer, Position};
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
