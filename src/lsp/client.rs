use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use serde_json::{json, Value};

use super::discover::discover_server;
use super::{
    CompletionItem, CompletionKind, Diagnostic, DiagnosticSeverity, HoverInfo, Location, Position,
    Range,
};

/// Request from the main thread to the LSP background thread.
#[allow(dead_code)]
enum LspRequest {
    /// Send initialize + initialized (must be first).
    Init,
    /// textDocument/didOpen
    DidOpen {
        uri: String,
        text: String,
        language: String,
    },
    /// textDocument/didChange (full document replacement)
    DidChange { uri: String, text: String },
    /// textDocument/didSave
    DidSave { uri: String, text: String },
    /// textDocument/didClose
    DidClose { uri: String },
    /// textDocument/hover
    Hover { uri: String, position: Position },
    /// textDocument/completion
    Completion { uri: String, position: Position },
    /// textDocument/definition
    Definition { uri: String, position: Position },
    /// Shut down and exit.
    Shutdown,
}

/// Response from the LSP background thread to the main thread.
pub enum LspMsg {
    /// textDocument/publishDiagnostics (push notification)
    Diagnostics {
        uri: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// Response to hover request.
    Hover(Option<HoverInfo>),
    /// Response to completion request.
    Completion(Option<Vec<CompletionItem>>),
    /// Response to go-to-definition request.
    Definition(Option<Vec<Location>>),
}

/// LSP client. Spawns the language server on a background thread.
#[derive(Debug)]
pub struct LspClient {
    server_tx: Sender<LspRequest>,
    msg_rx: Receiver<LspMsg>,
    _handle: Option<JoinHandle<()>>,
}

impl LspClient {
    /// Spawn the LSP server for the given file extension and workspace root.
    /// Returns `None` if no server is found for the extension.
    pub fn init(ext: &str, root: &Path) -> Option<Self> {
        let server = discover_server(ext)?;
        let root_uri = root.to_string_lossy().to_string();
        Some(Self::spawn(server, root_uri))
    }

    fn spawn(server: super::discover::LanguageServer, root_uri: String) -> Self {
        let (server_tx, server_rx) = mpsc::channel();
        let (msg_tx, msg_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_server(server, root_uri, server_rx, msg_tx);
        });

        Self {
            server_tx,
            msg_rx,
            _handle: Some(handle),
        }
    }

    /// Send textDocument/didOpen.
    pub fn did_open(&self, uri: &str, text: &str, language: &str) {
        self.server_tx
            .send(LspRequest::DidOpen {
                uri: uri.to_string(),
                text: text.to_string(),
                language: language.to_string(),
            })
            .ok();
    }

    /// Send textDocument/didChange (full document).
    pub fn did_change(&self, uri: &str, text: &str) {
        self.server_tx
            .send(LspRequest::DidChange {
                uri: uri.to_string(),
                text: text.to_string(),
            })
            .ok();
    }

    /// Send textDocument/didSave.
    pub fn did_save(&self, uri: &str, text: &str) {
        self.server_tx
            .send(LspRequest::DidSave {
                uri: uri.to_string(),
                text: text.to_string(),
            })
            .ok();
    }

    /// Send textDocument/didClose.
    pub fn did_close(&self, uri: &str) {
        self.server_tx
            .send(LspRequest::DidClose {
                uri: uri.to_string(),
            })
            .ok();
    }

    /// Request hover info. Response arrives via `poll()`.
    pub fn hover(&self, uri: &str, position: Position) {
        self.server_tx
            .send(LspRequest::Hover {
                uri: uri.to_string(),
                position,
            })
            .ok();
    }

    /// Request completions. Response arrives via `poll()`.
    pub fn completion(&self, uri: &str, position: Position) {
        self.server_tx
            .send(LspRequest::Completion {
                uri: uri.to_string(),
                position,
            })
            .ok();
    }

