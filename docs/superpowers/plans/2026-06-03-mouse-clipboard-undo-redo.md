# Mouse Clipboard Undo Redo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix ideot's editor foundation with fluid mouse drag selection, reliable copy/paste, whole-buffer select-all, and undo/redo.

**Architecture:** Extend the existing `Editor` as the owner of selection and edit history, keep clipboard access in `App`, and add a small `input` module that converts terminal mouse events into high-level editor actions. Use snapshot-based history capped at 100 entries for correctness and simple redo semantics.

**Tech Stack:** Rust 2021, ratatui, crossterm, arboard, existing integration tests under `tests/`.

---

## File Structure

- Modify: `src/buffer.rs`
  - Add whole-buffer replacement and robust multi-line insertion primitives.
- Modify: `src/editor.rs`
  - Add `EditKind`, `HistoryEntry`, undo/redo stacks, select-all, direct selection setting, multi-line insertion, selection replacement, undo, and redo.
- Modify: `src/app.rs`
  - Add app-level wrappers for select-all, undo, redo, drag selection actions, paste-as-one-operation, and status messages.
- Create: `src/input.rs`
  - Add `MouseInputController` and coordinate mapping for editor mouse drag selection.
- Modify: `src/lib.rs`
  - Export the new `input` module.
- Modify: `src/main.rs`
  - Own a `MouseInputController`, add keybindings, enable bracketed paste if available, and route mouse events through high-level input actions.
- Modify: `src/ui.rs`
  - Update help/footer text for select-all, undo, redo, and mouse drag selection.
- Modify: `README.md`
  - Document final keybindings and terminal-native modifier selection escape hatch.
- Modify: `tests/selection.rs`
  - Add editor tests for select-all and selection replacement.
- Modify: `tests/buffer_editing.rs`
  - Add buffer tests for replacement and multi-line insertion.
- Create: `tests/undo_redo.rs`
  - Add editor history tests.
- Create: `tests/mouse_input.rs`
  - Add mouse coordinate mapping and drag state tests.

Important: the working tree may already contain user changes. During implementation, inspect `git status --short` before edits and only stage files changed for each task.

---

## Task 1: Buffer text primitives

**Files:**
- Modify: `src/buffer.rs`
- Test: `tests/buffer_editing.rs`

- [ ] **Step 1: Append failing buffer tests**

Add these tests to the end of `tests/buffer_editing.rs`:

```rust
#[test]
fn replace_text_replaces_entire_buffer_and_marks_dirty() {
    let mut buffer = Buffer::from_text("old\ntext".to_string());

    buffer.replace_text("new\nfile".to_string());

    assert_eq!(buffer.line_count(), 2);
    assert_eq!(buffer.line(0), Some("new"));
    assert_eq!(buffer.line(1), Some("file"));
    assert_eq!(buffer.text(), "new\nfile");
    assert!(buffer.is_dirty());
}

#[test]
fn insert_text_at_position_handles_multiline_text() {
    let mut buffer = Buffer::from_text("hello world".to_string());

    let end = buffer.insert_text(Position { line: 0, column: 5 }, "\nsmall\n".to_string());

    assert_eq!(buffer.text(), "hello\nsmall\n world");
    assert_eq!(end, Position { line: 2, column: 0 });
    assert!(buffer.is_dirty());
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --test buffer_editing replace_text_replaces_entire_buffer_and_marks_dirty insert_text_at_position_handles_multiline_text
```

Expected: compile failure because `Buffer::replace_text` and `Buffer::insert_text` do not exist.

- [ ] **Step 3: Implement buffer primitives**

In `src/buffer.rs`, add these methods inside `impl Buffer` after `text()`:

```rust
    pub fn replace_text(&mut self, text: String) {
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(ToOwned::to_owned).collect()
        };
        self.dirty = true;
    }

    pub fn insert_text(&mut self, position: Position, text: String) -> Position {
        if text.is_empty() {
            return position;
        }

        let line_idx = position.line.min(self.lines.len().saturating_sub(1));
        let current = self.lines.get(line_idx).cloned().unwrap_or_default();
        let column = position.column.min(current.len());
        let left = &current[..column];
        let right = &current[column..];
        let parts: Vec<&str> = text.split('\n').collect();

        if parts.len() == 1 {
            self.lines[line_idx] = format!("{}{}{}", left, parts[0], right);
            self.dirty = true;
            return Position { line: line_idx, column: column + parts[0].len() };
        }

        self.lines[line_idx] = format!("{}{}", left, parts[0]);
        let mut insert_at = line_idx + 1;
        for middle in &parts[1..parts.len() - 1] {
            self.lines.insert(insert_at, (*middle).to_string());
            insert_at += 1;
        }
        let last = parts.last().copied().unwrap_or("");
        self.lines.insert(insert_at, format!("{}{}", last, right));
        self.dirty = true;

        Position { line: line_idx + parts.len() - 1, column: last.len() }
    }
```

