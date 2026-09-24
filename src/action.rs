#[derive(Debug, PartialEq)]
pub enum Action {
    App(AppAction),
}

#[derive(Debug, PartialEq)]
pub enum AppAction {
    Quit,
    Suspend,
    #[cfg(debug_assertions)]
    DebugPanic,
}
