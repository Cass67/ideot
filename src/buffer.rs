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
    fn normalize_line_endings(text: String) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
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

    pub fn from_text(text: String) -> Self {
        let text = Self::normalize_line_endings(text);
        let lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(ToOwned::to_owned).collect()
        };
        Self {
            path: None,
            lines,
            dirty: false,
        }
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
        std::fs::write(path, self.text())
            .with_context(|| format!("failed to write {}", path.display()))?;
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

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn replace_text(&mut self, text: String) {
        let text = Self::normalize_line_endings(text);
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(ToOwned::to_owned).collect()
        };
        self.dirty = true;
    }

    pub fn insert_text(&mut self, position: Position, text: String) -> Position {
        let text = Self::normalize_line_endings(text);
        if text.is_empty() {
            return position;
        }

        let line_idx = position.line.min(self.lines.len().saturating_sub(1));
        let current = self.lines.get(line_idx).cloned().unwrap_or_default();
        let column = Self::ceil_char_boundary(&current, position.column);
        let left = &current[..column];
        let right = &current[column..];
        let parts: Vec<&str> = text.split('\n').collect();

        if parts.len() == 1 {
            self.lines[line_idx] = format!("{}{}{}", left, parts[0], right);
            self.dirty = true;
            return Position {
                line: line_idx,
                column: column + parts[0].len(),
            };
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

        Position {
            line: line_idx + parts.len() - 1,
            column: last.len(),
        }
    }

    pub fn insert_char(&mut self, position: Position, ch: char) {
        if let Some(line) = self.lines.get_mut(position.line) {
            let column = Self::ceil_char_boundary(line, position.column);
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
        let column = Self::ceil_char_boundary(line, position.column);
        let right = line.split_off(column);
        self.lines.insert(position.line + 1, right);
        self.dirty = true;
    }

    pub fn delete_char_after(&mut self, position: Position) {
        if position.line >= self.lines.len() {
            return;
        }
        let line_len = self.lines[position.line].len();
        if position.column < line_len {
            let line = &mut self.lines[position.line];
            let column = Self::ceil_char_boundary(line, position.column);
            let next = Self::next_char_boundary(line, column);
            line.replace_range(column..next, "");
            self.dirty = true;
        }
    }

    pub fn delete_char_before(&mut self, position: Position) {
        if position.line >= self.lines.len() {
            return;
        }
        if position.column > 0 {
            let line = &mut self.lines[position.line];
            let column = Self::ceil_char_boundary(line, position.column);
            if column > 0 {
                let previous = Self::previous_char_boundary(line, column);
                line.replace_range(previous..column, "");
                self.dirty = true;
            }
        } else if position.line > 0 {
            let current = self.lines.remove(position.line);
            self.lines[position.line - 1].push_str(&current);
            self.dirty = true;
        }
    }

    pub fn replace_line(&mut self, index: usize, content: &str) {
        if index < self.lines.len() {
            self.lines[index] = content.to_string();
            self.dirty = true;
        }
    }

    pub fn remove_line(&mut self, index: usize) {
        if index < self.lines.len() {
            self.lines.remove(index);
            self.dirty = true;
        }
    }
}