    /// Request go-to-definition. Response arrives via `poll()`.
    pub fn definition(&self, uri: &str, position: Position) {
        self.server_tx
            .send(LspRequest::Definition {
                uri: uri.to_string(),
                position,
            })
            .ok();
    }

    /// Poll for LSP messages (non-blocking).
    pub fn poll(&self) -> Option<LspMsg> {
        self.msg_rx
            .recv_timeout(std::time::Duration::from_millis(0))
            .ok()
    }
}

// ─── Background Thread ───────────────────────────────────────────────

fn run_server(
    server: super::discover::LanguageServer,
    root_uri: String,
    req_rx: Receiver<LspRequest>,
    msg_tx: Sender<LspMsg>,
) {
    let mut cmd = Command::new(server.command);
    cmd.args(server.args.iter().copied());
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to spawn LSP server {}: {}", server.command, e);
            return;
        }
    };

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    // Pending request id -> callback kind
    let mut pending: HashMap<u64, PendingKind> = HashMap::new();
    let mut id_gen = 1u64;

    let mut next_id = || -> u64 {
        id_gen += 1;
        id_gen
    };

    // Send initialize
    let init_id = next_id();
    pending.insert(init_id, PendingKind::Initialize);
    write_jsonrpc(
        &mut stdin,
        init_id,
        "initialize",
        &json!({
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": { "dynamicRegistration": false },
                    "completion": { "dynamicRegistration": false },
                    "definition": { "dynamicRegistration": false },
                    "publishDiagnostics": { "relatedInformation": false }
                }
            },
            "initializationOptions": null
        }),
    );

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let params: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Route by id (response) or method (notification)
        if let Some(id) = params.get("id").and_then(|v| v.as_u64()) {
            if let Some(kind) = pending.remove(&id) {
                match kind {
                    PendingKind::Initialize => {
                        // Send initialized notification (no id)
                        write_jsonrpc(&mut stdin, 0, "initialized", &json!({}));
                    }
                    PendingKind::Hover => {
                        let info = parse_hover_result(&params);
                        msg_tx.send(LspMsg::Hover(info)).ok();
                    }
                    PendingKind::Completion => {
                        let items = parse_completion_result(&params);
                        msg_tx.send(LspMsg::Completion(items)).ok();
                    }
                    PendingKind::Definition => {
                        let locs = parse_definition_result(&params);
                        msg_tx.send(LspMsg::Definition(locs)).ok();
                    }
                }
            }
        }

        // Notifications (no id)
        if let Some(method) = params.get("method").and_then(|v| v.as_str()) {
            if method == "textDocument/publishDiagnostics" {
                if let Some(p) = params.get("params") {
                    let uri = p
                        .get("uri")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let diagnostics: Vec<Diagnostic> = p
                        .get("diagnostics")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(parse_diagnostic).collect())
                        .unwrap_or_default();
                    msg_tx.send(LspMsg::Diagnostics { uri, diagnostics }).ok();
                }
            }
        }
    }

    // Drain remaining requests so the main thread doesn't hang on send
    while req_rx.try_recv().is_ok() {}
}

/// What kind of response we're waiting for.
#[allow(dead_code)]
enum PendingKind {
    Initialize,
    Hover,
    Completion,
    Definition,
}

// ─── JSON-RPC Writer ─────────────────────────────────────────────────

fn write_jsonrpc(stdin: &mut std::process::ChildStdin, id: u64, method: &str, params: &Value) {
    let obj = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });

    let payload = serde_json::to_string(&obj).unwrap();
    let content_length = payload.len();
    let header = format!("Content-Length: {content_length}\r\n\r\n");

    let _ = stdin.write_all(header.as_bytes());
    let _ = stdin.write_all(payload.as_bytes());
}

// ─── Parsing Helpers ─────────────────────────────────────────────────

