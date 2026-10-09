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
    MoveUp(usize),
    MoveDown(usize),
    GoToLineNumber(usize),
    GoToLastLine,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    ScrollLeft(usize),
    ScrollRight(usize),
    ScrollLeftWide,
    ScrollRightWide,
    ScrollToFirstColumn,
}
