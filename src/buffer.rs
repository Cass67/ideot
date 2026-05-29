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
