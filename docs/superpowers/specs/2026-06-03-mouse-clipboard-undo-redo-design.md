# Phase 1 Design: Mouse Selection, Clipboard, Select All, Undo/Redo

## Goal

Make ideot feel like a fast, minimal terminal editor for everyday editing. Phase 1 fixes the foundation: mouse drag selection, reliable copy/paste, whole-buffer select-all, and undo/redo. This preserves ideot's current lightweight two-pane shape while making editor interactions fluid enough to replace a GUI editor for normal coding work.

## Scope

In scope:

- Editor-native mouse drag selection.
- Hybrid escape hatch for terminal-native selection.
- Reliable copy and paste through the system clipboard.
- Whole-buffer select-all for the current editor buffer, including off-screen text.
- Undo and redo for text-changing operations.
- Performance improvements needed to keep drag selection and paste fluid.
- Tests for selection, clipboard-facing editor behavior, select-all, undo, and redo.

Out of scope for this phase:

- Double-click word selection.
- Rectangular/block selection.
- Persistent undo history across process restarts.
- Diff-based history optimization.
- Full LSP UX. LSP is Phase 2.

## User-Facing Behavior

### Mouse Selection

- Normal drag inside the editor pane starts editor-native selection.
- The drag start position becomes the selection anchor.
- The selection end follows the current mouse position while dragging.
- Releasing the mouse finalizes the selection.
- Clicking inside the editor without dragging moves the cursor and clears any existing selection.
- Dragging outside the visible editor area clamps to the nearest valid visible buffer position.
- Scrolled content is accounted for when converting terminal coordinates to buffer positions.
- Modifier-drag remains the terminal-native selection escape hatch where supported by Ghostty/iTerm2. ideot should document this instead of trying to own every terminal selection case.

### Copy

- `Y` and `Ctrl+Shift+C` copy the current ideot selection to the system clipboard.
- If no ideot selection exists, copy shows a clear status such as `nothing selected`.
- Copying a multi-line selection preserves line breaks.
- Copying after select-all copies the whole current file, including off-screen text.
- Copy status should be informative, e.g. `copied 3 lines` or `copied 124 chars`.
- Clipboard errors should be surfaced as status messages, e.g. `clipboard unavailable`.

### Paste

- `Ctrl+V` pastes system clipboard text at the cursor.
- If an editor selection exists, paste replaces the selection.
- Multi-line paste is inserted as one logical editor operation, not a character-by-character UI churn.
- Paste updates status, e.g. `pasted 24 chars` or `pasted 3 lines`.
- Bracketed paste support should be enabled if crossterm supports it cleanly, so terminal paste input arrives as paste data rather than keypress noise.

### Select All

- Add a select-all command for the current editor buffer.
- Select-all selects the whole file, including text not visible on screen.
- Whole-buffer select-all is preferred over visible-screen select-all because it is predictable and editor-like.
- The selected range should be represented using the same selection model as drag and keyboard selection.
- Copy after select-all copies the entire file.
- Paste after select-all replaces the entire file and creates one undoable transaction.

### Undo/Redo

- Add editor history stacks:
  - `undo_stack`
  - `redo_stack`
- `U` undoes the last text-changing operation.
- Redo should be available via `Ctrl-R`.
- New edits after undo clear the redo stack.
- Undo/redo restores:
  - buffer content
  - cursor position
  - selection state
  - scroll position
- Non-text actions do not create history entries:
  - cursor moves
  - selection-only changes
  - copy
  - search
  - pane focus changes
  - save
- Save does not clear undo/redo history.
- Status messages should be clear:
  - `undid paste`
  - `redid delete`
  - `nothing to undo`
  - `nothing to redo`

## Architecture

### Mouse/Input Controller

Use a small mouse/input controller rather than adding more raw event logic directly to `main.rs` or `app.rs`.

Suggested responsibilities:

- Track mouse state:
  - `Idle`
  - `Selecting { anchor }`
  - possible future states such as `ResizingPane` or `ContextMenu`
- Convert terminal coordinates to editor buffer positions.
- Distinguish click from drag.
- Dispatch high-level app actions such as:
  - `start_editor_selection(position)`
  - `update_editor_selection(position)`
  - `finish_editor_selection()`
  - `move_cursor_to(position)`

