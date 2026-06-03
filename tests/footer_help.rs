use ideot::ui;

#[test]
fn footer_only_shows_help_and_quit_shortcuts() {
    assert_eq!(ui::footer_shortcuts(false), "F1 Help · Ctrl-Q Quit");
    assert_eq!(ui::footer_shortcuts(true), "F1 Help · Ctrl-Q Quit");
}

#[test]
fn help_page_contains_full_command_reference() {
    let help = ui::help_text_lines().join("\n");

    assert!(help.contains("Ctrl-A       Select all text in current file"));
    assert!(help.contains("Y            Copy selection"));
    assert!(help.contains("Ctrl+V       Paste"));
    assert!(help.contains("U            Undo"));
    assert!(help.contains("Ctrl-R       Redo"));
    assert!(help.contains("Ctrl-P       Search files"));
    assert!(help.contains("Ctrl-G       Git commit browser"));
    assert!(help.contains("Ctrl-H       LSP hover"));
    assert!(help.contains("Ctrl-/       LSP completion"));
    assert!(help.contains("Ctrl-]       LSP go to definition"));
    assert!(help.contains("Drag editor text  Select text in ideot"));
}
