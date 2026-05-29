pub mod command;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::fs::{ProjectFile, ProjectIndex};
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
    explorer_scroll: usize,
    editor_scroll: usize,
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
            explorer_scroll: 0,
            editor_scroll: 0,
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
            if let (Some(relative), Some(path)) = (self.current_relative.as_ref(), editor.buffer().path()) {
                self.lsp_sink.send(DocumentEvent::Changed { path: path.to_path_buf(), text: editor.buffer().text() });
                self.recent.record(relative.clone());
            }
        }
    }

    pub fn backspace(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.backspace();
            if let Some(path) = editor.buffer().path() {
                self.lsp_sink.send(DocumentEvent::Changed { path: path.to_path_buf(), text: editor.buffer().text() });
            }
        }
    }

    pub fn save_current(&mut self) -> Result<()> {
        let editor = self.editor.as_mut().context("no open file")?;
        editor.buffer_mut().save()?;
        if let Some(path) = editor.buffer().path() {
            self.lsp_sink.send(DocumentEvent::Saved { path: path.to_path_buf(), text: editor.buffer().text() });
        }
        self.status = "saved".to_string();
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<ProjectFile> {
        self.search_index.query(query, &self.recent)
    }

    pub fn explorer_entries(&self) -> Vec<ExplorerEntry> {
        let Some(index) = &self.index else { return Vec::new(); };
        let mut dirs = BTreeSet::new();
        let mut files = Vec::new();
        for file in index.files() {
            if let Some((dir, _rest)) = file.relative.split_once('/') {
                dirs.insert(dir.to_string());
            } else {
                files.push(file.relative.clone());
            }
        }
        let mut entries: Vec<ExplorerEntry> = dirs
            .into_iter()
            .map(|dir| ExplorerEntry { label: format!("▸ {dir}"), relative: None, is_dir: true })
            .collect();
        entries.extend(files.into_iter().map(|file| ExplorerEntry { label: format!("  {file}"), relative: Some(file), is_dir: false }));
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
        let query = if self.search_open { self.search_query.as_str() } else { "" };
        let max = self.search(query).len().saturating_sub(1);
        self.selected_file = (self.selected_file + 1).min(max);
    }

    pub fn move_selection_up(&mut self) {
        self.selected_file = self.selected_file.saturating_sub(1);
    }

    pub fn selected_file(&self) -> usize {
        self.selected_file
    }

    pub fn open_selected(&mut self) -> Result<()> {
        let query = if self.search_open { self.search_query.as_str() } else { "" };
        let files = self.search(query);
        if let Some(file) = files.get(self.selected_file) {
            let relative = file.relative.clone();
            self.open_relative(&relative)?;
        }
        Ok(())
    }

    pub fn toggle_search(&mut self) {
        self.search_open = !self.search_open;
        self.search_query.clear();
        self.selected_file = 0;
    }

    pub fn search_open(&self) -> bool {
        self.search_open
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
        self.explorer_scroll += 1;
    }

    pub fn scroll_explorer_up(&mut self) {
        self.explorer_scroll = self.explorer_scroll.saturating_sub(1);
    }

    pub fn scroll_editor_down(&mut self) {
        self.editor_scroll += 1;
    }

    pub fn scroll_editor_up(&mut self) {
        self.editor_scroll = self.editor_scroll.saturating_sub(1);
    }
}
