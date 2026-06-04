use ideot::buffer::{Buffer, Position};
use ideot::editor::Editor;

#[test]
fn home_end_move_to_line_bounds() {
    let buffer = Buffer::from_text("hello world\nsecond".into());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(Position { line: 0, column: 5 });

    editor.move_line_end();
    assert_eq!(
        editor.cursor(),
        Position {
            line: 0,
            column: 11
        }
    );

    editor.move_line_start();
    assert_eq!(editor.cursor(), Position { line: 0, column: 0 });
}

#[test]
fn word_movement_skips_words_and_space() {
    let buffer = Buffer::from_text("hello   world".into());
    let mut editor = Editor::new(buffer);

    editor.move_word_right();
    assert_eq!(editor.cursor(), Position { line: 0, column: 8 });
    editor.move_word_right();
    assert_eq!(
        editor.cursor(),
        Position {
            line: 0,
            column: 13
        }
    );
    editor.move_word_left();
    assert_eq!(editor.cursor(), Position { line: 0, column: 8 });
}

#[test]
fn delete_removes_character_after_cursor_and_joins_lines() {
    let buffer = Buffer::from_text("ab\ncd".into());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(Position { line: 0, column: 1 });

    editor.edit_delete();
    assert_eq!(editor.buffer().text(), "a\ncd");

    editor.edit_delete();
    assert_eq!(editor.buffer().text(), "acd");
    assert_eq!(editor.cursor(), Position { line: 0, column: 1 });
}
