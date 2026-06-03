use ideot::fs::ProjectFile;
use ideot::search::{RecentFiles, SearchIndex};
use std::path::PathBuf;

fn file(path: &str) -> ProjectFile {
    ProjectFile {
        absolute: PathBuf::from(path),
        relative: path.to_string(),
    }
}

#[test]
fn fuzzy_search_returns_matching_files() {
    let files = vec![
        file("src/main.rs"),
        file("docs/design.md"),
        file("Cargo.toml"),
    ];
    let search = SearchIndex::new(files);

    let results = search.query("main", &RecentFiles::default());

    assert_eq!(results[0].relative, "src/main.rs");
}

#[test]
fn fuzzy_search_matches_extensions_and_dotted_names() {
    let files = vec![file("src/app.rs"), file("src/ui.rs"), file("README.md")];
    let search = SearchIndex::new(files);

    let dotted = search.query("app.rs", &RecentFiles::default());
    let extension = search.query("rs", &RecentFiles::default());

    assert_eq!(dotted[0].relative, "src/app.rs");
    assert!(extension.iter().any(|file| file.relative == "src/app.rs"));
    assert!(extension.iter().any(|file| file.relative == "src/ui.rs"));
}

#[test]
fn recent_files_are_boosted_when_scores_are_close() {
    let files = vec![file("src/app.rs"), file("examples/app.rs")];
    let search = SearchIndex::new(files);
    let mut recent = RecentFiles::default();
    recent.record("examples/app.rs");

    let results = search.query("app", &recent);

    assert_eq!(results[0].relative, "examples/app.rs");
}
