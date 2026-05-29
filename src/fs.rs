use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    pub absolute: PathBuf,
    pub relative: String,
}

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    root: PathBuf,
    files: Vec<ProjectFile>,
}

impl ProjectIndex {
    pub fn build(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = Vec::new();
        for entry in WalkBuilder::new(&root)
            .hidden(false)
            .require_git(false)
            .build()
        {
            let entry = entry.context("failed to walk project")?;
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            let absolute = entry.path().to_path_buf();
            let relative = absolute
                .strip_prefix(&root)
                .unwrap_or(&absolute)
                .to_string_lossy()
                .replace('\\', "/");
            files.push(ProjectFile { absolute, relative });
        }
        files.sort_by(|a, b| a.relative.cmp(&b.relative));
        Ok(Self { root, files })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn files(&self) -> &[ProjectFile] {
        &self.files
    }
}
