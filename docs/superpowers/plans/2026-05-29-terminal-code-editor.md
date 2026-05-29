# Terminal Code Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the MVP Rust terminal code editor described in `docs/superpowers/specs/2026-05-29-terminal-code-editor-design.md`.

**Architecture:** Use a small custom editor core with clear module boundaries. `app` owns the event loop and state, `ui` renders a fixed explorer/editor layout, and focused modules handle buffers, file indexing, search, marks, highlighting, and future LSP events.

**Tech Stack:** Rust 2021, Cargo, ratatui, crossterm, ignore, nucleo-matcher, tree-sitter, tree-sitter-rust, tree-sitter-go, tree-sitter-javascript, tree-sitter-python, tree-sitter-md, serde, tempfile, insta.

---

## File Structure

Create this structure:

```text
Cargo.toml
src/main.rs
src/lib.rs
src/app.rs
src/app/command.rs
src/buffer.rs
src/editor.rs
src/fs.rs
src/search.rs
src/marks.rs
src/highlight.rs
src/lsp.rs
src/ui.rs
tests/buffer_editing.rs
tests/file_index.rs
tests/search_recent.rs
tests/marks.rs
tests/open_edit_save.rs
```

Responsibilities:

- `main.rs`: binary entry point and terminal setup/teardown.
- `lib.rs`: testable module exports.
- `app.rs`: top-level state, command dispatch, and app lifecycle.
- `app/command.rs`: normalized commands from key events.
- `buffer.rs`: editable document storage and disk loading/saving.
- `editor.rs`: cursor movement and buffer mutation helpers.
- `fs.rs`: `.gitignore`-aware project file index.
- `search.rs`: fuzzy file search with recent-file boost.
- `marks.rs`: temporary session file marks.
- `highlight.rs`: syntax highlighting abstraction and plain/Tree-sitter implementations.
- `lsp.rs`: future LSP document event types and sink trait.
- `ui.rs`: ratatui rendering for explorer, editor, overlays, and status bar.
- `tests/*`: integration tests for public behavior.

---

### Task 1: Cargo project and module skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/app.rs`
- Create: `src/app/command.rs`
- Create: `src/buffer.rs`
- Create: `src/editor.rs`
- Create: `src/fs.rs`
- Create: `src/search.rs`
- Create: `src/marks.rs`
- Create: `src/highlight.rs`
- Create: `src/lsp.rs`
- Create: `src/ui.rs`

- [ ] **Step 1: Write project manifest**

Create `Cargo.toml`:

```toml
[package]
name = "ideot"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
anyhow = "1"
crossterm = "0.28"
ratatui = "0.29"
ignore = "0.4"
nucleo-matcher = "0.3"
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-go = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-md = "0.3"
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tempfile = "3"
insta = "1"
```

- [ ] **Step 2: Create library exports**

Create `src/lib.rs`:

```rust
pub mod app;
pub mod buffer;
pub mod editor;
pub mod fs;
pub mod highlight;
pub mod lsp;
pub mod marks;
pub mod search;
pub mod ui;
```

- [ ] **Step 3: Create minimal app module files**

Create `src/app.rs`:

```rust
pub mod command;

use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self { root, should_quit: false }
    }
}
```

Create `src/app/command.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Quit,
    Save,
    OpenSearch,
    MarkCurrentFile,
    JumpToMark(usize),
    FocusNextPane,
    None,
}
```

- [ ] **Step 4: Create placeholder modules that compile**

Create each listed file with this exact content, replacing the module name in the comment:

```rust
//! Module implemented in later tasks.
```

Files: `src/buffer.rs`, `src/editor.rs`, `src/fs.rs`, `src/search.rs`, `src/marks.rs`, `src/highlight.rs`, `src/lsp.rs`, `src/ui.rs`.

- [ ] **Step 5: Create binary entry point**

Create `src/main.rs`:

```rust
use anyhow::Result;
use ideot::app::App;

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let _app = App::new(root);
    println!("ideot MVP skeleton");
    Ok(())
}
```

