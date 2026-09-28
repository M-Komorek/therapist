use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction, ViewAction};

#[derive(Debug, PartialEq)]
pub enum KeyResolution {
    Action(Action),
    PendingChanged,
    Ignored,
}

/// Turns key presses into actions. Between presses it keeps the count typed so far (`5` in
/// `5j`) and the keys of a started sequence (the first `g` of `gg`).
#[derive(Default)]
pub struct Keymap {
    count: Option<usize>,
    started_sequence: Vec<KeyEvent>,
}

impl Keymap {
    pub fn resolve(&mut self, key: KeyEvent) -> KeyResolution {
        let was_pending = self.is_pending();
        match self.consume(key) {
            Some(action) => KeyResolution::Action(action),
            None if was_pending || self.is_pending() => KeyResolution::PendingChanged,
            None => KeyResolution::Ignored,
        }
    }

    fn consume(&mut self, key: KeyEvent) -> Option<Action> {
        if key == ESCAPE {
            self.clear();
            return None;
        }
        if self.breaks_started_sequence(key) {
            self.started_sequence.clear();
        }
        if let Some(digit) = self.count_digit(key) {
            self.add_count_digit(digit);
            return None;
        }

        self.started_sequence.push(key);
        match find_binding(&self.started_sequence) {
            BindingMatch::Complete(action) => {
                let action = apply_count(action.clone(), self.count);
                self.clear();
                Some(action)
            }
            BindingMatch::Prefix => None,
            BindingMatch::NotBound => {
                self.clear();
                None
            }
        }
    }

    fn breaks_started_sequence(&self, key: KeyEvent) -> bool {
        if self.started_sequence.is_empty() {
            return false;
        }
        let extended_sequence = [self.started_sequence.as_slice(), &[key]].concat();
        matches!(find_binding(&extended_sequence), BindingMatch::NotBound)
    }

    fn count_digit(&self, key: KeyEvent) -> Option<usize> {
        if !self.started_sequence.is_empty() || !key.modifiers.is_empty() {
            return None;
        }
        let KeyCode::Char(character) = key.code else {
            return None;
        };
        let digit = character.to_digit(10)?;
        (digit != 0 || self.count.is_some()).then_some(digit as usize)
    }

    fn add_count_digit(&mut self, digit: usize) {
        let count = self
            .count
            .unwrap_or(0)
            .saturating_mul(10)
            .saturating_add(digit);
        self.count = Some(count);
    }

    fn is_pending(&self) -> bool {
        self.count.is_some() || !self.started_sequence.is_empty()
    }

    fn clear(&mut self) {
        self.count = None;
        self.started_sequence.clear();
    }
}

const ESCAPE: KeyEvent = plain(KeyCode::Esc);

/// What each key sequence does when no count is typed before it.
const BINDINGS: &[(&[KeyEvent], Action)] = &[
    // App
    #[cfg(debug_assertions)]
    (&[ctrl('p')], app(AppAction::DebugPanic)),
    (&[key('q')], app(AppAction::Quit)),
    (&[ctrl('c')], app(AppAction::Quit)),
    (&[ctrl('z')], app(AppAction::Suspend)),
    // Moving by lines
    (&[key('j')], view(ViewAction::MoveDown(1))),
    (&[plain(KeyCode::Down)], view(ViewAction::MoveDown(1))),
    (&[key('k')], view(ViewAction::MoveUp(1))),
    (&[plain(KeyCode::Up)], view(ViewAction::MoveUp(1))),
    // Jumping to a line
    (&[key('g'), key('g')], view(ViewAction::GoToLineNumber(1))),
    (&[key('G')], view(ViewAction::GoToLastLine)),
    // Moving by pages
    (&[ctrl('d')], view(ViewAction::HalfPageDown)),
    (&[ctrl('u')], view(ViewAction::HalfPageUp)),
    (&[ctrl('f')], view(ViewAction::PageDown)),
    (&[ctrl('b')], view(ViewAction::PageUp)),
    // Scrolling sideways
    (&[key('h')], view(ViewAction::ScrollLeft(1))),
    (&[plain(KeyCode::Left)], view(ViewAction::ScrollLeft(1))),
    (&[key('l')], view(ViewAction::ScrollRight(1))),
    (&[plain(KeyCode::Right)], view(ViewAction::ScrollRight(1))),
    (&[key('H')], view(ViewAction::ScrollLeftWide)),
    (&[key('L')], view(ViewAction::ScrollRightWide)),
    (&[key('0')], view(ViewAction::ScrollToFirstColumn)),
];