- [ ] **Step 4: Run buffer tests**

Run:

```bash
cargo test --test buffer_editing
```

Expected: all `buffer_editing` tests pass.

- [ ] **Step 5: Commit Task 1**

```bash
git add src/buffer.rs tests/buffer_editing.rs
git commit -m "feat: add buffer text primitives"
```

---

## Task 2: Editor select-all and selection replacement

**Files:**
- Modify: `src/editor.rs`
- Test: `tests/selection.rs`

- [ ] **Step 1: Append failing selection tests**

Add these tests to the end of `tests/selection.rs`:

```rust
#[test]
fn select_all_selects_entire_buffer() {
    let buffer = Buffer::from_text("one\ntwo\nthree".into());
    let mut editor = Editor::new(buffer);

    editor.select_all();

    assert_eq!(editor.selected_text(), "one\ntwo\nthree");
    let selection = editor.selection().expect("selection should exist");
    assert_eq!(selection.start, Position { line: 0, column: 0 });
    assert_eq!(selection.end, Position { line: 2, column: 5 });
}

#[test]
fn set_selection_uses_clamped_positions() {
    let buffer = Buffer::from_text("abc\ndef".into());
    let mut editor = Editor::new(buffer);

    editor.set_selection(Position { line: 0, column: 1 }, Position { line: 99, column: 99 });

    assert_eq!(editor.selected_text(), "bc\ndef");
    assert_eq!(editor.cursor(), Position { line: 1, column: 3 });
}

#[test]
fn replace_selection_replaces_multiline_range_and_clears_selection() {
    let buffer = Buffer::from_text("alpha\nbeta\ngamma".into());
    let mut editor = Editor::new(buffer);
    editor.set_selection(Position { line: 0, column: 2 }, Position { line: 1, column: 2 });

    editor.replace_selection("XYZ".to_string());

    assert_eq!(editor.buffer().text(), "alXYZta\ngamma");
    assert_eq!(editor.cursor(), Position { line: 0, column: 5 });
    assert_eq!(editor.selection(), None);
}

#[test]
fn insert_text_in_editor_moves_cursor_to_end_of_inserted_text() {
    let buffer = Buffer::from_text("hello world".into());
    let mut editor = Editor::new(buffer);
    editor.set_cursor(Position { line: 0, column: 5 });

    editor.insert_text("\nsmall".to_string());

    assert_eq!(editor.buffer().text(), "hello\nsmall world");
    assert_eq!(editor.cursor(), Position { line: 1, column: 5 });
}
```

- [ ] **Step 2: Run selection tests to verify failure**

Run:

```bash
cargo test --test selection select_all_selects_entire_buffer set_selection_uses_clamped_positions replace_selection_replaces_multiline_range_and_clears_selection insert_text_in_editor_moves_cursor_to_end_of_inserted_text
```

Expected: compile failure because editor methods do not exist.

- [ ] **Step 3: Implement selection and insertion editor APIs**

In `src/editor.rs`, add these methods inside `impl Editor` near the existing selection methods:

```rust
    pub fn set_selection(&mut self, start: Position, end: Position) {
        let start = self.clamped_position(start);
        let end = self.clamped_position(end);
        self.cursor = end;
        self.selection = if start == end {
            None
        } else {
            Some(Selection { start, end })
        };
    }

    pub fn select_all(&mut self) {
        let last_line = self.buffer.line_count().saturating_sub(1);
        let last_column = self.buffer.line(last_line).map(str::len).unwrap_or(0);
        self.cursor = Position { line: last_line, column: last_column };
        self.selection = Some(Selection {
            start: Position { line: 0, column: 0 },
            end: self.cursor,
        });
    }

    pub fn insert_text(&mut self, text: String) {
        let end = self.buffer.insert_text(self.cursor, text);
        self.cursor = end;
        self.selection = None;
    }

    pub fn replace_selection(&mut self, text: String) {
        if self.selection.is_some() {
            self.delete_selection();
        }
        self.insert_text(text);
    }
```