- [ ] **Step 6: Verify skeleton builds**

Run:

```bash
cargo test
```

Expected: dependencies compile and test result reports `ok` with zero or more tests.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src
git commit -m "chore: scaffold rust editor project"
```

---

### Task 2: Editable buffer with load, edit, dirty state, and save

**Files:**
- Modify: `src/buffer.rs`
- Create: `tests/buffer_editing.rs`

- [ ] **Step 1: Write failing buffer tests**

Create `tests/buffer_editing.rs`:

```rust
use ideot::buffer::{Buffer, Position};
use tempfile::tempdir;

#[test]
fn insert_and_delete_update_text_and_dirty_state() {
    let mut buffer = Buffer::from_text("hello\nworld".to_string());

    assert_eq!(buffer.line_count(), 2);
    assert!(!buffer.is_dirty());

    buffer.insert_char(Position { line: 0, column: 5 }, '!');
    assert_eq!(buffer.line(0), Some("hello!"));
    assert!(buffer.is_dirty());

    buffer.delete_char_before(Position { line: 0, column: 6 });
    assert_eq!(buffer.line(0), Some("hello"));
}

#[test]
fn newline_splits_line_and_backspace_joins_lines() {
    let mut buffer = Buffer::from_text("abcd".to_string());

    buffer.insert_newline(Position { line: 0, column: 2 });
    assert_eq!(buffer.line(0), Some("ab"));
    assert_eq!(buffer.line(1), Some("cd"));

    buffer.delete_char_before(Position { line: 1, column: 0 });
    assert_eq!(buffer.line_count(), 1);
    assert_eq!(buffer.line(0), Some("abcd"));
}

#[test]
fn load_and_save_round_trip_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "one\ntwo").unwrap();

    let mut buffer = Buffer::load(&path).unwrap();
    buffer.insert_char(Position { line: 1, column: 3 }, '!');
    buffer.save().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\ntwo!");
    assert!(!buffer.is_dirty());
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --test buffer_editing
```

Expected: FAIL because `ideot::buffer::Buffer` and `Position` do not exist.

- [ ] **Step 3: Implement buffer**

Replace `src/buffer.rs` with:

```rust
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    path: Option<PathBuf>,
    lines: Vec<String>,
    dirty: bool,
}