fn apply_count(action: Action, count: Option<usize>) -> Action {
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

enum BindingMatch {
    Complete(&'static Action),
    Prefix,
    NotBound,
}

fn find_binding(keys: &[KeyEvent]) -> BindingMatch {
    if let Some((_, action)) = BINDINGS.iter().find(|(binding, _)| *binding == keys) {
        return BindingMatch::Complete(action);
    }
    let is_prefix = BINDINGS
        .iter()
        .any(|(binding, _)| binding.starts_with(keys));
    if is_prefix {
        BindingMatch::Prefix
    } else {
        BindingMatch::NotBound
    }
}

const fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

const fn key(character: char) -> KeyEvent {
    plain(KeyCode::Char(character))
}

const fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

const fn app(app_action: AppAction) -> Action {
    Action::App(app_action)
}

const fn view(view_action: ViewAction) -> Action {
    Action::View(view_action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve_keys(keymap: &mut Keymap, keys: &[KeyEvent]) -> KeyResolution {
        let mut resolution = KeyResolution::Ignored;
        for &key in keys {
            resolution = keymap.resolve(key);
        }
        resolution
    }

    fn resolve_typed(keymap: &mut Keymap, typed: &str) -> KeyResolution {
        let keys: Vec<KeyEvent> = typed.chars().map(key).collect();
        resolve_keys(keymap, &keys)
    }

    fn resolve_fresh(typed: &str) -> KeyResolution {
        resolve_typed(&mut Keymap::default(), typed)
    }

    fn resolved_view(view_action: ViewAction) -> KeyResolution {
        KeyResolution::Action(view(view_action))
    }

    #[test]
    fn given_every_pair_of_bindings_when_compared_then_neither_equals_nor_starts_the_other() {
        for (index, (keys, _)) in BINDINGS.iter().enumerate() {
            for (other_keys, _) in &BINDINGS[index + 1..] {
                assert!(
                    !keys.starts_with(other_keys) && !other_keys.starts_with(keys),
                    "{keys:?} conflicts with {other_keys:?}"
                );
            }
        }
    }

    #[test]
    fn given_q_or_ctrl_c_when_resolving_then_app_quits() {
        let quit = KeyResolution::Action(app(AppAction::Quit));

        assert_eq!(resolve_fresh("q"), quit);
        assert_eq!(Keymap::default().resolve(ctrl('c')), quit);
    }

    #[test]
    fn given_ctrl_z_when_resolving_then_app_suspends() {
        assert_eq!(
            Keymap::default().resolve(ctrl('z')),
            KeyResolution::Action(app(AppAction::Suspend))
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn given_ctrl_p_in_debug_build_when_resolving_then_app_panics_on_purpose() {
        assert_eq!(
            Keymap::default().resolve(ctrl('p')),
            KeyResolution::Action(app(AppAction::DebugPanic))
        );
    }

    #[test]
    fn given_j_or_down_arrow_when_resolving_then_cursor_moves_down_one_line() {
        let move_down = resolved_view(ViewAction::MoveDown(1));

        assert_eq!(resolve_fresh("j"), move_down);
        assert_eq!(Keymap::default().resolve(plain(KeyCode::Down)), move_down);
    }

    #[test]
    fn given_k_or_up_arrow_when_resolving_then_cursor_moves_up_one_line() {
        let move_up = resolved_view(ViewAction::MoveUp(1));

        assert_eq!(resolve_fresh("k"), move_up);
        assert_eq!(Keymap::default().resolve(plain(KeyCode::Up)), move_up);
    }

    #[test]
    fn given_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        assert_eq!(resolve_fresh("5j"), resolved_view(ViewAction::MoveDown(5)));
    }

    #[test]
    fn given_count_when_h_or_l_is_pressed_then_view_scrolls_that_many_columns() {
        assert_eq!(
            resolve_fresh("3h"),
            resolved_view(ViewAction::ScrollLeft(3))
        );
        assert_eq!(
            resolve_fresh("3l"),
            resolved_view(ViewAction::ScrollRight(3))
        );
    }

    #[test]
    fn given_left_or_right_arrow_when_resolving_then_view_scrolls_one_column() {
        let resolve = |code| Keymap::default().resolve(plain(code));

        assert_eq!(
            resolve(KeyCode::Left),
            resolved_view(ViewAction::ScrollLeft(1))
        );
        assert_eq!(
            resolve(KeyCode::Right),
            resolved_view(ViewAction::ScrollRight(1))
        );
    }

    #[test]
    fn given_finished_motion_with_count_when_next_motion_is_pressed_then_count_does_not_carry_over()
    {
        assert_eq!(resolve_fresh("5jj"), resolved_view(ViewAction::MoveDown(1)));
    }

    #[test]
    fn given_g_pressed_twice_when_resolving_then_cursor_goes_to_first_line() {
        assert_eq!(
            resolve_fresh("gg"),
            resolved_view(ViewAction::GoToLineNumber(1))
        );
    }

    #[test]
    fn given_count_when_gg_is_pressed_then_cursor_goes_to_that_line() {
        assert_eq!(
            resolve_fresh("5gg"),
            resolved_view(ViewAction::GoToLineNumber(5))
        );
    }

    #[test]
    fn given_no_count_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        assert_eq!(resolve_fresh("G"), resolved_view(ViewAction::GoToLastLine));
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line() {
        assert_eq!(
            resolve_fresh("5G"),
            resolved_view(ViewAction::GoToLineNumber(5))
        );
    }

    #[test]
    fn given_terminal_sends_shift_with_capital_letter_when_resolving_then_it_still_matches() {
        let shift_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);

        assert_eq!(
            Keymap::default().resolve(shift_g),
            resolved_view(ViewAction::GoToLastLine)
        );
    }

    #[test]
    fn given_page_keys_when_resolving_then_they_move_half_or_full_pages() {
        let resolve = |key| Keymap::default().resolve(key);

        assert_eq!(resolve(ctrl('d')), resolved_view(ViewAction::HalfPageDown));
        assert_eq!(resolve(ctrl('u')), resolved_view(ViewAction::HalfPageUp));
        assert_eq!(resolve(ctrl('f')), resolved_view(ViewAction::PageDown));
        assert_eq!(resolve(ctrl('b')), resolved_view(ViewAction::PageUp));
    }

    #[test]
    fn given_count_when_page_key_is_pressed_then_count_is_ignored() {
        let resolution = resolve_keys(&mut Keymap::default(), &[key('5'), ctrl('d')]);

        assert_eq!(resolution, resolved_view(ViewAction::HalfPageDown));
    }

    #[test]
    fn given_capital_h_or_l_when_resolving_then_view_scrolls_wide() {
        assert_eq!(
            resolve_fresh("H"),
            resolved_view(ViewAction::ScrollLeftWide)
        );
        assert_eq!(
            resolve_fresh("L"),
            resolved_view(ViewAction::ScrollRightWide)
        );
    }

    #[test]
    fn given_no_count_when_0_is_pressed_then_view_scrolls_to_first_column() {
        assert_eq!(
            resolve_fresh("0"),
            resolved_view(ViewAction::ScrollToFirstColumn)
        );
    }

    #[test]
    fn given_count_digit_when_0_is_pressed_then_it_extends_the_count() {
        assert_eq!(resolve_fresh("10k"), resolved_view(ViewAction::MoveUp(10)));
    }

    #[test]
    fn given_huge_count_when_j_is_pressed_then_count_saturates() {
        assert_eq!(
            resolve_fresh("99999999999999999999999j"),
            resolved_view(ViewAction::MoveDown(usize::MAX))
        );
    }

    #[test]
    fn given_started_g_when_j_is_pressed_then_cursor_moves_down() {
        assert_eq!(resolve_fresh("gj"), resolved_view(ViewAction::MoveDown(1)));
    }

    #[test]
    fn given_count_and_started_g_when_j_is_pressed_then_count_still_applies() {
        assert_eq!(resolve_fresh("3gj"), resolved_view(ViewAction::MoveDown(3)));
    }

    #[test]
    fn given_started_g_when_digit_is_pressed_then_g_is_dropped_and_digit_starts_a_count() {
        assert_eq!(resolve_fresh("g5j"), resolved_view(ViewAction::MoveDown(5)));
    }

    #[test]
    fn given_started_g_when_unmapped_key_is_pressed_then_next_g_starts_a_new_sequence() {
        let mut keymap = Keymap::default();

        assert_eq!(
            resolve_typed(&mut keymap, "gx"),
            KeyResolution::PendingChanged
        );
        assert_eq!(
            resolve_typed(&mut keymap, "g"),
            KeyResolution::PendingChanged
        );
    }

    #[test]
    fn given_nothing_pending_when_count_digit_or_g_is_pressed_then_pending_keys_change() {
        assert_eq!(resolve_fresh("5"), KeyResolution::PendingChanged);
        assert_eq!(resolve_fresh("g"), KeyResolution::PendingChanged);
    }

    #[test]
    fn given_pending_count_when_escape_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5");

        assert_eq!(keymap.resolve(ESCAPE), KeyResolution::PendingChanged);
        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_view(ViewAction::MoveDown(1))
        );
    }

    #[test]
    fn given_started_g_when_escape_is_pressed_then_next_g_starts_a_new_sequence() {
        let mut keymap = Keymap::default();

        let resolution = resolve_keys(&mut keymap, &[key('g'), ESCAPE, key('g')]);

        assert_eq!(resolution, KeyResolution::PendingChanged);
    }

    #[test]
    fn given_nothing_pending_when_escape_is_pressed_then_it_is_ignored() {
        assert_eq!(Keymap::default().resolve(ESCAPE), KeyResolution::Ignored);
    }

    #[test]
    fn given_pending_count_when_unmapped_key_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();

        assert_eq!(
            resolve_typed(&mut keymap, "5x"),
            KeyResolution::PendingChanged
        );
        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_view(ViewAction::MoveDown(1))
        );
    }

    #[test]
    fn given_nothing_pending_when_unmapped_key_is_pressed_then_it_is_ignored() {
        assert_eq!(resolve_fresh("x"), KeyResolution::Ignored);
    }

    #[test]
    fn given_mapped_key_with_other_modifier_when_resolving_then_it_is_ignored() {
        assert_eq!(Keymap::default().resolve(ctrl('q')), KeyResolution::Ignored);
    }
}
