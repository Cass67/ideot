use crate::buffer::{Buffer, Position};

/// Text selection range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

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

impl Selection {
    pub fn new(position: Position) -> Self {
        Self {
            start: position,
            end: position,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns (min_pos, max_pos) regardless of selection direction
    pub fn bounds(&self) -> (Position, Position) {
        if self.start.line < self.end.line
            || (self.start.line == self.end.line && self.start.column <= self.end.column)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Extract the selected text from the buffer
    pub fn text(&self, buffer: &Buffer) -> String {
        let (start, end) = self.bounds();
        let mut result = String::new();
        for line_idx in start.line..=end.line {
            let line = buffer.line(line_idx).unwrap_or("");
            let start_col = if line_idx == start.line {
                start.column
            } else {
                0
            };
            let end_col = if line_idx == end.line {
                end.column
            } else {
                line.len()
            };
            if start_col < end_col {
                result.push_str(&line[start_col..end_col]);
            }
            if line_idx < end.line {
                result.push('\n');
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct Editor {
    buffer: Buffer,
    cursor: Position,
    selection: Option<Selection>,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Position { line: 0, column: 0 },
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    // --- Selection ---

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

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
        self.cursor = Position {
            line: last_line,
            column: last_column,
        };
        self.selection = Some(Selection {
            start: Position { line: 0, column: 0 },
            end: self.cursor,
        });
    }

    pub fn start_selection(&mut self) {
        self.selection = Some(Selection::new(self.cursor));
    }

    pub fn extend_selection_to(&mut self) {
        self.selection = Some(Selection {
            start: self.selection.map(|s| s.start).unwrap_or(self.cursor),
            end: self.cursor,
        });
    }

    pub fn selected_text(&self) -> String {
        match self.selection {
            Some(sel) => sel.text(&self.buffer),
            None => String::new(),
        }
    }

    /// Collapse selection to cursor position at the start of the selection
    pub fn collapse_selection(&mut self) {
        if let Some(sel) = self.selection {
            let (start, _) = sel.bounds();
            self.cursor = start;
            self.selection = None;
        }
    }

    /// Delete the selected text, placing cursor at the start
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

    pub fn delete_selection(&mut self) {
        let Some(sel) = self.selection else {
            return;
        };
        let (start, end) = sel.bounds();
        if start == end {
            return;
        }

        if start.line == end.line {
            // Single line selection
            let line = self
                .buffer
                .line(start.line)
                .map(|l| l.to_string())
                .unwrap_or_default();
            let new_line = format!("{}{}", &line[..start.column], &line[end.column..]);
            self.buffer.replace_line(start.line, &new_line);
        } else {
            // Multi-line selection
            let first_line = self.buffer.line(start.line).unwrap_or("");
            let last_line = self.buffer.line(end.line).unwrap_or("");
            let new_line = format!(
                "{}{}",
                &first_line[..start.column],
                &last_line[end.column..]
            );
            // Remove lines from start+1 to end (inclusive)
            for _ in (start.line + 1)..=end.line {
                self.buffer.remove_line(start.line + 1);
            }
            self.buffer.replace_line(start.line, &new_line);
        }

        self.cursor = start;
        self.selection = None;
    }

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

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn set_cursor(&mut self, position: Position) {
        self.cursor = self.clamped_position(position);
    }

    pub fn move_left(&mut self) {
        if self.cursor.column > 0 {
            if let Some(line) = self.buffer.line(self.cursor.line) {
                self.cursor.column = Self::previous_char_boundary(line, self.cursor.column);
            }
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = self
                .buffer
                .line(self.cursor.line)
                .map(str::len)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        let line = self.buffer.line(self.cursor.line).unwrap_or("");
        let line_len = line.len();
        if self.cursor.column < line_len {
            self.cursor.column = Self::next_char_boundary(line, self.cursor.column);
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
        let previous_line_len = if before.column == 0 && before.line > 0 {
            self.buffer.line(before.line - 1).map(str::len).unwrap_or(0)
        } else {
            0
        };
        self.buffer.delete_char_before(before);
        if before.column > 0 {
            if let Some(line) = self.buffer.line(before.line) {
                self.cursor.column = Self::previous_char_boundary(line, before.column);
            }
        } else if before.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = previous_line_len;
        }
    }

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

    fn clamp_column(&mut self) {
        let line = self.buffer.line(self.cursor.line).unwrap_or("");
        self.cursor.column = Self::ceil_char_boundary(line, self.cursor.column);
    }

    fn clamped_position(&self, position: Position) -> Position {
        let line = position
            .line
            .min(self.buffer.line_count().saturating_sub(1));
        let line_text = self.buffer.line(line).unwrap_or("");
        let column = Self::ceil_char_boundary(line_text, position.column);
        Position { line, column }
    }

    fn ceil_char_boundary(text: &str, index: usize) -> usize {
        let mut index = index.min(text.len());
        while index < text.len() && !text.is_char_boundary(index) {
            index += 1;
        }
        index
    }

    fn previous_char_boundary(text: &str, index: usize) -> usize {
        let index = Self::ceil_char_boundary(text, index);
        text[..index]
            .char_indices()
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn next_char_boundary(text: &str, index: usize) -> usize {
        let index = Self::ceil_char_boundary(text, index);
        text[index..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| index + offset)
            .unwrap_or(text.len())
    }
}
