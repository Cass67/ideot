pub mod command;

use crate::buffer::{Buffer, Position};
use crate::editor::Editor;
use crate::fs::{ProjectFile, ProjectIndex};
use crate::git::{self, DiffRow, GitCommit};
use crate::lsp::{CompletionItem, DiagnosticsStore, HoverInfo, LspClient, LspMsg};
use crate::marks::SessionMarks;
use crate::search::{RecentFiles, SearchIndex};
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplorerEntry {
    pub label: String,
    pub relative: Option<String>,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePrompt {
    New,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Explorer,
    Editor,
}

#[derive(Debug, Clone, Default)]
pub struct FilePromptState {
    pub kind: Option<FilePrompt>,
    pub input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitView {
    Commits,
    Files,
    Diff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitDiffLayout {
    #[default]
    Split,
    Unified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedDiffRow {
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub prefix: char,
    pub text: String,
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
    pub diff_layout: GitDiffLayout,
}

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
    pub lsp: Option<LspClient>,
    pub diagnostics: DiagnosticsStore,
    pub hover_popup: Option<HoverInfo>,
    pub completion_popup: Option<Vec<CompletionItem>>,
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
    file_prompt: FilePromptState,
    focus_pane: FocusPane,
    git: GitBrowserState,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            should_quit: false,
            lsp: None,
            diagnostics: DiagnosticsStore::new(),
            hover_popup: None,
            completion_popup: None,
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
            file_prompt: FilePromptState::default(),
            focus_pane: FocusPane::Explorer,
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
        self.lsp_did_open(&path, &text);

        // Spawn LSP client if not already initialized
        if self.lsp.is_none() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if let Some(lsp) = LspClient::init(ext, &self.root) {
                self.lsp = Some(lsp);
                self.lsp_did_open(&path, &text);
            }
        }
        Ok(())
    }

    pub fn lsp_hover(&self, path: &std::path::Path, pos: Position) {
        if let Some(lsp) = &self.lsp {
            lsp.hover(
                &self.lsp_uri(path),
                super::lsp::Position {
                    line: pos.line as u32,
                    character: pos.column as u32,
                },
            );
        }
    }

    pub fn lsp_completion(&self, path: &std::path::Path, pos: Position) {
        if let Some(lsp) = &self.lsp {
            lsp.completion(
                &self.lsp_uri(path),
                super::lsp::Position {
                    line: pos.line as u32,
                    character: pos.column as u32,
                },
            );
        }
    }

    pub fn lsp_definition(&self, path: &std::path::Path, pos: Position) {
        if let Some(lsp) = &self.lsp {
            lsp.definition(
                &self.lsp_uri(path),
                super::lsp::Position {
                    line: pos.line as u32,
                    character: pos.column as u32,
                },
            );
        }
    }

    fn lsp_did_open(&self, path: &std::path::Path, text: &str) {
        if let Some(lsp) = &self.lsp {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = ext.trim_start_matches('.');
            lsp.did_open(&self.lsp_uri(path), text, lang);
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        if let Some(editor) = &mut self.editor {
            editor.edit_insert_text(ch.to_string(), "insert");
            self.after_current_editor_changed();
        }
    }

    pub fn insert_newline(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.edit_insert_text("\n".to_string(), "insert");
            self.after_current_editor_changed();
        }
    }

    pub fn backspace(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.edit_backspace();
            self.after_current_editor_changed();
        }
    }

    pub fn save_current(&mut self) -> Result<()> {
        let editor = self.editor.as_mut().context("no open file")?;
        editor.buffer_mut().save()?;
        if let Some(path) = editor.buffer().path() {
            let path = path.to_path_buf();
            let text = editor.buffer().text().to_string();
            self.lsp_save(&path, text);
        }
        self.status = "saved".to_string();
        Ok(())
    }

    fn lsp_uri(&self, path: &std::path::Path) -> String {
        format!("file://{}", path.display())
    }

    fn lsp_change(&self, path: &std::path::Path, text: String) {
        if let Some(lsp) = &self.lsp {
            lsp.did_change(&self.lsp_uri(path), &text);
        }
    }

    fn lsp_save(&self, path: &std::path::Path, text: String) {
        if let Some(lsp) = &self.lsp {
            lsp.did_save(&self.lsp_uri(path), &text);
        }
    }

    fn after_current_editor_changed(&mut self) {
        let editor_change = self.editor.as_ref().and_then(|editor| {
            editor
                .buffer()
                .path()
                .map(|path| (path.to_path_buf(), editor.buffer().text().to_string()))
        });
        if let Some((path, text)) = editor_change {
            self.lsp_change(&path, text);
        }
        if let Some(relative) = self.current_relative.as_ref() {
            self.recent.record(relative.clone());
        }
    }

    pub fn poll_lsp(&mut self) {
        if let Some(lsp) = &self.lsp {
            while let Some(msg) = lsp.poll() {
                match msg {
                    LspMsg::Diagnostics { uri, diagnostics } => {
                        self.diagnostics.update(uri, diagnostics);
                    }
                    LspMsg::Hover(h) => self.hover_popup = h,
                    LspMsg::Completion(c) => self.completion_popup = c,
                    LspMsg::Definition(_) => {}
                }
            }
        }
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

    pub fn focused_arrow_down(&mut self) {
        match self.focus_pane {
            FocusPane::Explorer => self.move_selection_down(),
            FocusPane::Editor => self.page_editor_down(1),
        }
    }

    pub fn focused_arrow_up(&mut self) {
        match self.focus_pane {
            FocusPane::Explorer => self.move_selection_up(),
            FocusPane::Editor => self.page_editor_up(1),
        }
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

    pub fn move_editor_cursor_to(&mut self, position: Position) {
        self.focus_pane = FocusPane::Editor;
        if let Some(editor) = &mut self.editor {
            editor.set_cursor(position);
            editor.clear_selection();
        }
    }

    pub fn update_editor_selection(&mut self, anchor: Position, end: Position) {
        self.focus_pane = FocusPane::Editor;
        if let Some(editor) = &mut self.editor {
            editor.set_selection(anchor, end);
        }
    }

    pub fn finish_editor_selection(&mut self) {
        self.status = "selection ready".to_string();
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

    pub fn git_diff_layout(&self) -> GitDiffLayout {
        self.git.diff_layout
    }

    pub fn toggle_git_diff_layout(&mut self) {
        self.git.diff_layout = match self.git.diff_layout {
            GitDiffLayout::Split => GitDiffLayout::Unified,
            GitDiffLayout::Unified => GitDiffLayout::Split,
        };
    }

    pub fn git_unified_diff_rows(&self) -> Vec<UnifiedDiffRow> {
        let (mut old_line, mut new_line) = (1usize, 1usize);
        let mut rows = Vec::new();
        for row in &self.git.diff_rows {
            match row.kind {
                crate::git::DiffKind::Equal => {
                    rows.push(UnifiedDiffRow {
                        old_line: Some(old_line),
                        new_line: Some(new_line),
                        prefix: ' ',
                        text: row
                            .before
                            .clone()
                            .or_else(|| row.after.clone())
                            .unwrap_or_default(),
                    });
                    old_line += 1;
                    new_line += 1;
                }
                crate::git::DiffKind::Delete => {
                    rows.push(UnifiedDiffRow {
                        old_line: Some(old_line),
                        new_line: None,
                        prefix: '-',
                        text: row.before.clone().unwrap_or_default(),
                    });
                    old_line += 1;
                }
                crate::git::DiffKind::Add => {
                    rows.push(UnifiedDiffRow {
                        old_line: None,
                        new_line: Some(new_line),
                        prefix: '+',
                        text: row.after.clone().unwrap_or_default(),
                    });
                    new_line += 1;
                }
            }
        }
        rows
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
                self.git.diff_layout = GitDiffLayout::Unified;
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

    pub fn focus_pane(&self) -> FocusPane {
        self.focus_pane
    }

    pub fn toggle_focus_pane(&mut self) {
        self.focus_pane = match self.focus_pane {
            FocusPane::Explorer => FocusPane::Editor,
            FocusPane::Editor => FocusPane::Explorer,
        };
    }

    pub fn file_prompt(&self) -> Option<FilePrompt> {
        self.file_prompt.kind
    }

    pub fn file_prompt_input(&self) -> &str {
        &self.file_prompt.input
    }

    pub fn start_new_file_prompt(&mut self) {
        self.file_prompt.kind = Some(FilePrompt::New);
        self.file_prompt.input.clear();
    }

    pub fn start_delete_file_prompt(&mut self) {
        if self.current_relative.is_some() {
            self.file_prompt.kind = Some(FilePrompt::Delete);
            self.file_prompt.input.clear();
        }
    }

    pub fn cancel_file_prompt(&mut self) {
        self.file_prompt.kind = None;
        self.file_prompt.input.clear();
    }

    pub fn push_file_prompt_char(&mut self, ch: char) {
        if self.file_prompt.kind == Some(FilePrompt::New) {
            self.file_prompt.input.push(ch);
        }
    }

    pub fn pop_file_prompt_char(&mut self) {
        self.file_prompt.input.pop();
    }

    pub fn submit_file_prompt(&mut self) -> Result<()> {
        match self.file_prompt.kind {
            Some(FilePrompt::New) => self.create_prompt_file(),
            Some(FilePrompt::Delete) => self.confirm_delete_current_file(),
            None => Ok(()),
        }
    }

    fn create_prompt_file(&mut self) -> Result<()> {
        let name = self.file_prompt.input.trim().trim_start_matches('/');
        if name.is_empty() || name.contains("..") {
            bail!("invalid file name");
        }
        let base_dir = self
            .current_relative
            .as_deref()
            .and_then(|path| path.rsplit_once('/').map(|(dir, _)| dir.to_string()))
            .unwrap_or_default();
        let relative = if base_dir.is_empty() {
            name.to_string()
        } else {
            format!("{base_dir}/{name}")
        };
        let absolute = self.root.join(&relative);
        if let Some(parent) = absolute.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !absolute.exists() {
            std::fs::write(&absolute, "")?;
        }
        self.rebuild_index()?;
        self.open_relative(&relative)?;
        self.cancel_file_prompt();
        Ok(())
    }

    pub fn confirm_delete_current_file(&mut self) -> Result<()> {
        let Some(relative) = self.current_relative.clone() else {
            return Ok(());
        };
        std::fs::remove_file(self.root.join(&relative))?;
        self.editor = None;
        self.current_relative = None;
        self.rebuild_index()?;
        self.cancel_file_prompt();
        self.status = format!("deleted {relative}");
        Ok(())
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
        if let Some(editor) = &mut self.editor {
            for _ in 0..amount {
                editor.move_down();
            }
            self.editor_scroll = editor.cursor().line;
        }
    }

    pub fn page_editor_up(&mut self, amount: usize) {
        if let Some(editor) = &mut self.editor {
            for _ in 0..amount {
                editor.move_up();
            }
            self.editor_scroll = editor.cursor().line;
        }
    }

    /// Adjust editor scroll so the cursor line stays visible
    pub fn scroll_to_cursor(&mut self, visible_height: usize) {
        let Some(editor) = &self.editor else { return };
        let cursor_line = editor.cursor().line;
        // If cursor is below visible area, scroll down
        if cursor_line >= self.editor_scroll + visible_height {
            self.editor_scroll = cursor_line - visible_height + 1;
        }
        // If cursor is above visible area, scroll up
        if cursor_line < self.editor_scroll {
            self.editor_scroll = cursor_line;
        }
    }

    pub fn editor_move_down(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.move_down();
        }
    }

    pub fn editor_move_up(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.move_up();
        }
    }

    pub fn editor_move_left(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.move_left();
        }
    }

    pub fn editor_move_right(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.move_right();
        }
    }

    // --- Copy / Paste ---

    pub fn copy_selection(&mut self) -> Result<()> {
        let Some(editor) = &mut self.editor else {
            bail!("no open file");
        };
        let text = editor.selected_text();
        if text.is_empty() {
            bail!("nothing selected");
        }
        let line_count = text.matches('\n').count() + 1;
        let char_count = text.chars().count();
        let mut clipboard = arboard::Clipboard::new().context("failed to open clipboard")?;
        clipboard.set_text(text)?;
        self.status = if line_count > 1 {
            format!("copied {line_count} lines")
        } else {
            format!("copied {char_count} chars")
        };
        Ok(())
    }

    pub fn select_all(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.select_all();
            self.status = "selected all".to_string();
        }
    }

    pub fn undo(&mut self) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        match editor.undo() {
            Some(label) => {
                self.status = format!("undid {label}");
                self.after_current_editor_changed();
            }
            None => self.status = "nothing to undo".to_string(),
        }
    }

    pub fn redo(&mut self) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        match editor.redo() {
            Some(label) => {
                self.status = format!("redid {label}");
                self.after_current_editor_changed();
            }
            None => self.status = "nothing to redo".to_string(),
        }
    }

    pub fn paste(&mut self) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new().context("failed to open clipboard")?;
        let text = clipboard.get_text()?;
        self.insert_pasted_text(text);
        Ok(())
    }

    pub fn insert_pasted_text(&mut self, text: String) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        let line_count = text.matches('\n').count() + 1;
        let char_count = text.chars().count();
        if editor.selection().is_some() {
            editor.edit_replace_selection(text, "paste");
        } else {
            editor.edit_insert_text(text, "paste");
        }
        self.after_current_editor_changed();
        self.status = if line_count > 1 {
            format!("pasted {line_count} lines")
        } else {
            format!("pasted {char_count} chars")
        };
    }

    pub fn extend_selection_left(&mut self) {
        if let Some(editor) = &mut self.editor {
            if editor.selection().is_none() {
                editor.start_selection();
            }
            editor.move_left();
            editor.extend_selection_to();
        }
    }

    pub fn extend_selection_right(&mut self) {
        if let Some(editor) = &mut self.editor {
            if editor.selection().is_none() {
                editor.start_selection();
            }
            editor.move_right();
            editor.extend_selection_to();
        }
    }

    pub fn extend_selection_up(&mut self) {
        if let Some(editor) = &mut self.editor {
            if editor.selection().is_none() {
                editor.start_selection();
            }
            editor.move_up();
            editor.extend_selection_to();
        }
    }

    pub fn extend_selection_down(&mut self) {
        if let Some(editor) = &mut self.editor {
            if editor.selection().is_none() {
                editor.start_selection();
            }
            editor.move_down();
            editor.extend_selection_to();
        }
    }

    pub fn clear_selection(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.clear_selection();
        }
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
