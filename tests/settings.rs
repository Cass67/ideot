use ideot::settings::Settings;
use tempfile::tempdir;

#[test]
fn lsp_enabled_defaults_to_true_when_settings_file_missing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");

    let settings = Settings::load_from(&path).unwrap();

    assert!(settings.lsp_enabled);
    assert!(!settings.lsp_hover_enabled);
    assert!(settings.file_pane_visible);
    assert!(settings.line_numbers_visible);
}

#[test]
fn lsp_enabled_round_trips_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let settings = Settings {
        lsp_enabled: false,
        lsp_hover_enabled: true,
        file_pane_visible: false,
        line_numbers_visible: false,
    };

    settings.save_to(&path).unwrap();
    let loaded = Settings::load_from(&path).unwrap();

    assert!(!loaded.lsp_enabled);
    assert!(loaded.lsp_hover_enabled);
    assert!(!loaded.file_pane_visible);
    assert!(!loaded.line_numbers_visible);
}
