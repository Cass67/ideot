use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_opens_edits_saves_and_tracks_recent_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("main.rs");
    std::fs::write(&path, "fn main() {}").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.insert_char('/');
    app.save_current().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "/fn main() {}");
    assert_eq!(app.current_relative(), Some("main.rs"));
    assert_eq!(app.search("main")[0].relative, "main.rs");
}
