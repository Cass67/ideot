#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickAction {
    Render,
    WaitForInput,
}

#[derive(Debug, Clone)]
pub struct RenderScheduler {
    dirty: bool,
}

impl Default for RenderScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderScheduler {
    pub fn new() -> Self {
        Self { dirty: true }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn tick(&mut self) -> TickAction {
        if self.dirty {
            self.dirty = false;
            TickAction::Render
        } else {
            TickAction::WaitForInput
        }
    }

    pub fn should_poll_lsp_after_idle(&self) -> bool {
        true
    }
}
