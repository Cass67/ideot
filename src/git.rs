use anyhow::{anyhow, Context, Result};
use std::{path::Path, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommit {
    pub hash: String,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Equal,
    Add,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    pub before: Option<String>,
    pub after: Option<String>,
    pub kind: DiffKind,
}

pub fn recent_commits(root: &Path, limit: usize) -> Result<Vec<GitCommit>> {
    let output = git(root, &["log", "--oneline", &format!("-{limit}")])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let (hash, summary) = line.split_once(' ')?;
            Some(GitCommit {
                hash: hash.to_string(),
                summary: summary.to_string(),
            })
        })
        .collect())
}

pub fn changed_files(root: &Path, commit: &str) -> Result<Vec<String>> {
    let output = git(
        root,
        &["diff-tree", "--no-commit-id", "--name-only", "-r", commit],
    )?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}

pub fn file_at_rev(root: &Path, rev: &str, file: &str) -> Result<Vec<String>> {
    let output = git(root, &["show", &format!("{rev}:{file}")])?;
    Ok(output.lines().map(ToOwned::to_owned).collect())
}

pub fn diff_file_at_commit(root: &Path, commit: &str, file: &str) -> Result<Vec<DiffRow>> {
    let before = file_at_rev(root, &format!("{commit}^"), file).unwrap_or_default();
    let after = file_at_rev(root, commit, file).unwrap_or_default();
    Ok(align_lines(&before, &after))
}

pub fn align_lines(before: &[String], after: &[String]) -> Vec<DiffRow> {
    let mut dp = vec![vec![0usize; after.len() + 1]; before.len() + 1];
    for i in (0..before.len()).rev() {
        for j in (0..after.len()).rev() {
            dp[i][j] = if before[i] == after[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut rows = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < before.len() && j < after.len() {
        if before[i] == after[j] {
            rows.push(DiffRow {
                before: Some(before[i].clone()),
                after: Some(after[j].clone()),
                kind: DiffKind::Equal,
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            rows.push(DiffRow {
                before: Some(before[i].clone()),
                after: None,
                kind: DiffKind::Delete,
            });
            i += 1;
        } else {
            rows.push(DiffRow {
                before: None,
                after: Some(after[j].clone()),
                kind: DiffKind::Add,
            });
            j += 1;
        }
    }
    while i < before.len() {
        rows.push(DiffRow {
            before: Some(before[i].clone()),
            after: None,
            kind: DiffKind::Delete,
        });
        i += 1;
    }
    while j < after.len() {
        rows.push(DiffRow {
            before: None,
            after: Some(after[j].clone()),
            kind: DiffKind::Add,
        });
        j += 1;
    }
    rows
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {args:?}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