Also add this helper near `clamp_column`:

```rust
    fn clamped_position(&self, position: Position) -> Position {
        let line = position.line.min(self.buffer.line_count().saturating_sub(1));
        let column = position
            .column
            .min(self.buffer.line(line).map(str::len).unwrap_or(0));
        Position { line, column }
    }
```

Then change `set_cursor` to use the helper:

```rust
    pub fn set_cursor(&mut self, position: Position) {
        self.cursor = self.clamped_position(position);
    }
```

- [ ] **Step 4: Run selection tests**

Run:

```bash
cargo test --test selection
```

Expected: all selection tests pass.

- [ ] **Step 5: Run buffer tests**

Run:

```bash
cargo test --test buffer_editing
```

Expected: all buffer editing tests pass.

- [ ] **Step 6: Commit Task 2**

```bash
git add src/editor.rs tests/selection.rs
git commit -m "feat: add editor select all and replace selection"
```

---

## Task 3: Snapshot undo/redo in Editor

**Files:**
- Modify: `src/editor.rs`
- Create: `tests/undo_redo.rs`

- [ ] **Step 1: Create failing undo/redo tests**

Create `tests/undo_redo.rs` with:

```rust
use ideot::buffer::{Buffer, Position};
use ideot::editor::Editor;

#[test]
fn undo_and_redo_insert_text() {
    let mut editor = Editor::new(Buffer::from_text("abc".into()));
    editor.set_cursor(Position { line: 0, column: 1 });

    editor.edit_insert_text("XYZ".to_string(), "insert");

    assert_eq!(editor.buffer().text(), "aXYZbc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 4 });
    assert_eq!(editor.undo(), Some("insert".to_string()));
    assert_eq!(editor.buffer().text(), "abc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 1 });
    assert_eq!(editor.redo(), Some("insert".to_string()));
    assert_eq!(editor.buffer().text(), "aXYZbc");
    assert_eq!(editor.cursor(), Position { line: 0, column: 4 });
}

#[test]
fn undo_and_redo_replace_selection() {
    let mut editor = Editor::new(Buffer::from_text("one\ntwo\nthree".into()));
    editor.set_selection(Position { line: 0, column: 1 }, Position { line: 1, column: 2 });

    editor.edit_replace_selection("XX".to_string(), "replace selection");

    assert_eq!(editor.buffer().text(), "oXXo\nthree");
    assert_eq!(editor.undo(), Some("replace selection".to_string()));
    assert_eq!(editor.buffer().text(), "one\ntwo\nthree");
    assert_eq!(editor.selection().expect("selection restored").start, Position { line: 0, column: 1 });
    assert_eq!(editor.redo(), Some("replace selection".to_string()));
    assert_eq!(editor.buffer().text(), "oXXo\nthree");
}

#[test]
fn new_edit_after_undo_clears_redo() {
    let mut editor = Editor::new(Buffer::from_text("abc".into()));
    editor.set_cursor(Position { line: 0, column: 3 });
    editor.edit_insert_text("d".to_string(), "insert");
    editor.undo();

    editor.edit_insert_text("X".to_string(), "insert");

    assert_eq!(editor.buffer().text(), "abcX");
    assert_eq!(editor.redo(), None);
}

#[test]
fn undo_stack_is_capped_at_100_entries() {
    let mut editor = Editor::new(Buffer::from_text("".into()));

    for _ in 0..101 {
        editor.edit_insert_text("x".to_string(), "insert");
    }

    let mut undo_count = 0;
    while editor.undo().is_some() {
        undo_count += 1;
    }

    assert_eq!(undo_count, 100);
}
```

- [ ] **Step 2: Run undo/redo tests to verify failure**

Run:

```bash
cargo test --test undo_redo
```

Expected: compile failure because history APIs do not exist.

- [ ] **Step 3: Add history types and fields**

In `src/editor.rs`, add these types below `Selection`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSnapshot {
    text: String,
    cursor: Position,
    selection: Option<Selection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    label: String,
    before: EditorSnapshot,
    after: EditorSnapshot,
}
```

Change `Editor` to include history stacks:

```rust
pub struct Editor {
    buffer: Buffer,
    cursor: Position,
    selection: Option<Selection>,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}
