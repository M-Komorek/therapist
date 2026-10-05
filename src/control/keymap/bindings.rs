use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction, ViewAction};

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
    (&[char_key('j')], view(ViewAction::MoveDown(1))),
    (&[plain(KeyCode::Down)], view(ViewAction::MoveDown(1))),
    (&[char_key('k')], view(ViewAction::MoveUp(1))),
    (&[plain(KeyCode::Up)], view(ViewAction::MoveUp(1))),
    // Jumping to a line
    (
        &[char_key('g'), char_key('g')],
        view(ViewAction::GoToLineNumber(1)),
    ),
    (&[char_key('G')], view(ViewAction::GoToLastLine)),
    // Moving by pages
    (&[ctrl('d')], view(ViewAction::HalfPageDown)),
    (&[ctrl('u')], view(ViewAction::HalfPageUp)),
    (&[ctrl('f')], view(ViewAction::PageDown)),
    (&[ctrl('b')], view(ViewAction::PageUp)),
    // Scrolling sideways
    (&[char_key('h')], view(ViewAction::ScrollLeft(1))),
    (&[plain(KeyCode::Left)], view(ViewAction::ScrollLeft(1))),
    (&[char_key('l')], view(ViewAction::ScrollRight(1))),
    (&[plain(KeyCode::Right)], view(ViewAction::ScrollRight(1))),
    (&[char_key('H')], view(ViewAction::ScrollLeftWide)),
    (&[char_key('L')], view(ViewAction::ScrollRightWide)),
    (&[char_key('0')], view(ViewAction::ScrollToFirstColumn)),
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
        Action::View(ViewAction::MoveDown(_)) => view(ViewAction::MoveDown(count)),
        Action::View(ViewAction::MoveUp(_)) => view(ViewAction::MoveUp(count)),
        Action::View(ViewAction::ScrollLeft(_)) => view(ViewAction::ScrollLeft(count)),
        Action::View(ViewAction::ScrollRight(_)) => view(ViewAction::ScrollRight(count)),
        Action::View(ViewAction::GoToLineNumber(_) | ViewAction::GoToLastLine) => {
            view(ViewAction::GoToLineNumber(count))
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

pub(super) const fn view(view_action: ViewAction) -> Action {
    Action::View(view_action)
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
