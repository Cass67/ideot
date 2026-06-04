use ideot::app::{App, FocusPane};
use ideot::settings::Settings;
use tempfile::tempdir;

#[test]
fn hidden_file_pane_starts_with_editor_focus() {
    let dir = tempdir().unwrap();
    let settings = Settings {
        file_pane_visible: false,
        ..Settings::default()
    };

    let app = App::new_with_settings(dir.path().to_path_buf(), settings);

    assert_eq!(app.focus_pane(), FocusPane::Editor);
}

#[test]
fn focus_toggle_stays_on_editor_when_file_pane_hidden() {
    let dir = tempdir().unwrap();
    let settings = Settings {
        file_pane_visible: false,
        ..Settings::default()
    };
    let mut app = App::new_with_settings(dir.path().to_path_buf(), settings);

    app.toggle_focus_pane();

    assert_eq!(app.focus_pane(), FocusPane::Editor);
}

#[test]
fn hiding_file_pane_moves_focus_to_editor() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path().to_path_buf());

    assert_eq!(app.focus_pane(), FocusPane::Explorer);
    app.toggle_file_pane_visible().unwrap();

    assert_eq!(app.focus_pane(), FocusPane::Editor);
}