```

Update `Editor::new`:

```rust
        Self {
            buffer,
            cursor: Position { line: 0, column: 0 },
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
```

- [ ] **Step 4: Add snapshot helpers**

In `src/editor.rs`, add these private methods inside `impl Editor`:

```rust
    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            text: self.buffer.text(),
            cursor: self.cursor,
            selection: self.selection,
        }
    }

    fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
        self.buffer.replace_text(snapshot.text.clone());
        self.cursor = snapshot.cursor;
        self.selection = snapshot.selection;
    }

    fn push_history(&mut self, label: &str, before: EditorSnapshot, after: EditorSnapshot) {
        if before == after {
            return;
        }
        self.undo_stack.push(HistoryEntry {
            label: label.to_string(),
            before,
            after,
        });
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
```

- [ ] **Step 5: Add history-aware edit APIs and undo/redo**

In `src/editor.rs`, add these public methods inside `impl Editor`:

```rust
    pub fn edit_insert_text(&mut self, text: String, label: &str) {
        let before = self.snapshot();
        if self.selection.is_some() {
            self.replace_selection(text);
        } else {
            self.insert_text(text);
        }
        let after = self.snapshot();
        self.push_history(label, before, after);
    }

    pub fn edit_replace_selection(&mut self, text: String, label: &str) {
        let before = self.snapshot();
        self.replace_selection(text);
        let after = self.snapshot();
        self.push_history(label, before, after);
    }

    pub fn edit_backspace(&mut self) {
        let before = self.snapshot();
        if self.selection.is_some() {
            self.delete_selection();
            let after = self.snapshot();
            self.push_history("delete", before, after);
            return;
        }
        self.backspace();
        let after = self.snapshot();
        self.push_history("delete", before, after);
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.undo_stack.pop()?;
        self.restore_snapshot(&entry.before);
        let label = entry.label.clone();
        self.redo_stack.push(entry);
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let entry = self.redo_stack.pop()?;
        self.restore_snapshot(&entry.after);
        let label = entry.label.clone();
        self.undo_stack.push(entry);
        Some(label)
    }
```

- [ ] **Step 6: Run undo/redo tests**

Run:

```bash
cargo test --test undo_redo
```

Expected: all undo/redo tests pass.

- [ ] **Step 7: Run editor-related tests**

Run:

```bash
cargo test --test buffer_editing --test selection --test undo_redo
```

Expected: all selected tests pass.

- [ ] **Step 8: Commit Task 3**

```bash
git add src/editor.rs tests/undo_redo.rs
git commit -m "feat: add editor undo redo history"
```

---

## Task 4: App-level select-all, undo, redo, and paste transactions

**Files:**
- Modify: `src/app.rs`
- Test: existing tests plus `tests/undo_redo.rs`

- [ ] **Step 1: Add app-facing tests to `tests/undo_redo.rs`**

Append:

```rust
use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_undo_and_redo_restore_file_buffer_after_insert() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "abc").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.insert_char('X');
    assert_eq!(app.editor().unwrap().buffer().text(), "Xabc");

    app.undo();
    assert_eq!(app.editor().unwrap().buffer().text(), "abc");

    app.redo();
    assert_eq!(app.editor().unwrap().buffer().text(), "Xabc");
}

#[test]
fn app_select_all_selects_entire_open_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "one\ntwo").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.select_all();

    assert_eq!(app.editor().unwrap().selected_text(), "one\ntwo");
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --test undo_redo app_undo_and_redo_restore_file_buffer_after_insert app_select_all_selects_entire_open_file
```

Expected: compile failure because `App::undo`, `App::redo`, and `App::select_all` do not exist or because `App::insert_char` is not history-aware yet.

- [ ] **Step 3: Add app helper to notify LSP/recent after edits**

In `src/app.rs`, add this private helper near `lsp_change`:

```rust
    fn after_current_editor_changed(&mut self) {
        let Some(editor) = &self.editor else {
            return;
        };
        if let Some(path) = editor.buffer().path() {
            let path = path.to_path_buf();
            let text = editor.buffer().text().to_string();
            self.lsp_change(&path, text);
        }
        if let Some(relative) = self.current_relative.as_ref() {
            self.recent.record(relative.clone());
        }
    }
```

- [ ] **Step 4: Make app editing operations history-aware**

Replace `App::insert_char`, `App::insert_newline`, and `App::backspace` with:

```rust
    pub fn insert_char(&mut self, ch: char) {
        if let Some(editor) = &mut self.editor {
            editor.edit_insert_text(ch.to_string(), "insert");
            self.after_current_editor_changed();
        }
    }

    pub fn insert_newline(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.edit_insert_text("\n".to_string(), "insert");
            self.after_current_editor_changed();
        }
    }

    pub fn backspace(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.edit_backspace();
            self.after_current_editor_changed();
        }
    }
