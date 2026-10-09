use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction, LogViewAction};

/// What each key sequence does when no count is typed before it.
const BINDINGS: &[(&[KeyEvent], Action)] = &[
    // App
    #[cfg(debug_assertions)]
    (&[ctrl('p')], app(AppAction::DebugPanic)),
    (&[char_key('q')], app(AppAction::Quit)),
    (&[ctrl('c')], app(AppAction::Quit)),
    (&[ctrl('z')], app(AppAction::Suspend)),
    (&[char_key(':')], app(AppAction::OpenCommandLine)),
    // Moving by lines
    (&[char_key('j')], log_view(LogViewAction::MoveDown(1))),
    (
        &[plain(KeyCode::Down)],
        log_view(LogViewAction::MoveDown(1)),
    ),
    (&[char_key('k')], log_view(LogViewAction::MoveUp(1))),
    (&[plain(KeyCode::Up)], log_view(LogViewAction::MoveUp(1))),
    // Jumping to a line
    (
        &[char_key('g'), char_key('g')],
        log_view(LogViewAction::GoToLineNumber(1)),
    ),
    (&[char_key('G')], log_view(LogViewAction::GoToLastLine)),
    // Moving by pages
    (&[ctrl('d')], log_view(LogViewAction::HalfPageDown)),
    (&[ctrl('u')], log_view(LogViewAction::HalfPageUp)),
    (&[ctrl('f')], log_view(LogViewAction::PageDown)),
    (&[ctrl('b')], log_view(LogViewAction::PageUp)),
    // Scrolling sideways
    (&[char_key('h')], log_view(LogViewAction::ScrollLeft(1))),
    (
        &[plain(KeyCode::Left)],
        log_view(LogViewAction::ScrollLeft(1)),
    ),
    (&[char_key('l')], log_view(LogViewAction::ScrollRight(1))),
    (
        &[plain(KeyCode::Right)],
        log_view(LogViewAction::ScrollRight(1)),
    ),
    (&[char_key('H')], log_view(LogViewAction::ScrollLeftWide)),
    (&[char_key('L')], log_view(LogViewAction::ScrollRightWide)),
    (
        &[char_key('0')],
        log_view(LogViewAction::ScrollToFirstColumn),
    ),
];

pub(super) enum BindingMatch {
    Complete(&'static Action),
    Prefix,
    NotBound,
}

pub(super) fn find_binding(keys: &[KeyEvent]) -> BindingMatch {
    let complete_binding = BINDINGS
        .iter()
        .find(|(binding_keys, _)| *binding_keys == keys);
    if let Some((_, action)) = complete_binding {
        return BindingMatch::Complete(action);
    }
    let is_prefix = BINDINGS
        .iter()
        .any(|(binding_keys, _)| binding_keys.starts_with(keys));
    if is_prefix {
        BindingMatch::Prefix
    } else {
        BindingMatch::NotBound
    }
}

pub(super) fn apply_count(action: Action, count: Option<usize>) -> Action {
    let Some(count) = count else {
        return action;
    };
    match action {
        Action::LogView(LogViewAction::MoveDown(_)) => log_view(LogViewAction::MoveDown(count)),
        Action::LogView(LogViewAction::MoveUp(_)) => log_view(LogViewAction::MoveUp(count)),
        Action::LogView(LogViewAction::ScrollLeft(_)) => log_view(LogViewAction::ScrollLeft(count)),
        Action::LogView(LogViewAction::ScrollRight(_)) => {
            log_view(LogViewAction::ScrollRight(count))
        }
        Action::LogView(LogViewAction::GoToLineNumber(_) | LogViewAction::GoToLastLine) => {
            log_view(LogViewAction::GoToLineNumber(count))
        }
        _ => action,
    }
}

pub(super) const fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) const fn char_key(character: char) -> KeyEvent {
    plain(KeyCode::Char(character))
}

pub(super) const fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

pub(super) const fn app(app_action: AppAction) -> Action {
    Action::App(app_action)
}

pub(super) const fn log_view(log_view_action: LogViewAction) -> Action {
    Action::LogView(log_view_action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_every_pair_of_bindings_when_compared_then_neither_equals_nor_starts_the_other() {
        for (binding_position, (keys, _)) in BINDINGS.iter().enumerate() {
            for (other_keys, _) in &BINDINGS[binding_position + 1..] {
                assert!(
                    !keys.starts_with(other_keys) && !other_keys.starts_with(keys),
                    "{keys:?} conflicts with {other_keys:?}"
                );
            }
        }
    }
}
