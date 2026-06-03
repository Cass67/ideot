use ideot::buffer::Position;
use ideot::input::{EditorViewport, MouseAction, MouseInputController};

#[test]
fn maps_editor_cell_to_buffer_position_with_scroll() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 10,
    };

    let position = viewport.position_for_cell(
        35,
        4,
        &[
            "short".to_string(),
            "abcdef".to_string(),
            "0123456789".to_string(),
            "line".to_string(),
        ],
    );

    assert_eq!(
        position,
        Some(Position {
            line: 13,
            column: 4
        })
    );
}

#[test]
fn mapping_clamps_column_to_line_length() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 0,
    };

    let position = viewport.position_for_cell(99, 1, &["abc".to_string()]);

    assert_eq!(position, Some(Position { line: 0, column: 3 }));
}

#[test]
fn mapping_converts_display_columns_to_utf8_byte_columns() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 0,
    };

    let after_sqrt = viewport.position_for_cell(31, 1, &["√Clone".to_string()]);
    let after_sqrt_and_c = viewport.position_for_cell(32, 1, &["√Clone".to_string()]);

    assert_eq!(after_sqrt, Some(Position { line: 0, column: 3 }));
    assert_eq!(after_sqrt_and_c, Some(Position { line: 0, column: 4 }));
}

#[test]
fn drag_sequence_emits_start_update_finish_actions() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 0,
    };
    let lines = vec!["abcdef".to_string(), "second".to_string()];
    let mut controller = MouseInputController::default();

    assert_eq!(
        controller.left_down(32, 1, &viewport, &lines),
        Some(MouseAction::MoveCursor(Position { line: 0, column: 2 }))
    );
    assert_eq!(
        controller.drag(35, 1, &viewport, &lines),
        Some(MouseAction::UpdateSelection {
            anchor: Position { line: 0, column: 2 },
            end: Position { line: 0, column: 5 },
        })
    );
    assert_eq!(controller.left_up(), Some(MouseAction::FinishSelection));
}

use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_mouse_selection_actions_select_text() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "abcdef\nsecond").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.move_editor_cursor_to(Position { line: 0, column: 1 });
    app.update_editor_selection(
        Position { line: 0, column: 1 },
        Position { line: 1, column: 3 },
    );
    app.finish_editor_selection();

    assert_eq!(app.editor().unwrap().selected_text(), "bcdef\nsec");
}
