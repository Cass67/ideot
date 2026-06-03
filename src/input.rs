use crate::buffer::Position;
use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpModalAction {
    Close,
    Ignore,
}

pub fn help_modal_key_action(modifiers: KeyModifiers, code: KeyCode) -> HelpModalAction {
    match (modifiers, code) {
        (_, KeyCode::F(1)) | (_, KeyCode::Esc) => HelpModalAction::Close,
        _ => HelpModalAction::Ignore,
    }
}

pub fn help_modal_mouse_action(_kind: MouseEventKind) -> HelpModalAction {
    HelpModalAction::Ignore
}

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
        let display_column = column.saturating_sub(self.x) as usize;
        let byte_column = line_text
            .char_indices()
            .nth(display_column)
            .map(|(idx, _)| idx)
            .unwrap_or(line_text.len());
        Some(Position {
            line,
            column: byte_column,
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
