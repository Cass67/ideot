use ideot::app::{App, GitDiffLayout, GitView};
use ideot::git::{align_lines, changed_files, recent_commits, DiffKind};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aligns_before_and_after_lines() {
    let before = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let after = vec![
        "a".to_string(),
        "x".to_string(),
        "c".to_string(),
        "d".to_string(),
    ];

    let rows = align_lines(&before, &after);

    assert_eq!(rows[0].kind, DiffKind::Equal);
    assert_eq!(rows[1].kind, DiffKind::Delete);
    assert_eq!(rows[2].kind, DiffKind::Add);
    assert_eq!(rows[3].kind, DiffKind::Equal);
    assert_eq!(rows[4].kind, DiffKind::Add);
}

#[test]
fn reads_commits_and_changed_files_from_git_repo() {
    let dir = git_repo_with_two_commits();

    let commits = recent_commits(dir.path(), 10).unwrap();
    assert!(commits.len() >= 2);

    let files = changed_files(dir.path(), &commits[0].hash).unwrap();
    assert_eq!(files, vec!["file.txt".to_string()]);
}

#[test]
fn git_diff_view_scrolls_and_clicks_before_after_panes() {
    let dir = git_repo_with_two_commits();
    let mut app = App::new(dir.path().to_path_buf());
    app.open_git_browser().unwrap();
    app.activate_git_selection().unwrap();
    app.activate_git_selection().unwrap();

    assert_eq!(app.git_diff_scroll(), 0);
    app.scroll_git_diff_down();
    app.scroll_git_diff_down();
    assert_eq!(app.git_diff_scroll(), 2);
    app.scroll_git_diff_up();
    assert_eq!(app.git_diff_scroll(), 1);
    app.page_git_diff_down(3);
    assert_eq!(app.git_diff_scroll(), 4);
    app.page_git_diff_up(2);
    assert_eq!(app.git_diff_scroll(), 2);

    app.click_git_diff_row(0, true);
    assert_eq!(app.git_diff_selected_row(), Some(2));
    assert_eq!(app.git_diff_selected_side(), Some(true));
    app.click_git_diff_row(2, false);
    assert_eq!(app.git_diff_selected_row(), Some(4));
    assert_eq!(app.git_diff_selected_side(), Some(false));

    assert_eq!(app.git_diff_layout(), GitDiffLayout::Unified);
    let unified = app.git_unified_diff_rows();
    assert!(unified.iter().any(|row| row.prefix == '-'));
    assert!(unified.iter().any(|row| row.prefix == '+'));
    app.toggle_git_diff_layout();
    assert_eq!(app.git_diff_layout(), GitDiffLayout::Split);
    app.toggle_git_diff_layout();
    assert_eq!(app.git_diff_layout(), GitDiffLayout::Unified);
}

#[test]
fn app_git_flow_loads_commit_file_and_diff() {
    let dir = git_repo_with_two_commits();
    let mut app = App::new(dir.path().to_path_buf());

    app.open_git_browser().unwrap();
    assert!(matches!(app.git_view(), Some(GitView::Commits)));

    app.activate_git_selection().unwrap();
    assert!(matches!(app.git_view(), Some(GitView::Files)));

    app.activate_git_selection().unwrap();
    assert!(matches!(app.git_view(), Some(GitView::Diff)));
    assert!(!app.git_diff_rows().is_empty());
}

fn git_repo_with_two_commits() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(dir.path(), &["config", "user.name", "Test User"]);
    std::fs::write(
        dir.path().join("file.txt"),
        "one\ntwo\nthree\nfour\nfive\nsix\n",
    )
    .unwrap();
    git(dir.path(), &["add", "file.txt"]);
    git(dir.path(), &["commit", "-m", "first"]);
    std::fs::write(
        dir.path().join("file.txt"),
        "one\nTWO\nthree\nFOUR\nfive\nSIX\nseven\n",
    )
    .unwrap();
    git(dir.path(), &["add", "file.txt"]);
    git(dir.path(), &["commit", "-m", "second"]);
    dir
}

fn git(cwd: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
