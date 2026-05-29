use crate::buffer::Position;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEvent {
    Opened { path: PathBuf, text: String },
    Changed { path: PathBuf, text: String },
    Saved { path: PathBuf, text: String },
    CursorMoved { path: PathBuf, position: Position },
}

pub trait DocumentEventSink {
    fn send(&mut self, event: DocumentEvent);
}

#[derive(Debug, Default)]
pub struct NullDocumentEventSink;

impl DocumentEventSink for NullDocumentEventSink {
    fn send(&mut self, _event: DocumentEvent) {}
}