```

- [ ] **Step 5: Replace paste with a single transaction**

Replace `App::paste` with:

```rust
    pub fn paste(&mut self) -> Result<()> {
        let Some(editor) = &mut self.editor else {
            bail!("no open file");
        };
        let mut clipboard = arboard::Clipboard::new()
            .context("failed to open clipboard")?;
        let text = clipboard.get_text()?;
        let line_count = text.matches('\n').count() + 1;
        let char_count = text.chars().count();

        if editor.selection().is_some() {
            editor.edit_replace_selection(text, "paste");
        } else {
            editor.edit_insert_text(text, "paste");
        }
        self.after_current_editor_changed();
        self.status = if line_count > 1 {
            format!("pasted {line_count} lines")
        } else {
            format!("pasted {char_count} chars")
        };
        Ok(())
    }
```

- [ ] **Step 6: Add app select-all, undo, and redo methods**

Add these methods near copy/paste in `src/app.rs`:

```rust
    pub fn select_all(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.select_all();
            self.status = "selected all".to_string();
        }
    }

    pub fn undo(&mut self) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        match editor.undo() {
            Some(label) => {
                self.status = format!("undid {label}");
                self.after_current_editor_changed();
            }
            None => self.status = "nothing to undo".to_string(),
        }
    }

    pub fn redo(&mut self) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        match editor.redo() {
            Some(label) => {
                self.status = format!("redid {label}");
                self.after_current_editor_changed();
            }
            None => self.status = "nothing to redo".to_string(),
        }
    }
```

- [ ] **Step 7: Improve copy status**

Replace the success part of `App::copy_selection` with:

```rust
        let line_count = text.matches('\n').count() + 1;
        let char_count = text.chars().count();
        let mut clipboard = arboard::Clipboard::new()
            .context("failed to open clipboard")?;
        clipboard.set_text(text)?;
        self.status = if line_count > 1 {
            format!("copied {line_count} lines")
        } else {
            format!("copied {char_count} chars")
        };
        Ok(())
```

Keep the existing `bail!("nothing selected")` behavior for callers that ignore the result.

- [ ] **Step 8: Run app-level tests**

Run:

```bash
cargo test --test undo_redo
```

Expected: all undo/redo tests pass.

- [ ] **Step 9: Run full test suite**

Run:

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 10: Commit Task 4**

```bash
git add src/app.rs tests/undo_redo.rs
git commit -m "feat: wire editor history into app editing"
```

---

## Task 5: Mouse input controller and coordinate mapping

**Files:**
- Create: `src/input.rs`
- Modify: `src/lib.rs`
- Create: `tests/mouse_input.rs`

- [ ] **Step 1: Create failing mouse input tests**

Create `tests/mouse_input.rs` with:

```rust
use ideot::buffer::Position;
use ideot::input::{EditorViewport, MouseAction, MouseInputController};

#[test]
fn maps_editor_cell_to_buffer_position_with_scroll() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 10,
    };

    let position = viewport.position_for_cell(35, 4, &["short".to_string(), "abcdef".to_string()]);

    assert_eq!(position, Some(Position { line: 13, column: 5 }));
}

#[test]
fn mapping_clamps_column_to_line_length() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 0,
    };

    let position = viewport.position_for_cell(99, 1, &["abc".to_string()]);

    assert_eq!(position, Some(Position { line: 0, column: 3 }));
}

#[test]
fn drag_sequence_emits_start_update_finish_actions() {
    let viewport = EditorViewport {
        x: 30,
        y: 1,
        width: 70,
        height: 20,
        scroll: 0,
    };
    let lines = vec!["abcdef".to_string(), "second".to_string()];
    let mut controller = MouseInputController::default();

    assert_eq!(
        controller.left_down(32, 1, &viewport, &lines),
        Some(MouseAction::MoveCursor(Position { line: 0, column: 2 }))
    );
    assert_eq!(
        controller.drag(35, 1, &viewport, &lines),
        Some(MouseAction::UpdateSelection {
            anchor: Position { line: 0, column: 2 },
            end: Position { line: 0, column: 5 },
        })
    );
    assert_eq!(controller.left_up(), Some(MouseAction::FinishSelection));
}
```

- [ ] **Step 2: Run mouse input tests to verify failure**

Run:

```bash
cargo test --test mouse_input
```

Expected: compile failure because `ideot::input` does not exist.

- [ ] **Step 3: Create `src/input.rs`**

Create `src/input.rs` with:

```rust
use crate::buffer::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorViewport {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub scroll: usize,
}