fn parse_diagnostic(v: &Value) -> Option<Diagnostic> {
    let range = parse_range(v.get("range")?)?;
    let severity = v.get("severity").and_then(|s| {
        Some(match s.as_i64()? {
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Information,
            4 => DiagnosticSeverity::Hint,
            _ => return None,
        })
    });
    let message = v.get("message")?.as_str()?.to_string();
    Some(Diagnostic {
        range,
        severity,
        message,
    })
}

fn parse_position(v: &Value) -> Option<Position> {
    Some(Position {
        line: v.get("line")?.as_u64()? as u32,
        character: v.get("character")?.as_u64()? as u32,
    })
}

fn parse_range(v: &Value) -> Option<Range> {
    Some(Range {
        start: parse_position(v.get("start")?)?,
        end: parse_position(v.get("end")?)?,
    })
}

fn parse_hover_result(params: &Value) -> Option<HoverInfo> {
    let result = params.get("result")?;
    let contents = parse_hover_contents(result)?;
    let range = result.get("range").and_then(parse_range);
    Some(HoverInfo { contents, range })
}

fn parse_hover_contents(result: &Value) -> Option<String> {
    let contents = result.get("contents")?;
    // MarkupContent { kind, value }
    if let Some(value) = contents.get("value") {
        return Some(value.as_str()?.to_string());
    }
    // Plain string
    if let Some(s) = contents.as_str() {
        return Some(s.to_string());
    }
    // Array of MarkedStrings
    if let Some(arr) = contents.as_array() {
        return Some(
            arr.iter()
                .filter_map(|ms| {
                    ms.get("value")
                        .and_then(|v| v.as_str())
                        .or_else(|| ms.as_str())
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    None
}

fn parse_completion_result(params: &Value) -> Option<Vec<CompletionItem>> {
    let result = params.get("result")?;
    // Could be an array or a CompletionList
    let items = if let Some(arr) = result.as_array() {
        arr
    } else {
        result.get("items")?.as_array()?
    };

    Some(
        items
            .iter()
            .filter_map(|item| {
                let label = item.get("label")?.as_str()?.to_string();
                let kind = item
                    .get("kind")
                    .and_then(|k| k.as_i64())
                    .map(completion_kind_from_i64);
                let detail = item
                    .get("detail")
                    .and_then(|d| d.as_str())
                    .map(String::from);
                Some(CompletionItem {
                    label,
                    kind: kind.unwrap_or(CompletionKind::Text),
                    detail,
                })
            })
            .collect(),
    )
}

fn completion_kind_from_i64(v: i64) -> CompletionKind {
    match v {
        1 => CompletionKind::Text,
        2 => CompletionKind::Method,
        3 => CompletionKind::Function,
        4 => CompletionKind::Constructor,
        5 => CompletionKind::Field,
        6 => CompletionKind::Variable,
        7 => CompletionKind::Class,
        8 => CompletionKind::Interface,
        9 => CompletionKind::Module,
        10 => CompletionKind::Property,
        13 => CompletionKind::Unit,
        14 => CompletionKind::Value,
        15 => CompletionKind::Enum,
        16 => CompletionKind::Keyword,
        17 => CompletionKind::Snippet,
        18 => CompletionKind::Reference,
        _ => CompletionKind::Text,
    }
}

fn parse_definition_result(params: &Value) -> Option<Vec<Location>> {
    let result = params.get("result")?;
    let mut locations = Vec::new();

    // Single Location
    if let Some(loc) = parse_location(result) {
        locations.push(loc);
    }
    // Array of Locations
    if let Some(arr) = result.as_array() {
        for item in arr {
            if let Some(loc) = parse_location(item) {
                locations.push(loc);
            }
        }
    }

    Some(locations)
}

fn parse_location(v: &Value) -> Option<Location> {
    let uri = v.get("uri")?.as_str()?.to_string();
    let range = parse_range(v.get("range")?)?;
    Some(Location { uri, range })
}
