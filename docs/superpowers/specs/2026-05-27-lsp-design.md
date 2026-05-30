# LSP Client for ideot

## Summary

Implement a full LSP client: diagnostics, hover, completion, and go-to-definition. The LSP server process runs on a background thread communicating via `std::sync::mpsc` channels. The main event loop polls LSP messages between terminal event ticks. Language servers are auto-discovered by file extension.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Main Thread (event loop)                            │
│  ┌──────────┐  poll()  ┌──────────┐                 │
│  │crossterm │─────────▶│ App      │                 │
│  │events    │           │ state    │                 │
│  └──────────┘           └────┬─────┘                 │
│                               │                       │
│  ┌──────────┐  recv()        │                       │
│  │mpsc::Recv│◀───────────────┤                       │
│  │(LspMsg)  │                │  send()              │
│  └──────────┘                │─────────────────────▶│
│                               │     mpsc::Send       │
│  ┌────────────────────────────┴────────────────────┐ │
│  │              Background Thread                   │ │
│  │  ┌──────────────────────────────────────────┐   │ │
│  │  │  rust-analyzer / gopls / pylsp / tsserver │   │ │
│  │  │  (child process, stdin/stdout JSON-RPC)   │   │ │
│  │  └──────────────────────────────────────────┘   │ │
│  └──────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Message Flow

**App → LSP (via `mpsc::Sender<LspRequest>`):**
- `Hover { uri, position, oneshot }` — request hover info
- `Completion { uri, position, oneshot }` — request completions
- `Definition { uri, position, oneshot }` — request go-to-definition

**LSP → App (via `mpsc::Receiver<LspMsg>`):**
- `Diagnostics { uri, diagnostics }` — from `textDocument/publishDiagnostics`
- `Hover(Option<HoverInfo>)` — response to hover request
- `Completion(Option<Vec<CompletionItem>>)` — response to completion request
- `Definition(Option<Vec<Location>>)` — response to go-to-definition request

### Key Design Decisions

1. **No `lsp-types` crate** — we define only the LSP types we need. The protocol is simple enough and the crate pulls in ~200KB of types we won't use.

2. **One LSP server per language** — not per file. Opening `foo.rs` and `bar.rs` shares one `rust-analyzer` instance.

3. **Blocking initialization** — `LspClient::init()` spawns the process and blocks until the server responds to `initialize`. This only happens once per language on first file open.

4. **Oneshot channels for requests** — each LSP request carries its own `oneshot::Sender` so the response routes back correctly without a global callback registry.

## LSP Protocol Types

Minimal types we need, mirroring LSP 3.17:

```rust
// Position: zero-based line and character offsets
pub struct Position { pub line: u32, pub character: u32 }

// Range between two positions
pub struct Range { pub start: Position, pub end: Position }

// Diagnostic severity
pub enum DiagnosticSeverity { Error, Warning, Information, Hint }

// A diagnostic from the language server
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub message: String,
}

// Hover content
pub struct HoverInfo {
    pub contents: String,  // markdown or plain text
    pub range: Option<Range>,
}

// Completion item
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,  // Text, Method, Function, Variable, ...
    pub detail: Option<String>,
}

// Location for go-to-definition
pub struct Location {
    pub uri: String,
    pub range: Range,
}
```

## Auto-Discovery

Map file extensions to language server commands. Check `which <cmd>` before attempting init.

| Extension | Server | Args |
|---|---|---|
| `.rs` | `rust-analyzer` | *(none)* |
| `.go` | `gopls` | *(none)* |
| `.py` | `pylsp` | *(none)* |
| `.js`, `.jsx`, `.mjs` | `typescript-language-server` | `--stdio` |
| `.ts`, `.tsx` | `typescript-language-server` | `--stdio` |
| `.vue` | `vue-language-server` | `--stdio` |

If the command is not found on PATH, silently skip — no LSP for that language.

## UI Design

### Diagnostics

Inline markers in the left margin of the editor, 2 columns wide:

```
  ┌─ editor ─────────────────────────────────────┐
E │ use std::collections::HashMap;               │
  │                                               │
W │ let x: HashMap = HashMap::new();             │
  │                                               │
  │ fn main() { println!("hello"); }             │
  └───────────────────────────────────────────────┘
```

- `E` — red, for errors
- `W` — yellow, for warnings
- Blank — no diagnostic on that line
- Status bar shows `3 errors, 2 warnings` when diagnostics exist for the current file

### Hover

Keybinding: `Ctrl-H`

Shows in a popup at the bottom of the screen (above the status bar), similar to the search overlay but smaller and non-interactive:

