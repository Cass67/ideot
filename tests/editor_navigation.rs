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
fn app_typing_multiple_chars_preserves_order_and_advances_cursor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "").unwrap();
    let mut app = ideot::app::App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.insert_char('c');
    app.insert_char('a');

    assert_eq!(app.editor().unwrap().buffer().text(), "ca");
    assert_eq!(
        app.editor().unwrap().cursor(),
        ideot::buffer::Position { line: 0, column: 2 }
    );
}

#[test]
fn typing_multiple_chars_preserves_order_and_advances_cursor() {
    let buffer = ideot::buffer::Buffer::from_text(String::new());
    let mut editor = Editor::new(buffer);

    editor.edit_insert_text("c".to_string(), "insert");
    editor.edit_insert_text("a".to_string(), "insert");

    assert_eq!(editor.buffer().text(), "ca");
    assert_eq!(
        editor.cursor(),
        ideot::buffer::Position { line: 0, column: 2 }
    );
}

#[test]
fn editor_tab_inserts_four_spaces() {
    let buffer = ideot::buffer::Buffer::from_text("fn main() {".to_string());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(ideot::buffer::Position { line: 0, column: 0 });

    editor.edit_insert_text("    ".to_string(), "indent");

    assert_eq!(editor.buffer().text(), "    fn main() {");
    assert_eq!(
        editor.cursor(),
        ideot::buffer::Position { line: 0, column: 4 }
    );
}

#[test]
fn backspace_removes_previous_character_without_extra_cursor_jump() {
    let buffer = ideot::buffer::Buffer::from_text("pub label:".to_string());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(ideot::buffer::Position {
        line: 0,
        column: 10,
    });

    editor.edit_backspace();

    assert_eq!(editor.buffer().text(), "pub label");
    assert_eq!(
        editor.cursor(),
        ideot::buffer::Position { line: 0, column: 9 }
    );
}

#[test]
fn delete_at_end_of_line_does_not_jump_or_join_next_line() {
    let buffer = ideot::buffer::Buffer::from_text("x\ny".to_string());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(ideot::buffer::Position { line: 0, column: 1 });

    editor.edit_delete();

    assert_eq!(editor.buffer().text(), "x\ny");
    assert_eq!(
        editor.cursor(),
        ideot::buffer::Position { line: 0, column: 1 }
    );
}

#[test]
fn delete_removes_character_after_cursor() {
    let buffer = Buffer::from_text("ab\ncd".into());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(Position { line: 0, column: 1 });

    editor.edit_delete();
    assert_eq!(editor.buffer().text(), "a\ncd");

    editor.edit_delete();
    assert_eq!(editor.buffer().text(), "a\ncd");
    assert_eq!(editor.cursor(), Position { line: 0, column: 1 });
}
