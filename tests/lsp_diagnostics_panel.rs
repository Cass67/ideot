use ideot::app::App;
use ideot::lsp::{Diagnostic, DiagnosticSeverity, Position, Range};
use ideot::ui;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::tempdir;

fn diagnostic(line: u32, severity: DiagnosticSeverity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: Some(severity),
        message: message.to_string(),
    }
}

#[test]
fn next_diagnostic_moves_cursor_to_error_line() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "one\ntwo\nthree").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    let uri = app.current_lsp_uri().unwrap();
    app.diagnostics
        .update(uri, vec![diagnostic(2, DiagnosticSeverity::Error, "bad")]);

    app.next_diagnostic();

    assert_eq!(app.editor().unwrap().cursor().line, 2);
    assert!(app.status_line().contains("line 3"));
}

#[test]
fn diagnostics_panel_renders_messages() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "one\ntwo").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    let uri = app.current_lsp_uri().unwrap();
    app.diagnostics.update(
        uri,
        vec![diagnostic(1, DiagnosticSeverity::Warning, "careful")],
    );
    app.toggle_diagnostics_panel();

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let rendered = format!("{:?}", terminal.backend().buffer());

    assert!(rendered.contains("diagnostics"));
    assert!(rendered.contains("line 2"));
    assert!(rendered.contains("careful"));
}
