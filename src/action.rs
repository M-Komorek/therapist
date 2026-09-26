#[derive(Debug, PartialEq)]
pub enum Action {
    App(AppAction),
    View(ViewAction),
}

#[derive(Debug, PartialEq)]
pub enum AppAction {
    Quit,
    Suspend,
    #[cfg(debug_assertions)]
    DebugPanic,
}

#[derive(Debug, PartialEq)]
pub enum ViewAction {
    MoveUp(usize),
    MoveDown(usize),
}
