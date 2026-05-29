# Terminal Code Editor Design

## Summary

Build a fast, minimal Rust terminal code editor with a classic two-pane interface: a file explorer on the left and an editable file buffer on the right. The editor prioritizes fast file navigation, syntax-highlighted editing, and a simple single-binary deployment model.

The MVP includes an LSP-ready architecture but does not implement LSP behavior until the first post-MVP milestone.

## Product Scope

### MVP Features

- Classic two-pane terminal UI.
- Left panel project file explorer.
- Right panel editable file buffer.
- `Ctrl-P` fuzzy file search with recently opened files boosted.
- Temporary Harpoon-like session marks.
- Pluggable syntax highlighting with Tree-sitter-backed highlighters for bundled languages and plain-text fallback.
- Normal terminal editing controls rather than Vim-style modal editing.
- Single deployable Rust binary.
- LSP-ready internal seams: document open, change, save, and cursor-position events are modeled even though no LSP client runs in the MVP.

### Out of Scope for MVP

- Actual LSP client behavior.
- Plugins.
- Tabs or splits beyond the fixed explorer/editor layout.
- Persistent marks or project config.
- Content grep.
- Rope-backed internals, though buffer interfaces should allow a later rope implementation.

## Recommended Approach

Use Rust with `ratatui` and `crossterm`, plus a custom editor core.

This approach gives maximum control over performance, layout, keybindings, and future editor behavior while preserving the single-binary deployment goal. It is more work than embedding an existing editor engine, but it keeps the product small and focused.

## Architecture

Recommended stack:

- TUI/rendering: `ratatui` and `crossterm`.
- File walking: `ignore` crate for `.gitignore`-aware traversal.
- Fuzzy search: a Rust fuzzy matcher such as `nucleo` or a comparable library.
- Syntax highlighting: `tree-sitter` plus bundled grammars behind a `Highlighter` trait.
- Editing core: custom `Buffer` abstraction, initially backed by simple full-file text storage.
- Event model: one app event loop with explicit focus/mode state.

Core modules:

```text
app          event loop, commands, global state
ui           ratatui rendering/layout/components
fs           project tree, ignore-aware file discovery
buffer       editable document model
editor       cursor, movement, insert/delete/save behavior
search       fuzzy file picker and recent-file boosting
marks        temporary session marks
highlight    Tree-sitter/plain highlighter interface
lsp          placeholder traits/events for future LSP integration
config       future defaults/keybinding configuration
```

## User Experience

Default layout:

```text
┌ file explorer ─────┬ editor ─────────────────────────────┐
│ ▾ src              │ src/main.rs                         │
│   main.rs          │  1 use ratatui::prelude::*;          │
│   app.rs           │  2                                  │
│ ▸ docs             │  3 fn main() {                      │
│ README.md          │  4     // ...                       │
└────────────────────┴─────────────────────────────────────┘
```

MVP keybindings:

- `Tab`: switch focus between explorer and editor.
- Arrow keys and PageUp/PageDown: move in focused pane.
- `Enter`: open selected file from explorer or search.
- Normal typing: insert text in editor.
- `Backspace` and `Delete`: edit text.
- `Ctrl-S`: save current file.
- `Ctrl-P`: open quick file search.
- `Ctrl-M`: mark current file for the session.
- `Ctrl-1` through `Ctrl-9`: jump to session mark.
- `Ctrl-Q`: quit, with dirty-buffer confirmation.

The status bar shows current focus, file path, dirty state, line/column, and short errors.

## Data Flow

1. On startup, open the current directory as the project root.
2. Build an ignore-aware file index for explorer and search.
3. Opening a file loads it into a `Buffer`.
4. Editor mutations update the buffer and mark it dirty.
5. Rendering requests visible buffer lines and highlight spans for the viewport.
6. Saving writes the buffer back to disk.
7. Recent files and session marks update in memory.
8. LSP-ready events are emitted for document open/change/save and cursor changes, but are ignored until the LSP milestone.

## Error Handling

- Unreadable files show a status-bar error and do not crash the app.
- Binary or huge files are blocked or opened as plain-text previews depending on size/type thresholds.
- Save failures keep the buffer dirty and show an error.
- Quitting with dirty buffers asks for confirmation.
- Tree-sitter setup or grammar failures fall back to plain text.

## Testing Strategy

- Unit tests for buffer editing operations.
- Unit tests for file index filtering and fuzzy ordering.
- Unit tests for session mark behavior.
- Render tests for important UI states where practical.
- Integration tests for open, edit, save flows using temporary directories.
- Manual terminal smoke testing for keybindings, resizing, and rendering.

## Post-MVP Milestone 1: LSP

Add a real LSP client after the core editor is usable. The first LSP milestone should include:

- Language server process management.
- JSON-RPC transport.
- Document sync from existing buffer events.
- Diagnostics display.
- Hover display.
- Go-to-definition.
- Minimal per-language server configuration.

This milestone should reuse the MVP's `lsp` boundary rather than changing buffer/editor internals directly.