impl EditorViewport {
    pub fn contains(&self, column: u16, row: u16) -> bool {
        column >= self.x
            && column < self.x.saturating_add(self.width)
            && row >= self.y
            && row < self.y.saturating_add(self.height)
    }

    pub fn position_for_cell(
        &self,
        column: u16,
        row: u16,
        visible_lines: &[String],
    ) -> Option<Position> {
        if !self.contains(column, row) {
            return None;
        }
        let visible_row = row.saturating_sub(self.y) as usize;
        let line = self.scroll + visible_row;
        let line_text = visible_lines.get(visible_row)?;
        let raw_column = column.saturating_sub(self.x) as usize;
        Some(Position {
            line,
            column: raw_column.min(line_text.len()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    MoveCursor(Position),
    UpdateSelection { anchor: Position, end: Position },
    FinishSelection,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MouseInputController {
    anchor: Option<Position>,
    dragging: bool,
}

impl MouseInputController {
    pub fn left_down(
        &mut self,
        column: u16,
        row: u16,
        viewport: &EditorViewport,
        visible_lines: &[String],
    ) -> Option<MouseAction> {
        let position = viewport.position_for_cell(column, row, visible_lines)?;
        self.anchor = Some(position);
        self.dragging = false;
        Some(MouseAction::MoveCursor(position))
    }

    pub fn drag(
        &mut self,
        column: u16,
        row: u16,
        viewport: &EditorViewport,
        visible_lines: &[String],
    ) -> Option<MouseAction> {
        let anchor = self.anchor?;
        let end = viewport.position_for_cell(column, row, visible_lines)?;
        self.dragging = true;
        Some(MouseAction::UpdateSelection { anchor, end })
    }

    pub fn left_up(&mut self) -> Option<MouseAction> {
        let was_dragging = self.dragging;
        self.anchor = None;
        self.dragging = false;
        if was_dragging {
            Some(MouseAction::FinishSelection)
        } else {
            None
        }
    }
}
```

- [ ] **Step 4: Export input module**

Add this line to `src/lib.rs`:

```rust
pub mod input;
```

- [ ] **Step 5: Run mouse input tests**

Run:

```bash
cargo test --test mouse_input
```

Expected: all mouse input tests pass.

- [ ] **Step 6: Commit Task 5**

```bash
git add src/input.rs src/lib.rs tests/mouse_input.rs
git commit -m "feat: add mouse input controller"
```

---

## Task 6: Wire mouse drag selection into App and main loop

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Test: `tests/mouse_input.rs` plus full test suite

- [ ] **Step 1: Add app selection action tests**

Append to `tests/mouse_input.rs`:

```rust
use ideot::app::App;
use tempfile::tempdir;

#[test]
fn app_mouse_selection_actions_select_text() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "abcdef\nsecond").unwrap();
    let mut app = App::new(dir.path().to_path_buf());
    app.rebuild_index().unwrap();
    app.open_relative("note.txt").unwrap();

    app.move_editor_cursor_to(Position { line: 0, column: 1 });
    app.update_editor_selection(Position { line: 0, column: 1 }, Position { line: 1, column: 3 });
    app.finish_editor_selection();

    assert_eq!(app.editor().unwrap().selected_text(), "bcdef\nsec");
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test mouse_input app_mouse_selection_actions_select_text
```

Expected: compile failure because app mouse selection methods do not exist.

- [ ] **Step 3: Add app-level mouse selection methods**

In `src/app.rs`, add:

```rust
    pub fn move_editor_cursor_to(&mut self, position: Position) {
        self.focus_pane = FocusPane::Editor;
        if let Some(editor) = &mut self.editor {
            editor.set_cursor(position);
            editor.clear_selection();
        }
    }

    pub fn update_editor_selection(&mut self, anchor: Position, end: Position) {
        self.focus_pane = FocusPane::Editor;
        if let Some(editor) = &mut self.editor {
            editor.set_selection(anchor, end);
        }
    }

    pub fn finish_editor_selection(&mut self) {
        self.status = "selection ready".to_string();
    }
```

Scrolling is handled by the existing `App::scroll_to_cursor(visible_height)` method from `main.rs` after applying mouse actions.

- [ ] **Step 4: Run mouse app test**

Run:

```bash
cargo test --test mouse_input app_mouse_selection_actions_select_text
```

Expected: test passes.

- [ ] **Step 5: Route controller in `main.rs`**

In `src/main.rs`, add imports:

```rust
use ideot::input::{EditorViewport, MouseAction, MouseInputController};
```

At the start of `run`, before the loop, add:

```rust
    let mut mouse_input = MouseInputController::default();
```

Inside `Event::Mouse(mouse)`, before `match mouse.kind`, compute visible editor lines:

```rust
                let editor_x = explorer_width.saturating_add(1);
                let editor_height = content_height.saturating_sub(1);
                let editor_viewport = EditorViewport {
                    x: editor_x,
                    y: 1,
                    width: size.width.saturating_sub(editor_x),
                    height: editor_height,
                    scroll: app.editor_scroll(),
                };
                let visible_lines: Vec<String> = app
                    .editor()
                    .map(|editor| {
                        editor
                            .buffer()
                            .lines()
                            .iter()
                            .skip(app.editor_scroll())
                            .take(editor_height as usize)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
```

Then add mouse match arms before the existing editor click arm:

```rust
                    MouseEventKind::Down(MouseButton::Left)
                        if mouse.column > explorer_width && in_content_rows =>
                    {
                        if let Some(action) = mouse_input.left_down(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left)
                        if mouse.column > explorer_width && in_content_rows =>
                    {
                        if let Some(action) = mouse_input.drag(
                            mouse.column,
                            mouse.row,
                            &editor_viewport,
                            &visible_lines,
                        ) {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(action) = mouse_input.left_up() {
                            apply_mouse_action(app, action, editor_height as usize);
                        }
                    }
```

Remove or skip the older editor `Down(MouseButton::Left)` arm that directly calls `app.place_editor_cursor(row, column)`, so clicks do not execute twice.

Add this helper below `run`:

```rust
fn apply_mouse_action(app: &mut App, action: MouseAction, visible_height: usize) {
    match action {
        MouseAction::MoveCursor(position) => app.move_editor_cursor_to(position),
        MouseAction::UpdateSelection { anchor, end } => app.update_editor_selection(anchor, end),
        MouseAction::FinishSelection => app.finish_editor_selection(),
    }
    app.scroll_to_cursor(visible_height);
}
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test --test mouse_input
cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Commit Task 6**

```bash
git add src/app.rs src/main.rs tests/mouse_input.rs
git commit -m "feat: wire mouse drag selection"
```

---

## Task 7: Keybindings for select-all, undo, redo, and bracketed paste

**Files:**
- Modify: `src/main.rs`
- Modify: `src/app.rs`
- Test: full test suite and manual compile

- [ ] **Step 1: Add `App::insert_pasted_text`**

In `src/app.rs`, add this method near `paste`:

```rust
    pub fn insert_pasted_text(&mut self, text: String) {
        let Some(editor) = &mut self.editor else {
            self.status = "no open file".to_string();
            return;
        };
        let line_count = text.matches('\n').count() + 1;
        let char_count = text.chars().count();
        if editor.selection().is_some() {
            editor.edit_replace_selection(text, "paste");
        } else {
            editor.edit_insert_text(text, "paste");
        }
        self.after_current_editor_changed();
        self.status = if line_count > 1 {
            format!("pasted {line_count} lines")
        } else {
            format!("pasted {char_count} chars")
        };
    }
```

Then simplify `App::paste` so after reading clipboard text it delegates to `insert_pasted_text`:

```rust
    pub fn paste(&mut self) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()
            .context("failed to open clipboard")?;
        let text = clipboard.get_text()?;
        self.insert_pasted_text(text);
        Ok(())
    }
```

- [ ] **Step 2: Add keybinding arms in `main.rs`**

In the `Event::Key(key)` match, add these arms before the plain character arm:

```rust
                (KeyModifiers::CONTROL, KeyCode::Char('a')) => app.select_all(),
                (_, KeyCode::Char('U')) => app.undo(),
                (KeyModifiers::CONTROL, KeyCode::Char('r')) => app.redo(),
```

Keep existing `Y` for copy and `Ctrl+V` for paste.

- [ ] **Step 3: Enable bracketed paste**

`crossterm` 0.28.1 exposes `EnableBracketedPaste`, `DisableBracketedPaste`, and `Event::Paste`. Update the `src/main.rs` event imports from:

```rust
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
```

to:

```rust
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
        EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
    },
```

Add `EnableBracketedPaste` to the startup `execute!` call:

```rust
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        SetCursorStyle::SteadyBar
    )?;
```

Add `DisableBracketedPaste` to the cleanup `execute!` call:

```rust
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape
    )?;
```

Add this event arm before `Event::Mouse(mouse)`:

```rust
            Event::Paste(text) => {
                app.insert_pasted_text(text);
            }
```

- [ ] **Step 4: Run formatting and tests**

Run:

```bash
cargo fmt
cargo test
```

Expected: formatting succeeds and all tests pass.

- [ ] **Step 5: Commit Task 7**

```bash
git add src/main.rs src/app.rs
git commit -m "feat: add selection undo redo keybindings"
```

---

## Task 8: Help text and README documentation

**Files:**
- Modify: `src/ui.rs`
- Modify: `README.md`

- [ ] **Step 1: Update footer/help text in `src/ui.rs`**

Find the non-git `shortcuts` string and replace it with:

```rust
"Tab Focus · Arrows Nav · Drag Select · Ctrl-A All · Y Copy · Ctrl-V Paste · U Undo · Ctrl-R Redo · Ctrl-P Search · Ctrl-G Git · F1 Help · Ctrl-Q Quit"
```

In the help overlay, update or add lines so the keyboard and mouse sections include:

```rust
        Line::from("  Ctrl-A       Select all text in current file"),
        Line::from("  Y            Copy selection"),
        Line::from("  Ctrl+V       Paste"),
        Line::from("  U            Undo"),
        Line::from("  Ctrl-R       Redo"),
```

And the mouse section includes:

```rust
        Line::from("  Drag editor text  Select text in ideot"),
        Line::from("  Modifier-drag     Terminal-native selection escape hatch"),
```

- [ ] **Step 2: Update README keybindings**

In `README.md`, update the keybindings table to include:

```markdown
| `Mouse drag` | Select editor text inside ideot |
| terminal modifier drag | Use terminal-native selection escape hatch |
| `Ctrl-A` | Select all text in the current editor buffer |
| `U` | Undo last edit |
| `Ctrl-R` | Redo last undone edit |
| `Y` / `Ctrl-Shift-C` | Copy selection to system clipboard |
| `Ctrl-V` | Paste from system clipboard |
```

- [ ] **Step 3: Run docs-adjacent checks**

Run:

```bash
cargo fmt
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit Task 8**

```bash
git add src/ui.rs README.md
git commit -m "docs: document mouse clipboard undo redo controls"
```

---

## Task 9: Final verification and manual checklist

**Files:**
- No required source changes unless verification exposes a defect.

- [ ] **Step 1: Run full automated verification**

Run:

```bash
cargo fmt --check
cargo test
```

Expected: formatting check passes and all tests pass.

- [ ] **Step 2: Run the app manually**

Run:

```bash
cargo run -- .
```

Manual checks in Ghostty and iTerm2:

- Open a file.
- Drag inside editor text; selection follows mouse.
- Click without dragging; cursor moves and selection clears.
- Press `Y`; selected text is copied to system clipboard.
- Press `Ctrl-A`, then `Y`; whole file is copied, including off-screen text.
- Press `Ctrl-V`; clipboard text inserts at cursor.
- Select text, press `Ctrl-V`; selected text is replaced.
- Press `U`; last text edit is undone.
- Press `Ctrl-R`; last undone edit is redone.
- Paste a multi-line clipboard; it appears in one operation and one `U` removes it.
- Use terminal modifier-drag; terminal-native selection is still available.
- Large paste remains responsive.

- [ ] **Step 3: Update status if manual verification reveals terminal-specific caveat**

If Ghostty or iTerm2 requires a specific modifier for terminal-native selection, update `README.md` and `src/ui.rs` with the exact modifier text. Then run:

```bash
cargo fmt --check
cargo test
```

Expected: checks pass.

- [ ] **Step 4: Final commit if docs changed during manual verification**

If Step 3 changed files:

```bash
git add src/ui.rs README.md
git commit -m "docs: clarify terminal selection escape hatch"
```

- [ ] **Step 5: Final status summary**

Run:

```bash
git status --short
```

Expected: only pre-existing user changes remain, or working tree is clean if the implementation branch owned all changes.
