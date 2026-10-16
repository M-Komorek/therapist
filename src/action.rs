use crate::domain::search::{SearchDirection, SearchQuery};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    App(AppAction),
    LogView(LogViewAction),
    Search(SearchAction),
    Bookmark(BookmarkAction),
    Tab(TabAction),
    Highlight(HighlightAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    Suspend,
    OpenCommandLine(CommandLineKind),
    OpenHelp,
    #[cfg(debug_assertions)]
    DebugPanic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandLineKind {
    Search(SearchDirection),
    Grep,
    Highlight,
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
    ScrollLeftWide(usize),
    ScrollRightWide(usize),
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

#[derive(Debug, Clone, PartialEq)]
pub enum BookmarkAction {
    Toggle,
    RemoveAll,
    JumpToNext,
    JumpToPrevious,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TabAction {
    Grep(String),
    SwitchToNext,
    SwitchToPrevious,
    CloseActive,
    CloseAllGrepTabs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HighlightAction {
    Toggle(String),
    RemoveAll,
}