impl Buffer {
    pub fn from_text(text: String) -> Self {
        let lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(ToOwned::to_owned).collect()
        };
        Self { path: None, lines, dirty: false }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut buffer = Self::from_text(text);
        buffer.path = Some(path);
        Ok(buffer)
    }

    pub fn save(&mut self) -> Result<()> {
        let path = self.path.as_ref().context("buffer has no path")?;
        std::fs::write(path, self.text()).with_context(|| format!("failed to write {}", path.display()))?;
        self.dirty = false;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn insert_char(&mut self, position: Position, ch: char) {
        if let Some(line) = self.lines.get_mut(position.line) {
            let column = position.column.min(line.len());
            line.insert(column, ch);
            self.dirty = true;
        }
    }

    pub fn insert_newline(&mut self, position: Position) {
        if position.line >= self.lines.len() {
            self.lines.push(String::new());
            self.dirty = true;
            return;
        }
        let line = &mut self.lines[position.line];
        let column = position.column.min(line.len());
        let right = line.split_off(column);
        self.lines.insert(position.line + 1, right);
        self.dirty = true;
    }

    pub fn delete_char_before(&mut self, position: Position) {
        if position.line >= self.lines.len() {
            return;
        }
        if position.column > 0 {
            let line = &mut self.lines[position.line];
            let column = position.column.min(line.len());
            if column > 0 {
                line.remove(column - 1);
                self.dirty = true;
            }
        } else if position.line > 0 {
            let current = self.lines.remove(position.line);
            self.lines[position.line - 1].push_str(&current);
            self.dirty = true;
        }
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test --test buffer_editing
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs tests/buffer_editing.rs
git commit -m "feat: add editable file buffer"
```

---

### Task 3: Editor cursor movement and text mutation wrapper

**Files:**
- Modify: `src/editor.rs`
- Modify: `tests/buffer_editing.rs`

- [ ] **Step 1: Add failing editor test**

Append to `tests/buffer_editing.rs`:

```rust
use ideot::editor::Editor;

#[test]
fn editor_tracks_cursor_while_editing() {
    let mut editor = Editor::new(Buffer::from_text("abc".to_string()));

    editor.move_right();
    editor.move_right();
    editor.insert_char('X');
    assert_eq!(editor.buffer().line(0), Some("abXc"));
    assert_eq!(editor.cursor(), Position { line: 0, column: 3 });

    editor.insert_newline();
    assert_eq!(editor.buffer().line(0), Some("abX"));
    assert_eq!(editor.buffer().line(1), Some("c"));
    assert_eq!(editor.cursor(), Position { line: 1, column: 0 });

    editor.backspace();
    assert_eq!(editor.buffer().line(0), Some("abXc"));
    assert_eq!(editor.cursor(), Position { line: 0, column: 3 });
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test buffer_editing editor_tracks_cursor_while_editing
```

Expected: FAIL because `Editor` does not exist.

- [ ] **Step 3: Implement editor wrapper**

Replace `src/editor.rs` with:

```rust
use crate::buffer::{Buffer, Position};

#[derive(Debug, Clone)]
pub struct Editor {
    buffer: Buffer,
    cursor: Position,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self { buffer, cursor: Position { line: 0, column: 0 } }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn move_left(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = self.buffer.line(self.cursor.line).map(str::len).unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.buffer.line(self.cursor.line).map(str::len).unwrap_or(0);
        if self.cursor.column < line_len {
            self.cursor.column += 1;
        } else if self.cursor.line + 1 < self.buffer.line_count() {
            self.cursor.line += 1;
            self.cursor.column = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.clamp_column();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.line + 1 < self.buffer.line_count() {
            self.cursor.line += 1;
            self.clamp_column();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert_char(self.cursor, ch);
        self.cursor.column += ch.len_utf8();
    }

    pub fn insert_newline(&mut self) {
        self.buffer.insert_newline(self.cursor);
        self.cursor.line += 1;
        self.cursor.column = 0;
    }

    pub fn backspace(&mut self) {
        let before = self.cursor;
        self.buffer.delete_char_before(before);
        if before.column > 0 {
            self.cursor.column -= 1;
        } else if before.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = self.buffer.line(self.cursor.line).map(str::len).unwrap_or(0);
        }
    }

    fn clamp_column(&mut self) {
        let line_len = self.buffer.line(self.cursor.line).map(str::len).unwrap_or(0);
        self.cursor.column = self.cursor.column.min(line_len);
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test --test buffer_editing
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/editor.rs tests/buffer_editing.rs
git commit -m "feat: add editor cursor operations"
```

---

### Task 4: Ignore-aware project file index

**Files:**
- Modify: `src/fs.rs`
- Create: `tests/file_index.rs`

- [ ] **Step 1: Write failing file index tests**

Create `tests/file_index.rs`:

```rust
use ideot::fs::ProjectIndex;
use tempfile::tempdir;

#[test]
fn indexes_files_relative_to_root_and_respects_gitignore() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("README.md"), "readme").unwrap();
    std::fs::write(dir.path().join("target/cache.txt"), "ignored").unwrap();
    std::fs::write(dir.path().join(".gitignore"), "target\n").unwrap();

    let index = ProjectIndex::build(dir.path()).unwrap();
    let paths: Vec<_> = index.files().iter().map(|file| file.relative.as_str()).collect();

    assert!(paths.contains(&"src/main.rs"));
    assert!(paths.contains(&"README.md"));
    assert!(!paths.contains(&"target/cache.txt"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test file_index
```

Expected: FAIL because `ProjectIndex` does not exist.

- [ ] **Step 3: Implement project index**

Replace `src/fs.rs` with:

```rust
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    pub absolute: PathBuf,
    pub relative: String,
}

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    root: PathBuf,
    files: Vec<ProjectFile>,
}

impl ProjectIndex {
    pub fn build(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = Vec::new();
        for entry in WalkBuilder::new(&root).hidden(false).build() {
            let entry = entry.context("failed to walk project")?;
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            let absolute = entry.path().to_path_buf();
            let relative = absolute
                .strip_prefix(&root)
                .unwrap_or(&absolute)
                .to_string_lossy()
                .replace('\\', "/");
            files.push(ProjectFile { absolute, relative });
        }
        files.sort_by(|a, b| a.relative.cmp(&b.relative));
        Ok(Self { root, files })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn files(&self) -> &[ProjectFile] {
        &self.files
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test --test file_index
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/fs.rs tests/file_index.rs
git commit -m "feat: add ignore aware file index"
```

---

### Task 5: Fuzzy file search with recent-file boost

**Files:**
- Modify: `src/search.rs`
- Create: `tests/search_recent.rs`

- [ ] **Step 1: Write failing search tests**

Create `tests/search_recent.rs`:

```rust
use ideot::fs::ProjectFile;
use ideot::search::{RecentFiles, SearchIndex};
use std::path::PathBuf;

fn file(path: &str) -> ProjectFile {
    ProjectFile { absolute: PathBuf::from(path), relative: path.to_string() }
}

#[test]
fn fuzzy_search_returns_matching_files() {
    let files = vec![file("src/main.rs"), file("docs/design.md"), file("Cargo.toml")];
    let search = SearchIndex::new(files);

    let results = search.query("main", &RecentFiles::default());

    assert_eq!(results[0].relative, "src/main.rs");
}

#[test]
fn recent_files_are_boosted_when_scores_are_close() {
    let files = vec![file("src/app.rs"), file("examples/app.rs")];
    let search = SearchIndex::new(files);
    let mut recent = RecentFiles::default();
    recent.record("examples/app.rs");

    let results = search.query("app", &recent);

    assert_eq!(results[0].relative, "examples/app.rs");
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test search_recent
```

Expected: FAIL because search types do not exist.

- [ ] **Step 3: Implement search and recent files**

Replace `src/search.rs` with:

```rust
use crate::fs::ProjectFile;
use nucleo_matcher::{pattern::Pattern, Config, Matcher};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct RecentFiles {
    items: VecDeque<String>,
}

impl RecentFiles {
    pub fn record(&mut self, relative: impl Into<String>) {
        let relative = relative.into();
        self.items.retain(|item| item != &relative);
        self.items.push_front(relative);
        self.items.truncate(32);
    }

    pub fn rank(&self, relative: &str) -> Option<usize> {
        self.items.iter().position(|item| item == relative)
    }
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    files: Vec<ProjectFile>,
}

impl SearchIndex {
    pub fn new(files: Vec<ProjectFile>) -> Self {
        Self { files }
    }

    pub fn query(&self, query: &str, recent: &RecentFiles) -> Vec<ProjectFile> {
        if query.trim().is_empty() {
            return self.files.clone();
        }
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, nucleo_matcher::pattern::CaseMatching::Smart, nucleo_matcher::pattern::Normalization::Smart);
        let mut scored: Vec<(i64, ProjectFile)> = self
            .files
            .iter()
            .filter_map(|file| {
                pattern.score(file.relative.as_str(), &mut matcher).map(|score| {
                    let boost = recent.rank(&file.relative).map(|rank| 10_000 - rank as i64).unwrap_or(0);
                    (score as i64 + boost, file.clone())
                })
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.relative.cmp(&b.1.relative)));
        scored.into_iter().map(|(_, file)| file).collect()
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test --test search_recent
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/search.rs tests/search_recent.rs
git commit -m "feat: add fuzzy file search"
```

---

### Task 6: Temporary session marks

**Files:**
- Modify: `src/marks.rs`
- Create: `tests/marks.rs`

- [ ] **Step 1: Write failing mark tests**

Create `tests/marks.rs`:

```rust
use ideot::marks::SessionMarks;

#[test]
fn stores_current_file_in_first_available_mark_slot() {
    let mut marks = SessionMarks::default();

    let slot = marks.mark("src/main.rs");

    assert_eq!(slot, 1);
    assert_eq!(marks.get(1).map(String::as_str), Some("src/main.rs"));
}

#[test]
fn remarking_existing_file_keeps_single_entry() {
    let mut marks = SessionMarks::default();

    marks.mark("src/main.rs");
    let slot = marks.mark("src/main.rs");

    assert_eq!(slot, 1);
    assert_eq!(marks.iter().count(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test marks
```

Expected: FAIL because `SessionMarks` does not exist.

- [ ] **Step 3: Implement session marks**

Replace `src/marks.rs` with:

```rust
#[derive(Debug, Clone, Default)]
pub struct SessionMarks {
    slots: [Option<String>; 9],
}

impl SessionMarks {
    pub fn mark(&mut self, relative: impl Into<String>) -> usize {
        let relative = relative.into();
        if let Some(index) = self.slots.iter().position(|item| item.as_deref() == Some(relative.as_str())) {
            return index + 1;
        }
        let index = self.slots.iter().position(Option::is_none).unwrap_or(0);
        self.slots[index] = Some(relative);
        index + 1
    }

    pub fn get(&self, slot: usize) -> Option<&String> {
        if !(1..=9).contains(&slot) {
            return None;
        }
        self.slots[slot - 1].as_ref()
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &String)> {
        self.slots.iter().enumerate().filter_map(|(index, value)| value.as_ref().map(|path| (index + 1, path)))
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test --test marks
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/marks.rs tests/marks.rs
git commit -m "feat: add session marks"
```

---

### Task 7: LSP-ready document event boundary

**Files:**
- Modify: `src/lsp.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add LSP event types**

Replace `src/lsp.rs` with:

```rust
use crate::buffer::Position;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEvent {
    Opened { path: PathBuf, text: String },
    Changed { path: PathBuf, text: String },
    Saved { path: PathBuf, text: String },
    CursorMoved { path: PathBuf, position: Position },
}

pub trait DocumentEventSink {
    fn send(&mut self, event: DocumentEvent);
}

#[derive(Debug, Default)]
pub struct NullDocumentEventSink;

impl DocumentEventSink for NullDocumentEventSink {
    fn send(&mut self, _event: DocumentEvent) {}
}
```

- [ ] **Step 2: Wire app to hold future LSP sink seam**

Replace `src/app.rs` with:

```rust
pub mod command;

use crate::lsp::NullDocumentEventSink;
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
    pub lsp_sink: NullDocumentEventSink,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self { root, should_quit: false, lsp_sink: NullDocumentEventSink }
    }
}
```

- [ ] **Step 3: Verify code compiles**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lsp.rs src/app.rs
git commit -m "feat: add lsp ready document events"
```

---

### Task 8: Syntax highlighting abstraction with plain-text fallback

**Files:**
- Modify: `src/highlight.rs`

- [ ] **Step 1: Implement highlighter interface and fallback**

Replace `src/highlight.rs` with:

```rust
use ratatui::style::{Color, Style};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

pub trait Highlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan>;
}

#[derive(Debug, Default)]
pub struct PlainHighlighter;

impl Highlighter for PlainHighlighter {
    fn highlight_line(&mut self, _language_hint: Option<&str>, _line: &str) -> Vec<HighlightSpan> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct SimpleTreeSitterHighlighter {
    fallback: PlainHighlighter,
}

impl Highlighter for SimpleTreeSitterHighlighter {
    fn highlight_line(&mut self, language_hint: Option<&str>, line: &str) -> Vec<HighlightSpan> {
        match language_hint {
            Some("rs") | Some("rust") if line.trim_start().starts_with("fn ") => {
                let start = line.find("fn").unwrap_or(0);
                vec![HighlightSpan { start, end: start + 2, style: Style::default().fg(Color::Magenta) }]
            }
            _ => self.fallback.highlight_line(language_hint, line),
        }
    }
}
```

- [ ] **Step 2: Verify code compiles**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/highlight.rs
git commit -m "feat: add highlighter abstraction"
```

---

### Task 9: App state, commands, and open/edit/save integration

**Files:**
- Modify: `src/app.rs`
- Modify: `src/app/command.rs`
- Create: `tests/open_edit_save.rs`

- [ ] **Step 1: Write failing app integration test**

Create `tests/open_edit_save.rs`:

```rust
use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_opens_edits_saves_and_tracks_recent_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("main.rs");
    std::fs::write(&path, "fn main() {}").unwrap();

    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("main.rs").unwrap();
    app.insert_char('/');
    app.save_current().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "/fn main() {}");
    assert_eq!(app.current_relative(), Some("main.rs"));
    assert_eq!(app.search("main")[0].relative, "main.rs");
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test open_edit_save
```

Expected: FAIL because `App` lacks these methods.

- [ ] **Step 3: Implement app integration**

Replace `src/app.rs` with:

```rust
pub mod command;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::fs::{ProjectFile, ProjectIndex};
use crate::lsp::{DocumentEvent, DocumentEventSink, NullDocumentEventSink};
use crate::marks::SessionMarks;
use crate::search::{RecentFiles, SearchIndex};
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub should_quit: bool,
    pub lsp_sink: NullDocumentEventSink,
    index: Option<ProjectIndex>,
    search_index: SearchIndex,
    recent: RecentFiles,
    marks: SessionMarks,
    editor: Option<Editor>,
    current_relative: Option<String>,
    status: String,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            should_quit: false,
            lsp_sink: NullDocumentEventSink,
            index: None,
            search_index: SearchIndex::new(Vec::new()),
            recent: RecentFiles::default(),
            marks: SessionMarks::default(),
            editor: None,
            current_relative: None,
            status: String::new(),
        }
    }

    pub fn rebuild_index(&mut self) -> Result<()> {
        let index = ProjectIndex::build(&self.root)?;
        self.search_index = SearchIndex::new(index.files().to_vec());
        self.index = Some(index);
        Ok(())
    }

    pub fn open_relative(&mut self, relative: &str) -> Result<()> {
        let path = self.root.join(relative);
        let buffer = Buffer::load(&path)?;
        let text = buffer.text();
        self.editor = Some(Editor::new(buffer));
        self.current_relative = Some(relative.to_string());
        self.recent.record(relative);
        self.lsp_sink.send(DocumentEvent::Opened { path, text });
        Ok(())
    }

    pub fn insert_char(&mut self, ch: char) {
        if let Some(editor) = &mut self.editor {
            editor.insert_char(ch);
            if let (Some(relative), Some(path)) = (self.current_relative.as_ref(), editor.buffer().path()) {
                self.lsp_sink.send(DocumentEvent::Changed { path: path.to_path_buf(), text: editor.buffer().text() });
                self.recent.record(relative.clone());
            }
        }
    }

    pub fn save_current(&mut self) -> Result<()> {
        let editor = self.editor.as_mut().context("no open file")?;
        editor.buffer_mut().save()?;
        if let Some(path) = editor.buffer().path() {
            self.lsp_sink.send(DocumentEvent::Saved { path: path.to_path_buf(), text: editor.buffer().text() });
        }
        self.status = "saved".to_string();
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<ProjectFile> {
        self.search_index.query(query, &self.recent)
    }

    pub fn current_relative(&self) -> Option<&str> {
        self.current_relative.as_deref()
    }

    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    pub fn marks(&self) -> &SessionMarks {
        &self.marks
    }
}
```

Replace `src/app/command.rs` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Quit,
    Save,
    OpenSearch,
    OpenRelative(String),
    InsertChar(char),
    Backspace,
    MarkCurrentFile,
    JumpToMark(usize),
    FocusNextPane,
    None,
}
```

- [ ] **Step 4: Verify integration test passes**

Run:

```bash
cargo test --test open_edit_save
```

Expected: PASS.

- [ ] **Step 5: Run all tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/app/command.rs tests/open_edit_save.rs
git commit -m "feat: integrate app open edit save flow"
```

---

### Task 10: Terminal UI render and event loop

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Implement fixed two-pane renderer**

Replace `src/ui.rs` with:

```rust
use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(root[0]);

    let files: Vec<ListItem> = app
        .search("")
        .into_iter()
        .take(200)
        .map(|file| ListItem::new(file.relative))
        .collect();
    frame.render_widget(List::new(files).block(Block::default().title("files").borders(Borders::ALL)), panes[0]);

    let editor_text = app
        .editor()
        .map(|editor| editor.buffer().text())
        .unwrap_or_else(|| "Open a file with Ctrl-P or select from explorer".to_string());
    frame.render_widget(Paragraph::new(editor_text).block(Block::default().title("editor").borders(Borders::ALL)), panes[1]);

    let status = app.current_relative().unwrap_or("no file");
    frame.render_widget(Paragraph::new(status.to_string()), root[1]);
}
```

- [ ] **Step 2: Implement terminal setup and simple event loop**

Replace `src/main.rs` with:

```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ideot::{app::App, ui};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let mut app = App::new(root);
    app.rebuild_index()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if app.should_quit {
            break;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => app.should_quit = true,
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                    let _ = app.save_current();
                }
                (_, KeyCode::Char(ch)) => app.insert_char(ch),
                _ => {}
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Verify compile and tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Manual smoke test**

Run:

```bash
cargo run
```

Expected: terminal enters alternate screen, shows left file list and right editor pane. Press `Ctrl-Q` to quit. If a file was opened by code or future UI selection, typing modifies it and `Ctrl-S` saves it.

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs src/main.rs
git commit -m "feat: render terminal two pane ui"
```

---

### Task 11: MVP gap pass for explorer selection, search overlay, and marks commands

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `src/ui.rs`

- [ ] **Step 1: Add app fields for selected file, search mode, and mark actions**

Modify `App` in `src/app.rs` to add these fields:

```rust
selected_file: usize,
search_query: String,
search_open: bool,
```

Initialize them in `App::new`:

```rust
selected_file: 0,
search_query: String::new(),
search_open: false,
```

Add methods:

```rust
pub fn move_selection_down(&mut self) {
    let max = self.search("").len().saturating_sub(1);
    self.selected_file = (self.selected_file + 1).min(max);
}

pub fn move_selection_up(&mut self) {
    self.selected_file = self.selected_file.saturating_sub(1);
}

pub fn selected_file(&self) -> usize {
    self.selected_file
}

pub fn open_selected(&mut self) -> anyhow::Result<()> {
    let files = self.search("");
    if let Some(file) = files.get(self.selected_file) {
        let relative = file.relative.clone();
        self.open_relative(&relative)?;
    }
    Ok(())
}

pub fn toggle_search(&mut self) {
    self.search_open = !self.search_open;
    self.search_query.clear();
}

pub fn search_open(&self) -> bool {
    self.search_open
}

pub fn search_query(&self) -> &str {
    &self.search_query
}

pub fn push_search_char(&mut self, ch: char) {
    self.search_query.push(ch);
}

pub fn pop_search_char(&mut self) {
    self.search_query.pop();
}

pub fn mark_current_file(&mut self) -> Option<usize> {
    let relative = self.current_relative.clone()?;
    Some(self.marks.mark(relative))
}

pub fn jump_to_mark(&mut self, slot: usize) -> anyhow::Result<()> {
    if let Some(relative) = self.marks.get(slot).cloned() {
        self.open_relative(&relative)?;
    }
    Ok(())
}
```

- [ ] **Step 2: Route keybindings in `src/main.rs`**

Update the `match` in `run`:

```rust
match (key.modifiers, key.code) {
    (KeyModifiers::CONTROL, KeyCode::Char('q')) => app.should_quit = true,
    (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
        let _ = app.save_current();
    }
    (KeyModifiers::CONTROL, KeyCode::Char('p')) => app.toggle_search(),
    (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
        let _ = app.mark_current_file();
    }
    (KeyModifiers::CONTROL, KeyCode::Char(ch)) if ('1'..='9').contains(&ch) => {
        let slot = ch.to_digit(10).unwrap() as usize;
        let _ = app.jump_to_mark(slot);
    }
    (_, KeyCode::Down) => app.move_selection_down(),
    (_, KeyCode::Up) => app.move_selection_up(),
    (_, KeyCode::Enter) => {
        let _ = app.open_selected();
    }
    (_, KeyCode::Backspace) if app.search_open() => app.pop_search_char(),
    (_, KeyCode::Char(ch)) if app.search_open() => app.push_search_char(ch),
    (_, KeyCode::Char(ch)) => app.insert_char(ch),
    _ => {}
}
```

- [ ] **Step 3: Render selected file and search overlay in `src/ui.rs`**

Update the file list creation to include marker text:

```rust
let files: Vec<ListItem> = app
    .search(if app.search_open() { app.search_query() } else { "" })
    .into_iter()
    .enumerate()
    .take(200)
    .map(|(index, file)| {
        let prefix = if index == app.selected_file() { "> " } else { "  " };
        ListItem::new(format!("{prefix}{}", file.relative))
    })
    .collect();
```

Before rendering the status bar, add this overlay render when search is open:

```rust
if app.search_open() {
    let area = centered_rect(60, 20, frame.area());
    let text = format!("Find file: {}", app.search_query());
    frame.render_widget(Paragraph::new(text).block(Block::default().title("search").borders(Borders::ALL)), area);
}
```

Add helper function to `src/ui.rs`:

```rust
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

- [ ] **Step 4: Verify compile and smoke test**

Run:

```bash
cargo test
cargo run
```

Expected: tests pass. In the running app, Up/Down changes selected file, Enter opens it, Ctrl-P opens search overlay, typing filters, Ctrl-M marks current file, Ctrl-1 jumps to first mark, Ctrl-Q exits.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs src/ui.rs
git commit -m "feat: add explorer selection search and marks ui"
```

---

### Task 12: Final verification and release build

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Create README usage docs**

Create `README.md`:

```markdown
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
```

- [ ] **Step 2: Run full test suite**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 3: Run release build**

Run:

```bash
cargo build --release
```

Expected: PASS and `target/release/ideot` exists.

- [ ] **Step 4: Run manual terminal smoke test**

Run:

```bash
cargo run
```

Expected: app opens in alternate screen, renders two panes, opens a selected file with Enter, allows typing, saves with Ctrl-S, opens search with Ctrl-P, marks with Ctrl-M, jumps with Ctrl-1, exits with Ctrl-Q.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: add ideot usage guide"
```

---

## Self-Review

Spec coverage:

- Classic two-pane UI: Tasks 10 and 11.
- File explorer: Tasks 4, 10, and 11.
- Editable right buffer: Tasks 2, 3, 9, 10, and 11.
- `Ctrl-P` fuzzy search with recent boosting: Tasks 5 and 11.
- Temporary session marks: Tasks 6 and 11.
- Syntax highlighting abstraction with fallback: Task 8.
- Normal terminal controls: Tasks 10 and 11.
- Single binary: Tasks 1 and 12.
- LSP-ready architecture: Task 7 and Task 9 event emission.
- Error handling: Tasks 2, 4, 9, and 10 use `anyhow::Result`; UI status can be expanded after MVP.
- Testing: Tasks 2 through 6 and 9 add automated tests; Task 12 adds manual smoke and release verification.

No placeholders are intentionally left in the plan. The first implementation may need dependency-version API adjustments if upstream crates differ from the assumed signatures; resolve those within the task that first compiles the dependency.
