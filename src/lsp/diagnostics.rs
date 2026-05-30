use std::collections::HashMap;

use super::Diagnostic;

/// Stores diagnostics by document URI.
#[derive(Debug)]
pub struct DiagnosticsStore {
    map: HashMap<String, Vec<Diagnostic>>,
}

impl DiagnosticsStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn update(&mut self, uri: String, diagnostics: Vec<Diagnostic>) {
        self.map.insert(uri, diagnostics);
    }

    pub fn get(&self, uri: &str) -> Option<&[Diagnostic]> {
        self.map.get(uri).map(|v| v.as_slice())
    }

    pub fn clear(&mut self, uri: &str) {
        self.map.remove(uri);
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl Default for DiagnosticsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::{Position, Range};

    fn make_diagnostic(
        severity: Option<crate::lsp::DiagnosticSeverity>,
        message: &str,
    ) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 5,
                },
            },
            severity,
            message: message.to_string(),
        }
    }

    #[test]
    fn insert_and_query() {
        let mut store = DiagnosticsStore::new();
        let uri = "file:///test.rs";
        let diags = vec![make_diagnostic(None, "test diag")];

        store.update(uri.to_string(), diags);
        let result = store.get(uri).expect("should have diagnostics");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].message, "test diag");
    }

    #[test]
    fn update_overwrites() {
        let mut store = DiagnosticsStore::new();
        let uri = "file:///test.rs";

        store.update(uri.to_string(), vec![make_diagnostic(None, "first")]);
        assert_eq!(store.get(uri).unwrap()[0].message, "first");

        store.update(uri.to_string(), vec![make_diagnostic(None, "second")]);
        assert_eq!(store.get(uri).unwrap()[0].message, "second");
    }

    #[test]
    fn missing_uri_returns_none() {
        let store = DiagnosticsStore::new();
        assert!(store.get("file:///nope.rs").is_none());
    }

    #[test]
    fn clear_removes() {
        let mut store = DiagnosticsStore::new();
        let uri = "file:///test.rs";
        store.update(uri.to_string(), vec![make_diagnostic(None, "x")]);
        assert!(store.get(uri).is_some());

        store.clear(uri);
        assert!(store.get(uri).is_none());
    }
}
