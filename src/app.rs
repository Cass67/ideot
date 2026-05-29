pub mod command;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::fs::{ProjectFile, ProjectIndex};
use crate::lsp::{DocumentEvent, DocumentEventSink, NullDocumentEventSink};
use crate::marks::SessionMarks;
use crate::search::{RecentFiles, SearchIndex};
use anyhow::{Context, Result};
use std::path::PathBuf;

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

    pub fn current_relative(&self) -> Option<&str> {
        self.current_relative.as_deref()
    }

    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    pub fn marks(&self) -> &SessionMarks {
        &self.marks
    }
}
