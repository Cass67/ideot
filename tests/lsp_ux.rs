use ideot::app::App;
use ideot::lsp::{Diagnostic, DiagnosticSeverity, Position, Range};
use ideot::ui;
use tempfile::tempdir;

fn diagnostic(line: u32, severity: DiagnosticSeverity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 5 },
        },
        severity: Some(severity),
        message: message.to_string(),
    }
}

#[test]
fn rust_files_report_rust_analyzer_status() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();

    assert!(
        app.status_line().contains("rust-analyzer"),
        "status should name rust-analyzer, got {}",
        app.status_line()
    );
}

#[test]
fn status_line_reports_lsp_unavailable_when_no_server_is_running() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "hello").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    assert_eq!(
        app.status_line(),
        "note.txt · LSP unavailable: no server configured"
    );
}

#[test]
fn status_line_summarizes_current_file_diagnostics_and_current_line() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "fn main() {}\nlet x =").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();
    let uri = app.current_lsp_uri().unwrap();
    app.diagnostics.update(
        uri,
        vec![
            diagnostic(0, DiagnosticSeverity::Warning, "unused function"),
            diagnostic(1, DiagnosticSeverity::Error, "expected expression"),
        ],
    );
    app.editor_mut()
        .unwrap()
        .set_cursor(ideot::buffer::Position { line: 1, column: 0 });

    assert_eq!(
        app.status_line(),
        "note.txt · LSP unavailable: no server configured · 1 error, 1 warning · expected expression"
    );
}

#[test]
fn editor_lines_are_prefixed_with_diagnostic_markers() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("note.txt"), "fn main() {}\nlet x =").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();
    let uri = app.current_lsp_uri().unwrap();
    app.diagnostics.update(
        uri,
        vec![
            diagnostic(0, DiagnosticSeverity::Warning, "unused function"),
            diagnostic(1, DiagnosticSeverity::Error, "expected expression"),
        ],
    );

    let rendered = ui::highlighted_editor_lines_for_height(&app, 2);
    assert!(format!("{:?}", rendered[0]).contains("!    1 warning │ "));
    assert!(format!("{:?}", rendered[1]).contains("✗    2 error │ "));
}
