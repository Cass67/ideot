use ideot::buffer::{Buffer, Position};
use ideot::editor::Editor;

#[test]
fn undo_and_redo_insert_text() {
    let mut editor = Editor::new(Buffer::from_text("abc".into()));
    editor.set_cursor(Position { line: 0, column: 1 });

    editor.edit_insert_text("XYZ".to_string(), "insert");

    assert_eq!(editor.buffer().text(), "aXYZbc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 4 });
    assert_eq!(editor.undo(), Some("insert".to_string()));
    assert_eq!(editor.buffer().text(), "abc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 1 });
    assert_eq!(editor.redo(), Some("insert".to_string()));
    assert_eq!(editor.buffer().text(), "aXYZbc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 4 });
}

#[test]
fn undo_and_redo_replace_selection() {
    let mut editor = Editor::new(Buffer::from_text("one\ntwo\nthree".into()));
    editor.set_selection(
        Position { line: 0, column: 1 },
        Position { line: 1, column: 2 },
    );

    editor.edit_replace_selection("XX".to_string(), "replace selection");

    assert_eq!(editor.buffer().text(), "oXXo\nthree");
    assert_eq!(editor.undo(), Some("replace selection".to_string()));
    assert_eq!(editor.buffer().text(), "one\ntwo\nthree");
    assert_eq!(
        editor.selection().expect("selection restored").start,
        Position { line: 0, column: 1 }
    );
    assert_eq!(editor.redo(), Some("replace selection".to_string()));
    assert_eq!(editor.buffer().text(), "oXXo\nthree");
}

#[test]
fn new_edit_after_undo_clears_redo() {
    let mut editor = Editor::new(Buffer::from_text("abc".into()));
    editor.set_cursor(Position { line: 0, column: 3 });
    editor.edit_insert_text("d".to_string(), "insert");
    editor.undo();

    editor.edit_insert_text("X".to_string(), "insert");

    assert_eq!(editor.buffer().text(), "abcX");
    assert_eq!(editor.redo(), None);
}

#[test]
fn undo_stack_is_capped_at_100_entries() {
    let mut editor = Editor::new(Buffer::from_text("".into()));

    for _ in 0..101 {
        editor.edit_insert_text("x".to_string(), "insert");
    }

    let mut undo_count = 0;
    while editor.undo().is_some() {
        undo_count += 1;
    }

    assert_eq!(undo_count, 100);
}

use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_undo_and_redo_restore_file_buffer_after_insert() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "abc").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.insert_char('X');
    assert_eq!(app.editor().unwrap().buffer().text(), "Xabc");

    app.undo();
    assert_eq!(app.editor().unwrap().buffer().text(), "abc");

    app.redo();
    assert_eq!(app.editor().unwrap().buffer().text(), "Xabc");
}

#[test]
fn app_select_all_selects_entire_open_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "one\ntwo").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.select_all();

    assert_eq!(app.editor().unwrap().selected_text(), "one\ntwo");
}
