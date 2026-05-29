# ideot

> A fast, minimal terminal code editor written in Rust.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- **Two-pane layout** — file explorer on the left, editor on the right, with per-pane focus
- **Fuzzy file search** — `Ctrl-P` with nucleo-based fuzzy matching and recent-file boost
- **Syntax highlighting** — Tree-sitter powered for Rust, Go, Python, JavaScript, Markdown, TOML, and YAML
- **Git commit browser** — `Ctrl-G` to browse commits, select files, and view side-by-side or unified diffs
- **Session marks** — `Ctrl-M` to mark a file, `Ctrl-1` through `Ctrl-9` to jump back
- **File actions** — create new files and delete existing ones from the explorer
- **Page up/down scrolling** — both in the editor and the git diff viewer
- **LSP-ready** — document event boundary (open, change, save, cursor move) for future LSP integration

## Quick Start

```bash
cargo run
```

## Build

```bash
# Development build
cargo run

# Release binary
cargo build --release
```

Binary path: `target/release/ideot`

## Keybindings

| Key | Action |
|---|---|
| `Up` / `Down` | Navigate file explorer |
| `Enter` | Open selected file |
| `Ctrl-S` | Save current file |
| `Ctrl-P` | Open fuzzy file search |
| `Ctrl-G` | Open git commit browser |
| `Ctrl-M` | Mark current file |
| `Ctrl-1` .. `Ctrl-9` | Jump to mark 1-9 |
| `Ctrl-/` | Toggle focus between explorer and editor |
| `Ctrl-Q` | Quit |
| `F1` | Toggle help overlay |

### Git Browser (`Ctrl-G`)

1. Select a commit from the list
2. Select a changed file
3. View the diff in split or unified layout
4. `Esc` to go back, `Ctrl-Q` to close

### File Search (`Ctrl-P`)

Type to fuzzy-match file paths. Recently opened files are ranked higher.

### Marks

Press `Ctrl-M` to mark the current file, then `Ctrl-1` through `Ctrl-9` to jump back. Marks persist for the session and auto-reuse slots when revisiting the same file.

## Architecture

```
ideot/
├── src/
│   ├── main.rs          # Entry point, event loop
│   ├── app.rs            # Application state, command routing
│   ├── editor.rs         # Cursor movement, insert, backspace
│   ├── buffer.rs         # Line-based text buffer
│   ├── fs.rs             # Project file index (gitignore-aware)
│   ├── search.rs         # Fuzzy search with recent-file boost
│   ├── git.rs            # Git commit browser, diff alignment
│   ├── highlight.rs      # Tree-sitter syntax highlighting
│   ├── marks.rs          # Session mark slots
│   ├── lsp.rs            # Document event boundary
│   └── ui.rs             # TUI rendering (ratatui)
├── tests/                # Integration tests
└── Cargo.toml
```

### Dependencies

- [ratatui](https://ratatui.rs) — terminal UI framework
- [crossterm](https://crates.io/crates/crossterm) — terminal I/O
- [tree-sitter](https://tree-sitter.github.io) — incremental parsing for syntax highlighting
- [nucleo-matcher](https://crates.io/crates/nucleo-matcher) — fuzzy matching
- [ignore](https://crates.io/crates/ignore) — gitignore-aware file walking

## Design Philosophy

- **Fast** — single binary, no network calls, incremental parsing
- **Minimal** — core editing, file search, git browsing. Nothing more.
- **Terminal-native** — works in any terminal, no GUI dependencies
- **Composable** — clean boundaries between editor, UI, and filesystem layers

## License

MIT
