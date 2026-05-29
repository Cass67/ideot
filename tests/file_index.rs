use ideot::fs::ProjectIndex;
use tempfile::tempdir;

#[test]
fn indexes_files_relative_to_root_and_respects_gitignore() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("README.md"), "readme").unwrap();
    std::fs::write(dir.path().join("target/cache.txt"), "ignored").unwrap();
    std::fs::write(dir.path().join(".gitignore"), "target\n").unwrap();

    let index = ProjectIndex::build(dir.path()).unwrap();
    let paths: Vec<_> = index.files().iter().map(|file| file.relative.as_str()).collect();

    assert!(paths.contains(&"src/main.rs"));
    assert!(paths.contains(&"README.md"));
    assert!(!paths.contains(&"target/cache.txt"));
}
