# ideot

A fast, minimal Rust terminal code editor.

## MVP features

- Classic two-pane layout: file explorer left, editor right.
- Normal terminal editing controls.
- `Ctrl-P` fuzzy file search with recent-file boost.
- Temporary session marks with `Ctrl-M` and `Ctrl-1` through `Ctrl-9`.
- Syntax highlighting abstraction with plain fallback.
- LSP-ready document event boundary for a future LSP milestone.

## Run

```bash
cargo run
```

## Keybindings

- `Up` / `Down`: move file selection.
- `Enter`: open selected file.
- Type normally: insert text.
- `Ctrl-S`: save current file.
- `Ctrl-P`: open file search.
- `Ctrl-M`: mark current file.
- `Ctrl-1`..`Ctrl-9`: jump to mark.
- `Ctrl-Q`: quit.

## Build single binary

```bash
cargo build --release
```

Binary path: `target/release/ideot`.
