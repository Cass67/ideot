#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Quit,
    Save,
    OpenSearch,
    MarkCurrentFile,
    JumpToMark(usize),
    FocusNextPane,
    None,
}
