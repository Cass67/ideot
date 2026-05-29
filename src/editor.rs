use crate::buffer::{Buffer, Position};

#[derive(Debug, Clone)]
pub struct Editor {
    buffer: Buffer,
    cursor: Position,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Position { line: 0, column: 0 },
        }
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
        let line = position
            .line
            .min(self.buffer.line_count().saturating_sub(1));
        let line_len = self.buffer.line(line).map(str::len).unwrap_or(0);
        self.cursor = Position {
            line,
            column: position.column.min(line_len),
        };
    }

    pub fn move_left(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
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
        let line_len = self
            .buffer
            .line(self.cursor.line)
            .map(str::len)
            .unwrap_or(0);
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
        let previous_line_len = if before.column == 0 && before.line > 0 {
            self.buffer.line(before.line - 1).map(str::len).unwrap_or(0)
        } else {
            0
        };
        self.buffer.delete_char_before(before);
        if before.column > 0 {
            self.cursor.column -= 1;
        } else if before.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = previous_line_len;
        }
    }

    fn clamp_column(&mut self) {
        let line_len = self
            .buffer
            .line(self.cursor.line)
            .map(str::len)
            .unwrap_or(0);
        self.cursor.column = self.cursor.column.min(line_len);
    }
}
