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