```
┌─ hover ──────────────────────────────────────────┐
│ struct HashMap<K, V, S = RandomState>            │
│ A hash map powered by SIMD-concurrent hashing... │
└───────────────────────────────────────────────────┘
```

Cleared when user presses `Esc` or moves the cursor.

### Completion

Keybinding: `Ctrl-/`

Shows a centered overlay with a scrollable list of completions:

```
┌─ completion ──────────────────────────────────────┐
│ > HashMap                                          │
│   HashSet                                          │
│   Hash                                             │
│   hash                                             │
└───────────────────────────────────────────────────┘
```

- `Enter` — insert selected completion at cursor
- `Up`/`Down` — navigate
- `Esc` — close

### Go-to-Definition

Keybinding: `Ctrl-]`

Opens the file at the definition location and places the cursor at the position. If the file is outside the workspace, shows "file outside workspace" in status bar.

## App Integration

### New State in `App`

```rust
pub struct App {
    // ... existing fields ...
    lsp: LspClient,
    diagnostics: DiagnosticsStore,
    hover_popup: Option<HoverInfo>,
    completion_popup: Option<Vec<CompletionItem>>,
}
```

### LSP Client

```rust
pub struct LspClient {
    server_tx: mpsc::Sender<LspRequest>,  // to background thread
    msg_rx: mpsc::Receiver<LspMsg>,        // from background thread
}

impl LspClient {
    pub fn init(language: &str, root: &Path) -> Option<Self>;
    pub fn did_open(&self, uri: &str, text: &str, language: &str);
    pub fn did_change(&self, uri: &str, text: &str);
    pub fn did_save(&self, uri: &str, text: &str);
    pub fn did_close(&self, uri: &str);
    pub fn hover(&self, uri: &str, position: Position) -> Option<oneshot::Sender<HoverInfo>>;
    pub fn completion(&self, uri: &str, position: Position) -> Option<oneshot::Sender<Vec<CompletionItem>>>;
    pub fn definition(&self, uri: &str, position: Position) -> Option<oneshot::Sender<Vec<Location>>>;
    pub fn poll(&self) -> Option<LspMsg>;  // non-blocking recv
}
```

### Diagnostics Store

```rust
pub struct DiagnosticsStore {
    // uri -> diagnostics
    map: HashMap<String, Vec<Diagnostic>>,
}

impl DiagnosticsStore {
    pub fn update(&mut self, uri: String, diagnostics: Vec<Diagnostic>);
    pub fn get(&self, uri: &str) -> Option<&[Diagnostic]>;
}
```

### Event Loop Changes

In `main.rs`, after polling the terminal event:

```rust
// Poll LSP messages (non-blocking)
while let Ok(msg) = app.lsp.poll() {
    match msg {
        LspMsg::Diagnostics { uri, diagnostics } => {
            app.diagnostics.update(uri, diagnostics);
        }
        LspMsg::Hover(h) => app.hover_popup = h,
        LspMsg::Completion(c) => app.completion_popup = c,
        LspMsg::Definition(locs) => handle_definition(locs, app),
    }
}
```

### Document Event Wiring

Replace `NullDocumentEventSink` with real LSP calls:

- `Opened` → `lsp.did_open()`
- `Changed` → `lsp.did_change()`
- `Saved` → `lsp.did_save()`
- `CursorMoved` → *(not sent to LSP directly; hover/completion requests fire on keybinding)*

## Files

### New Files
- `src/lsp/client.rs` — LspClient, LspRequest, LspMsg, background thread
- `src/lsp/protocol.rs` — LSP JSON types (Position, Range, Diagnostic, HoverInfo, CompletionItem, Location)
- `src/lsp/discover.rs` — auto-discovery map and `which` check
- `src/lsp/diagnostics.rs` — DiagnosticsStore

### Modified Files
- `src/lsp.rs` — re-export from submodules (rename current `lsp.rs` content or fold in)
- `src/app.rs` — add `lsp`, `diagnostics`, `hover_popup`, `completion_popup` fields; wire document events
- `src/ui.rs` — render diagnostic markers, hover popup, completion popup
- `src/main.rs` — add LSP polling, add keybindings for hover/completion/definition
- `Cargo.toml` — add `serde_json = "1"`

## Testing

- Unit test: `DiagnosticsStore` insert/query
- Unit test: `discover.rs` — known extension maps to correct server
- Integration test: mock LSP server via stdin/stdout, verify `textDocument/didOpen` is sent on file open
- Integration test: mock server sends `publishDiagnostics`, verify `DiagnosticsStore` receives it

## Out of Scope
- Rename, references, code actions, signature help
- LSP server configuration via config file
- Inlay hints
- Code lens
- Formatting

