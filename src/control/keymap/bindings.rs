use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction, LogViewAction};

const BINDINGS: &[(&[KeyEvent], Action)] = &[
    // App
    #[cfg(debug_assertions)]
    (&[ctrl('p')], app(AppAction::DebugPanic)),
    (&[char_key('q')], app(AppAction::Quit)),
    (&[ctrl('c')], app(AppAction::Quit)),
    (&[ctrl('z')], app(AppAction::Suspend)),
    (&[char_key(':')], app(AppAction::OpenCommandLine)),
    // Moving by lines
    (&[char_key('j')], log_view(LogViewAction::MoveCursorDown(1))),
    (
        &[plain(KeyCode::Down)],
        log_view(LogViewAction::MoveCursorDown(1)),
    ),
    (&[char_key('k')], log_view(LogViewAction::MoveCursorUp(1))),
    (
        &[plain(KeyCode::Up)],
        log_view(LogViewAction::MoveCursorUp(1)),
    ),
    // Jumping to a line
    (
        &[char_key('g'), char_key('g')],
        log_view(LogViewAction::MoveCursorToLineNumber(1)),
    ),
    (
        &[char_key('G')],
        log_view(LogViewAction::MoveCursorToLastLine),
    ),
    // Moving by pages
    (&[ctrl('d')], log_view(LogViewAction::ScrollHalfPageDown)),
    (&[ctrl('u')], log_view(LogViewAction::ScrollHalfPageUp)),
    (&[ctrl('f')], log_view(LogViewAction::ScrollPageDown)),
    (&[ctrl('b')], log_view(LogViewAction::ScrollPageUp)),
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
    Partial,
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
        BindingMatch::Partial
    } else {
        BindingMatch::NotBound
    }
}

pub(super) fn apply_count(action: Action, count: Option<usize>) -> Action {
    let Some(count) = count else {
        return action;
    };
    match action {
        Action::LogView(LogViewAction::MoveCursorDown(_)) => {
            log_view(LogViewAction::MoveCursorDown(count))
        }
        Action::LogView(LogViewAction::MoveCursorUp(_)) => {
            log_view(LogViewAction::MoveCursorUp(count))
        }
        Action::LogView(LogViewAction::ScrollLeft(_)) => log_view(LogViewAction::ScrollLeft(count)),
        Action::LogView(LogViewAction::ScrollRight(_)) => {
            log_view(LogViewAction::ScrollRight(count))
        }
        Action::LogView(
            LogViewAction::MoveCursorToLineNumber(_) | LogViewAction::MoveCursorToLastLine,
        ) => log_view(LogViewAction::MoveCursorToLineNumber(count)),
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
