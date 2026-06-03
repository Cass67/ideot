use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
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
        let root_uri = format!("file://{}", root.display());
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

    let pending: Arc<Mutex<HashMap<u64, PendingKind>>> = Arc::new(Mutex::new(HashMap::new()));
    let initialized = Arc::new((Mutex::new(false), Condvar::new()));
    let next_id = Arc::new(AtomicU64::new(1));
    let (writer_tx, writer_rx) = mpsc::channel();

    let writer_handle = thread::spawn(move || {
        while let Ok(msg) = writer_rx.recv() {
            match msg {
                WriterMsg::Request { id, method, params } => {
                    write_jsonrpc_request(&mut stdin, id, &method, &params);
                }
                WriterMsg::Notification { method, params } => {
                    write_jsonrpc_notification(&mut stdin, &method, &params);
                }
                WriterMsg::Shutdown => break,
            }
        }
    });

    let init_id = allocate_request_id(&next_id);
    pending
        .lock()
        .unwrap()
        .insert(init_id, PendingKind::Initialize);
    let _ = writer_tx.send(WriterMsg::Request {
        id: init_id,
        method: "initialize".to_string(),
        params: json!({
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
    });

    let request_writer_tx = writer_tx.clone();
    let request_pending = Arc::clone(&pending);
    let request_initialized = Arc::clone(&initialized);
    let request_next_id = Arc::clone(&next_id);
    let request_handle = thread::spawn(move || {
        while let Ok(req) = req_rx.recv() {
            wait_for_initialized(&request_initialized);
            if !forward_lsp_request(req, &request_writer_tx, &request_pending, &request_next_id) {
                break;
            }
        }
    });

    let mut reader = BufReader::new(stdout);
    while let Ok(Some(params)) = read_jsonrpc_message(&mut reader) {
        // Route by id (response) or method (notification)
        if let Some(id) = params.get("id").and_then(|v| v.as_u64()) {
            let kind = pending.lock().unwrap().remove(&id);
            if let Some(kind) = kind {
                match kind {
                    PendingKind::Initialize => {
                        let _ = writer_tx.send(WriterMsg::Notification {
                            method: "initialized".to_string(),
                            params: json!({}),
                        });
                        let (lock, cvar) = &*initialized;
                        *lock.lock().unwrap() = true;
                        cvar.notify_all();
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

    let _ = writer_tx.send(WriterMsg::Shutdown);
    let _ = writer_handle.join();
    drop(request_handle);
}

enum WriterMsg {
    Request {
        id: u64,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    Shutdown,
}

fn allocate_request_id(next_id: &AtomicU64) -> u64 {
    next_id.fetch_add(1, Ordering::Relaxed) + 1
}

fn wait_for_initialized(initialized: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cvar) = &**initialized;
    let mut ready = lock.lock().unwrap();
    while !*ready {
        ready = cvar.wait(ready).unwrap();
    }
}

fn forward_lsp_request(
    req: LspRequest,
    writer_tx: &Sender<WriterMsg>,
    pending: &Arc<Mutex<HashMap<u64, PendingKind>>>,
    next_id: &AtomicU64,
) -> bool {
    match req {
        LspRequest::Init => true,
        LspRequest::DidOpen {
            uri,
            text,
            language,
        } => writer_tx
            .send(WriterMsg::Notification {
                method: "textDocument/didOpen".to_string(),
                params: json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": language,
                        "version": 1,
                        "text": text,
                    }
                }),
            })
            .is_ok(),
        LspRequest::DidChange { uri, text } => writer_tx
            .send(WriterMsg::Notification {
                method: "textDocument/didChange".to_string(),
                params: json!({
                    "textDocument": { "uri": uri, "version": 1 },
                    "contentChanges": [{ "text": text }]
                }),
            })
            .is_ok(),
        LspRequest::DidSave { uri, text } => writer_tx
            .send(WriterMsg::Notification {
                method: "textDocument/didSave".to_string(),
                params: json!({
                    "textDocument": { "uri": uri },
                    "text": text,
                }),
            })
            .is_ok(),
        LspRequest::DidClose { uri } => writer_tx
            .send(WriterMsg::Notification {
                method: "textDocument/didClose".to_string(),
                params: json!({ "textDocument": { "uri": uri } }),
            })
            .is_ok(),
        LspRequest::Hover { uri, position } => send_pending_request(
            writer_tx,
            pending,
            next_id,
            PendingKind::Hover,
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri }, "position": position }),
        ),
        LspRequest::Completion { uri, position } => send_pending_request(
            writer_tx,
            pending,
            next_id,
            PendingKind::Completion,
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": position }),
        ),
        LspRequest::Definition { uri, position } => send_pending_request(
            writer_tx,
            pending,
            next_id,
            PendingKind::Definition,
            "textDocument/definition",
            json!({ "textDocument": { "uri": uri }, "position": position }),
        ),
        LspRequest::Shutdown => writer_tx.send(WriterMsg::Shutdown).is_ok(),
    }
}

fn send_pending_request(
    writer_tx: &Sender<WriterMsg>,
    pending: &Arc<Mutex<HashMap<u64, PendingKind>>>,
    next_id: &AtomicU64,
    kind: PendingKind,
    method: &str,
    params: Value,
) -> bool {
    let id = allocate_request_id(next_id);
    pending.lock().unwrap().insert(id, kind);
    writer_tx
        .send(WriterMsg::Request {
            id,
            method: method.to_string(),
            params,
        })
        .is_ok()
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

fn write_jsonrpc_request(
    stdin: &mut std::process::ChildStdin,
    id: u64,
    method: &str,
    params: &Value,
) {
    write_jsonrpc_value(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }),
    );
}

fn write_jsonrpc_notification(stdin: &mut std::process::ChildStdin, method: &str, params: &Value) {
    write_jsonrpc_value(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }),
    );
}

fn write_jsonrpc_value(stdin: &mut std::process::ChildStdin, obj: &Value) {
    let payload = serde_json::to_string(obj).unwrap();
    let content_length = payload.len();
    let header = format!("Content-Length: {content_length}\r\n\r\n");

    let _ = stdin.write_all(header.as_bytes());
    let _ = stdin.write_all(payload.as_bytes());
    let _ = stdin.flush();
}

fn read_jsonrpc_message<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let Some(content_length) = content_length else {
        return Ok(None);
    };
    let mut payload = vec![0u8; content_length];
    reader.read_exact(&mut payload)?;
    let value = serde_json::from_slice(&payload)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    Ok(Some(value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_header_framed_jsonrpc_message() {
        let payload = r#"{"jsonrpc":"2.0","id":7,"result":{"contents":"hover text"}}"#;
        let bytes = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
        let mut reader = Cursor::new(bytes.into_bytes());

        let value = read_jsonrpc_message(&mut reader).unwrap().unwrap();

        assert_eq!(value["id"], 7);
        assert_eq!(value["result"]["contents"], "hover text");
    }

    #[test]
    fn initialized_message_is_notification_without_id() {
        let obj = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {},
        });

        assert!(obj.get("id").is_none());
    }
}
