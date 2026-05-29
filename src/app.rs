pub mod command;

use crate::buffer::{Buffer, Position};
use crate::editor::Editor;
use crate::fs::{ProjectFile, ProjectIndex};
use crate::git::{self, DiffRow, GitCommit};
use crate::lsp::{DocumentEvent, DocumentEventSink, NullDocumentEventSink};
use crate::marks::SessionMarks;
use crate::search::{RecentFiles, SearchIndex};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplorerEntry {
    pub label: String,
    pub relative: Option<String>,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitView {
    Commits,
    Files,
    Diff,
}

#[derive(Debug, Clone, Default)]
pub struct GitBrowserState {
    pub commits: Vec<GitCommit>,
    pub files: Vec<String>,
    pub diff_rows: Vec<DiffRow>,
    pub selected_commit: usize,
    pub selected_file: usize,
    pub view: Option<GitView>,
    pub diff_scroll: usize,
    pub diff_selected_row: Option<usize>,
    pub diff_selected_before: Option<bool>,
}

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
    pub lsp_sink: NullDocumentEventSink,
    index: Option<ProjectIndex>,
    search_index: SearchIndex,
    recent: RecentFiles,
    marks: SessionMarks,
    editor: Option<Editor>,
    current_relative: Option<String>,
    status: String,
    selected_file: usize,
    search_query: String,
    search_open: bool,
    expanded_dirs: BTreeSet<String>,
    explorer_scroll: usize,
    editor_scroll: usize,
    help_open: bool,
    git: GitBrowserState,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            should_quit: false,
            lsp_sink: NullDocumentEventSink,
            index: None,
            search_index: SearchIndex::new(Vec::new()),
            recent: RecentFiles::default(),
            marks: SessionMarks::default(),
            editor: None,
            current_relative: None,
            status: String::new(),
            selected_file: 0,
            search_query: String::new(),
            search_open: false,
            expanded_dirs: BTreeSet::new(),
            explorer_scroll: 0,
            editor_scroll: 0,
            help_open: false,
            git: GitBrowserState::default(),
        }
    }

    pub fn rebuild_index(&mut self) -> Result<()> {
        let index = ProjectIndex::build(&self.root)?;
        self.search_index = SearchIndex::new(index.files().to_vec());
        self.index = Some(index);
        Ok(())
    }

    pub fn open_relative(&mut self, relative: &str) -> Result<()> {
        let path = self.root.join(relative);
        let buffer = Buffer::load(&path)?;
        let text = buffer.text();
        self.editor = Some(Editor::new(buffer));
        self.current_relative = Some(relative.to_string());
        self.recent.record(relative);
        self.search_open = false;
        self.search_query.clear();
        self.editor_scroll = 0;
        self.lsp_sink.send(DocumentEvent::Opened { path, text });
        Ok(())
    }

    pub fn insert_char(&mut self, ch: char) {
        if let Some(editor) = &mut self.editor {
            editor.insert_char(ch);
            if let (Some(relative), Some(path)) =
                (self.current_relative.as_ref(), editor.buffer().path())
            {
                self.lsp_sink.send(DocumentEvent::Changed {
                    path: path.to_path_buf(),
                    text: editor.buffer().text(),
                });
                self.recent.record(relative.clone());
            }
        }
    }

    pub fn backspace(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.backspace();
            if let Some(path) = editor.buffer().path() {
                self.lsp_sink.send(DocumentEvent::Changed {
                    path: path.to_path_buf(),
                    text: editor.buffer().text(),
                });
            }
        }
    }

    pub fn save_current(&mut self) -> Result<()> {
        let editor = self.editor.as_mut().context("no open file")?;
        editor.buffer_mut().save()?;
        if let Some(path) = editor.buffer().path() {
            self.lsp_sink.send(DocumentEvent::Saved {
                path: path.to_path_buf(),
                text: editor.buffer().text(),
            });
        }
        self.status = "saved".to_string();
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<ProjectFile> {
        self.search_index.query(query, &self.recent)
    }

    pub fn explorer_entries(&self) -> Vec<ExplorerEntry> {
        let Some(index) = &self.index else {
            return Vec::new();
        };
        let mut dirs = BTreeSet::new();
        let mut root_files = Vec::new();
        for file in index.files() {
            let parts: Vec<&str> = file.relative.split('/').collect();
            if parts.len() == 1 {
                root_files.push(file.relative.clone());
            } else {
                for depth in 1..parts.len() {
                    dirs.insert(parts[..depth].join("/"));
                }
            }
        }

        let mut entries = Vec::new();
        for dir in dirs {
            let parent_expanded = dir
                .rsplit_once('/')
                .map(|(parent, _)| self.expanded_dirs.contains(parent))
                .unwrap_or(true);
            if !parent_expanded {
                continue;
            }
            let depth = dir.matches('/').count();
            let marker = if self.expanded_dirs.contains(&dir) {
                '▾'
            } else {
                '▸'
            };
            entries.push(ExplorerEntry {
                label: format!("{}{marker} {dir}", "  ".repeat(depth)),
                relative: None,
                is_dir: true,
            });
            if self.expanded_dirs.contains(&dir) {
                let file_indent = "  ".repeat(depth + 2);
                for file in index
                    .files()
                    .iter()
                    .filter(|file| direct_child_file(&file.relative, &dir))
                {
                    entries.push(ExplorerEntry {
                        label: format!("{file_indent}{}", file.relative),
                        relative: Some(file.relative.clone()),
                        is_dir: false,
                    });
                }
            }
        }
        entries.extend(root_files.into_iter().map(|file| ExplorerEntry {
            label: format!("  {file}"),
            relative: Some(file),
            is_dir: false,
        }));
        entries
    }

    pub fn current_relative(&self) -> Option<&str> {
        self.current_relative.as_deref()
    }

    pub fn language_hint(&self) -> Option<&str> {
        self.current_relative.as_ref()?.rsplit('.').next()
    }

    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    pub fn marks(&self) -> &SessionMarks {
        &self.marks
    }

    pub fn move_selection_down(&mut self) {
        let max = if self.search_open {
            self.search(self.search_query.as_str())
                .len()
                .saturating_sub(1)
        } else {
            self.explorer_entries().len().saturating_sub(1)
        };
        self.selected_file = (self.selected_file + 1).min(max);
    }

    pub fn move_selection_up(&mut self) {
        self.selected_file = self.selected_file.saturating_sub(1);
    }

    pub fn selected_file(&self) -> usize {
        self.selected_file
    }

    pub fn open_selected(&mut self) -> Result<()> {
        let query = if self.search_open {
            self.search_query.as_str()
        } else {
            ""
        };
        let files = self.search(query);
        if let Some(file) = files.get(self.selected_file) {
            let relative = file.relative.clone();
            self.open_relative(&relative)?;
        }
        Ok(())
    }

    pub fn activate_selected(&mut self) -> Result<()> {
        if self.search_open {
            return self.open_selected();
        }
        self.activate_explorer_index(self.selected_file)
    }

    pub fn activate_explorer_visible_row(&mut self, row: usize) -> Result<()> {
        let index = self.explorer_scroll + row;
        self.selected_file = index.min(self.explorer_entries().len().saturating_sub(1));
        self.activate_explorer_index(index)
    }

    fn activate_explorer_index(&mut self, index: usize) -> Result<()> {
        let Some(entry) = self.explorer_entries().get(index).cloned() else {
            return Ok(());
        };
        if let Some(relative) = entry.relative {
            self.open_relative(&relative)?;
        } else if entry.is_dir {
            let dir = entry
                .label
                .trim_start_matches([' ', '▸', '▾'])
                .trim()
                .to_string();
            if !self.expanded_dirs.insert(dir.clone()) {
                self.expanded_dirs.remove(&dir);
            }
        }
        Ok(())
    }

    pub fn place_editor_cursor(&mut self, visible_row: usize, column: usize) {
        if let Some(editor) = &mut self.editor {
            editor.set_cursor(Position {
                line: self.editor_scroll + visible_row,
                column,
            });
        }
    }

    pub fn toggle_search(&mut self) {
        self.search_open = !self.search_open;
        self.search_query.clear();
        self.selected_file = 0;
    }

    pub fn search_open(&self) -> bool {
        self.search_open
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    pub fn open_git_browser(&mut self) -> Result<()> {
        self.git.commits = git::recent_commits(&self.root, 100)?;
        self.git.files.clear();
        self.git.diff_rows.clear();
        self.git.selected_commit = 0;
        self.git.selected_file = 0;
        self.git.view = Some(GitView::Commits);
        Ok(())
    }

    pub fn git_view(&self) -> Option<GitView> {
        self.git.view
    }

    pub fn git_commits(&self) -> &[GitCommit] {
        &self.git.commits
    }

    pub fn git_files(&self) -> &[String] {
        &self.git.files
    }

    pub fn git_diff_rows(&self) -> &[DiffRow] {
        &self.git.diff_rows
    }

    pub fn git_diff_scroll(&self) -> usize {
        self.git.diff_scroll
    }

    pub fn git_diff_selected_row(&self) -> Option<usize> {
        self.git.diff_selected_row
    }

    pub fn git_diff_selected_side(&self) -> Option<bool> {
        self.git.diff_selected_before
    }

    pub fn git_selected_index(&self) -> usize {
        match self.git.view {
            Some(GitView::Commits) => self.git.selected_commit,
            Some(GitView::Files) => self.git.selected_file,
            _ => 0,
        }
    }

    pub fn activate_git_selection(&mut self) -> Result<()> {
        match self.git.view {
            Some(GitView::Commits) => {
                let Some(commit) = self.git.commits.get(self.git.selected_commit) else {
                    return Ok(());
                };
                self.git.files = git::changed_files(&self.root, &commit.hash)?;
                self.git.selected_file = 0;
                self.git.view = Some(GitView::Files);
            }
            Some(GitView::Files) => {
                let Some(commit) = self.git.commits.get(self.git.selected_commit) else {
                    return Ok(());
                };
                let Some(file) = self.git.files.get(self.git.selected_file) else {
                    return Ok(());
                };
                self.git.diff_rows = git::diff_file_at_commit(&self.root, &commit.hash, file)?;
                self.git.diff_scroll = 0;
                self.git.diff_selected_row = None;
                self.git.diff_selected_before = None;
                self.git.view = Some(GitView::Diff);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn git_move_down(&mut self) {
        match self.git.view {
            Some(GitView::Commits) => {
                self.git.selected_commit =
                    (self.git.selected_commit + 1).min(self.git.commits.len().saturating_sub(1))
            }
            Some(GitView::Files) => {
                self.git.selected_file =
                    (self.git.selected_file + 1).min(self.git.files.len().saturating_sub(1))
            }
            Some(GitView::Diff) => self.scroll_git_diff_down(),
            _ => {}
        }
    }

    pub fn git_move_up(&mut self) {
        match self.git.view {
            Some(GitView::Commits) => {
                self.git.selected_commit = self.git.selected_commit.saturating_sub(1)
            }
            Some(GitView::Files) => {
                self.git.selected_file = self.git.selected_file.saturating_sub(1)
            }
            Some(GitView::Diff) => self.scroll_git_diff_up(),
            _ => {}
        }
    }

    pub fn scroll_git_diff_down(&mut self) {
        self.page_git_diff_down(1);
    }

    pub fn scroll_git_diff_up(&mut self) {
        self.page_git_diff_up(1);
    }

    pub fn page_git_diff_down(&mut self, amount: usize) {
        self.git.diff_scroll =
            (self.git.diff_scroll + amount).min(self.git.diff_rows.len().saturating_sub(1));
    }

    pub fn page_git_diff_up(&mut self, amount: usize) {
        self.git.diff_scroll = self.git.diff_scroll.saturating_sub(amount);
    }

    pub fn click_git_diff_row(&mut self, visible_row: usize, before_side: bool) {
        if self.git.view != Some(GitView::Diff) {
            return;
        }
        let row = self.git.diff_scroll + visible_row;
        if row < self.git.diff_rows.len() {
            self.git.diff_selected_row = Some(row);
            self.git.diff_selected_before = Some(before_side);
        }
    }

    pub fn git_back(&mut self) {
        self.git.view = match self.git.view {
            Some(GitView::Diff) => Some(GitView::Files),
            Some(GitView::Files) => Some(GitView::Commits),
            Some(GitView::Commits) => None,
            None => None,
        };
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn push_search_char(&mut self, ch: char) {
        self.search_query.push(ch);
        self.selected_file = 0;
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.selected_file = 0;
    }

    pub fn mark_current_file(&mut self) -> Option<usize> {
        let relative = self.current_relative.clone()?;
        let slot = self.marks.mark(relative);
        self.status = format!("marked file in slot {slot}");
        Some(slot)
    }

    pub fn jump_to_mark(&mut self, slot: usize) -> Result<()> {
        if let Some(relative) = self.marks.get(slot).cloned() {
            self.open_relative(&relative)?;
        }
        Ok(())
    }

    pub fn explorer_scroll(&self) -> usize {
        self.explorer_scroll
    }

    pub fn editor_scroll(&self) -> usize {
        self.editor_scroll
    }

    pub fn scroll_explorer_down(&mut self) {
        self.page_explorer_down(1);
    }

    pub fn scroll_explorer_up(&mut self) {
        self.page_explorer_up(1);
    }

    pub fn page_explorer_down(&mut self, amount: usize) {
        self.explorer_scroll += amount;
    }

    pub fn page_explorer_up(&mut self, amount: usize) {
        self.explorer_scroll = self.explorer_scroll.saturating_sub(amount);
    }

    pub fn scroll_editor_down(&mut self) {
        self.page_editor_down(1);
    }

    pub fn scroll_editor_up(&mut self) {
        self.page_editor_up(1);
    }

    pub fn page_editor_down(&mut self, amount: usize) {
        self.editor_scroll += amount;
    }

    pub fn page_editor_up(&mut self, amount: usize) {
        self.editor_scroll = self.editor_scroll.saturating_sub(amount);
    }
}

fn direct_child_file(file: &str, dir: &str) -> bool {
    let Some(rest) = file
        .strip_prefix(dir)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return false;
    };
    !rest.contains('/')
}
