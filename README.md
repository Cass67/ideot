# ideot

> A fast, minimal terminal code editor written in Rust.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- **Two-pane layout** — file explorer on the left, editor on the right, with per-pane focus
- **Fuzzy file search** — `Ctrl-P` with nucleo-based fuzzy matching and recent-file boost
- **Syntax highlighting** — Tree-sitter powered for Rust, Go, Python, JavaScript/TypeScript, JSON, Bash, HTML, CSS, C/C++, Lua, Markdown, TOML, and YAML
- **Git commit browser** — `Ctrl-G` to browse commits, select files, and view side-by-side or unified diffs
- **Session marks** — `Ctrl-M` to mark a file, `Ctrl-1` through `Ctrl-9` to jump back
- **File actions** — create new files and delete existing ones from the explorer
- **Page up/down scrolling** — both in the editor and the git diff viewer
- **LSP diagnostics and hover** — optional language servers for diagnostics, status, hover, completion data, and go-to-definition plumbing
- **Remembered UI toggles** — file pane, line numbers, LSP, and LSP hover settings persist across runs

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
| `Up` / `Down` | Navigate focused pane |
| `Mouse drag` | Select editor text inside ideot |
| terminal modifier drag | Use terminal-native selection escape hatch |
| `Shift+Arrows` | Extend text selection |
| `Ctrl-A` | Select all text in the current editor buffer |
| `Y` / `Ctrl-Shift-C` | Copy selection to system clipboard |
| `Ctrl-V` | Paste from system clipboard |
| `U` | Undo last edit |
| `Ctrl-R` | Redo last undone edit |
| `Enter` | Open selected file / insert newline in editor |
| `Ctrl-S` | Save current file |
| `Ctrl-P` | Open fuzzy file search |
| `Ctrl-G` | Open git commit browser |
| `Ctrl-B` | Toggle file pane on/off, remembered |
| `Ctrl-T` | Toggle line numbers on/off, remembered |
| `Ctrl-L` | Toggle LSP on/off, remembered |
| `Ctrl-O` | Toggle LSP hover on/off, remembered |
| `Ctrl-U` | Toggle LSP diagnostics display on/off, remembered (default off) |
| `Ctrl-H` | Request LSP hover panel |
| `Ctrl-/` | Request LSP completions |
| `Ctrl-]` | Request LSP go to definition |
| `Ctrl-M` | Mark current file |
| `Ctrl-1` .. `Ctrl-9` | Jump to mark 1-9 |
| `Ctrl-W` | Toggle focus between explorer and editor |
| `Tab` | Insert indent in editor |
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

## LSP support

ideot starts a language server when it opens a supported file and the server binary is available on `PATH`. LSP can be toggled with `Ctrl-L`; mouse/keyboard hover popups can be toggled separately with `Ctrl-O`. Diagnostic display is quiet by default and can be toggled with `Ctrl-U`.

| File types | Language server command | macOS install | Linux install |
|---|---|---|---|
| Rust `.rs` | `rust-analyzer` | `brew install rust-analyzer` | `rustup component add rust-analyzer` |
| Go `.go` | `gopls` | `brew install gopls` | `go install golang.org/x/tools/gopls@latest` |
| Python `.py` | `pylsp` | `python3 -m pip install 'python-lsp-server[all]'` | `python3 -m pip install --user 'python-lsp-server[all]'` |
| JavaScript/TypeScript `.js`, `.jsx`, `.mjs`, `.ts`, `.tsx` | `typescript-language-server --stdio` | `npm install -g typescript typescript-language-server` | `npm install -g typescript typescript-language-server` |
| Vue `.vue` | `vue-language-server --stdio` | `npm install -g @vue/language-server` | `npm install -g @vue/language-server` |
| JSON `.json` | `vscode-json-language-server --stdio` | `npm install -g vscode-langservers-extracted` | `npm install -g vscode-langservers-extracted` |
| YAML `.yaml`, `.yml` | `yaml-language-server --stdio` | `npm install -g yaml-language-server` | `npm install -g yaml-language-server` |
| Bash/Shell `.sh`, `.bash`, `.zsh` | `bash-language-server start` | `npm install -g bash-language-server` | `npm install -g bash-language-server` |
| HTML `.html`, `.htm` | `vscode-html-language-server --stdio` | `npm install -g vscode-langservers-extracted` | `npm install -g vscode-langservers-extracted` |
| CSS/SCSS/LESS `.css`, `.scss`, `.less` | `vscode-css-language-server --stdio` | `npm install -g vscode-langservers-extracted` | `npm install -g vscode-langservers-extracted` |
| C/C++ `.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.cxx`, `.hh`, `.hxx` | `clangd` | `brew install llvm` | `sudo apt install clangd` / distro equivalent |
| Lua `.lua` | `lua-language-server` | `brew install lua-language-server` | install from distro package or lua-language-server release |
| Markdown `.md`, `.mdx` | `marksman` | `brew install marksman` | install from distro package or Marksman release |

Notes:

- Restart ideot after installing a new language server so it can be discovered on `PATH`.
- Linux package managers may also provide these servers, but names vary by distro.
- Python LSP is provided by the `python-lsp-server` package and exposes the `pylsp` command.
- Vue support expects the `vue-language-server` command from `@vue/language-server`.
- JSON/HTML/CSS servers are provided by `vscode-langservers-extracted`.
- C/C++ support uses `clangd`; ensure compile commands are available for best diagnostics.

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
│   ├── lsp/              # Language server discovery, protocol, client, diagnostics
│   ├── runtime.rs        # Render scheduling / idle CPU control
│   ├── settings.rs       # Remembered user settings
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
