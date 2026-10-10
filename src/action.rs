#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    App(AppAction),
    LogView(LogViewAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    Suspend,
    OpenCommandLine,
    ShowError(String),
    #[cfg(debug_assertions)]
    DebugPanic,
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
