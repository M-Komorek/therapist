use crate::domain::search::{SearchDirection, SearchQuery};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    App(AppAction),
    LogView(LogViewAction),
    Search(SearchAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    Suspend,
    OpenCommandLine(CommandLineKind),
    ShowError(String),
    #[cfg(debug_assertions)]
    DebugPanic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandLineKind {
    Command,
    Search(SearchDirection),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogViewAction {
    MoveCursorUp(usize),
    MoveCursorDown(usize),
    MoveCursorToLineNumber(usize),
    MoveCursorToLastLine,
    ScrollHalfPageUp,
    ScrollHalfPageDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollLeft(usize),
    ScrollRight(usize),
    ScrollLeftWide,
    ScrollRightWide,
    ScrollToFirstColumn,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    Start(SearchQuery),
    RepeatLast(SearchDirection),
    JumpToNextMatch,
    JumpToPreviousMatch,
    Clear,
}
