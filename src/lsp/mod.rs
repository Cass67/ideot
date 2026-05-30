mod client;
mod diagnostics;
mod discover;
mod protocol;

use std::path::PathBuf;

pub use client::{LspClient, LspMsg};
pub use diagnostics::DiagnosticsStore;
pub use discover::*;
pub use protocol::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEvent {
    Opened {
        path: PathBuf,
        text: String,
    },
    Changed {
        path: PathBuf,
        text: String,
    },
    Saved {
        path: PathBuf,
        text: String,
    },
    CursorMoved {
        path: PathBuf,
        position: crate::buffer::Position,
    },
}

pub trait DocumentEventSink {
    fn send(&mut self, event: DocumentEvent);
}

#[derive(Debug, Default)]
pub struct NullDocumentEventSink;

impl DocumentEventSink for NullDocumentEventSink {
    fn send(&mut self, _event: DocumentEvent) {}
}