This keeps `App` focused on editor state and actions, not raw terminal event interpretation.

### Editor Selection Model

The existing selection model should be reused and extended:

- Selection has `start` and `end` positions.
- Bounds are direction-independent for copy/delete/paste.
- Drag selection updates `end` while preserving `start`.
- Select-all sets selection from buffer start to buffer end.
- Empty selections should be normalized or treated as no selection where appropriate.

### Clipboard Boundary

Clipboard interaction remains at the app boundary using `arboard`, but text extraction and replacement should be handled by editor/buffer primitives.

Recommended shape:

- `Editor::selected_text() -> String`
- `Editor::select_all()`
- `Editor::replace_selection(text)`
- `Editor::insert_text(text)`
- `App::copy_selection()` handles `arboard::Clipboard::set_text`.
- `App::paste()` handles `arboard::Clipboard::get_text` and delegates one logical editor operation.

### Undo/Redo History

Use snapshot-based history for Phase 1:

- It is simpler and safer than diff-based history.
- It makes paste, delete, selection replacement, and whole-file replacement straightforward.
- Cap history by entry count at 100 edits for Phase 1.

Each history entry should include:

- operation label, e.g. `insert`, `delete`, `paste`, `replace selection`
- buffer snapshot before the edit
- cursor before the edit
- selection before the edit
- optionally scroll before the edit
- buffer snapshot after the edit if redo cannot be reconstructed from current state
- cursor/selection after the edit

Implementation can later optimize to text diffs without changing user-facing behavior.

## Performance Requirements

- Mouse drag selection must update live without obvious lag.
- During drag, only the latest mouse position needs to win if events arrive faster than rendering.
- Avoid unnecessary full-file work during drag; redraw visible rows only.
- Paste should insert the full string as one editor operation and redraw once.
- Syntax highlighting should not be recalculated for the full file on every mouse event.
- History should be capped to avoid large-paste memory blowups.
- Status updates must not block input.

## Testing Plan

Automated tests:

- Select-all selects the whole buffer, including off-screen lines.
- Copy text extraction works for single-line and multi-line selections.
- Paste inserts at cursor.
- Paste replaces a selection.
- Paste after select-all replaces the entire buffer.
- Undo restores previous content after insert.
- Undo restores previous content after delete/backspace.
- Undo restores previous content after paste.
- Undo restores previous content after selection replacement.
- Redo reapplies an undone insert/delete/paste/replacement.
- New edit after undo clears redo history.
- Mouse drag coordinate mapping accounts for editor pane origin and scroll offset.
- Click without drag clears selection and moves cursor.

Manual verification:

- Ghostty: drag selection, copy, paste, select-all, undo, redo.
- iTerm2: drag selection, copy, paste, select-all, undo, redo.
- Modifier-drag terminal-native selection escape hatch.
- Large paste remains responsive.
- Drag selection across visible rows feels fluid.

## Keybindings

Preferred bindings:

- Drag mouse in editor: editor-native selection.
- Terminal modifier-drag: terminal-native selection escape hatch, terminal-dependent.
- `Y`: copy selection.
- `Ctrl+Shift+C`: copy selection.
- `Ctrl+V`: paste.
- `Ctrl+A`: select all current editor buffer, unless it conflicts with terminal behavior.
- `U`: undo.
- `Ctrl-R`: redo.

## Rollout Order

1. Add editor primitives for select-all, replace-selection, multi-line insert, and history transactions.
2. Add undo/redo stacks and tests.
3. Make paste a single undoable operation.
4. Add select-all and copy whole-buffer behavior.
5. Add mouse/input controller and drag selection state.
6. Wire mouse drag to editor selection.
7. Tune rendering/event coalescing for fluid drag.
8. Update README/help text with bindings and terminal-native selection escape hatch.
9. Manually verify in Ghostty and iTerm2.

## Future Phase

Phase 2 should focus on LSP UX:

- Diagnostics display.
- Hover popup.
- Completion menu.
- Go-to-definition and navigation stack.
- References/rename/formatting if supported by the active language server.
