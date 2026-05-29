use ideot::app::{App, FilePrompt};
use tempfile::tempdir;

#[test]
fn new_file_prompt_creates_file_in_current_open_directory() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("src/main.rs").unwrap();

    app.start_new_file_prompt();
    assert_eq!(app.file_prompt(), Some(FilePrompt::New));
    app.push_file_prompt_char('l');
    app.push_file_prompt_char('i');
    app.push_file_prompt_char('b');
    app.push_file_prompt_char('.');
    app.push_file_prompt_char('r');
    app.push_file_prompt_char('s');
    app.submit_file_prompt().unwrap();

    assert!(dir.path().join("src/lib.rs").exists());
    assert_eq!(app.current_relative(), Some("src/lib.rs"));
}

#[test]
fn delete_prompt_deletes_current_file_and_clears_editor() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "hello").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.start_delete_file_prompt();
    assert_eq!(app.file_prompt(), Some(FilePrompt::Delete));
    app.confirm_delete_current_file().unwrap();

    assert!(!dir.path().join("note.txt").exists());
    assert_eq!(app.current_relative(), None);
    assert!(app.editor().is_none());
}
