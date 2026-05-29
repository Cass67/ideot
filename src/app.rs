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
