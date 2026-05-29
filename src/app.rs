pub mod command;

use crate::lsp::NullDocumentEventSink;
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
    pub lsp_sink: NullDocumentEventSink,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self { root, should_quit: false, lsp_sink: NullDocumentEventSink }
    }
}
