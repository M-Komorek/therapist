mod bindings;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

use bindings::{BindingMatch, apply_count, find_binding, plain};

#[derive(Debug, PartialEq)]
pub enum KeyResolution {
    Action(Action),
    LastKeysChanged,
    Ignored,
}

/// Turns key presses into actions. Between presses it keeps the count typed so far (`5` in
/// `5j`) and the keys of a started sequence (the first `g` of `gg`). It also remembers the keys
/// of the current or last command, so they can be shown until the next key.
#[derive(Default)]
pub struct Keymap {
    count: Option<usize>,
    started_sequence: Vec<KeyEvent>,
    last_keys: Vec<KeyEvent>,
}

impl Keymap {
    pub fn resolve(&mut self, key: KeyEvent) -> KeyResolution {
        let last_keys_before = self.last_keys.clone();
        match self.consume(key) {
            Some(action) => KeyResolution::Action(action),
            None if self.last_keys != last_keys_before => KeyResolution::LastKeysChanged,
            None => KeyResolution::Ignored,
        }
    }

    pub fn last_keys_text(&self) -> String {
        self.last_keys.iter().map(key_name).collect()
    }

    fn consume(&mut self, key: KeyEvent) -> Option<Action> {
        if key == ESCAPE {
            self.clear();
            self.last_keys.clear();
            return None;
        }
        self.remember_key(key);
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

    fn remember_key(&mut self, key: KeyEvent) {
        if !self.is_pending() {
            self.last_keys.clear();
        }
        self.last_keys.push(key);
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
        let zero_extends_count = self.count.is_some();
        (digit != 0 || zero_extends_count).then_some(digit as usize)
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

fn key_name(key: &KeyEvent) -> String {
    let mut modifiers = String::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        modifiers.push_str("Ctrl-");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        modifiers.push_str("Alt-");
    }
    match key.code {
        KeyCode::Char(character) if modifiers.is_empty() => character.to_string(),
        code => format!("<{modifiers}{code}>"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bindings::{app, char_key, ctrl, view},
        *,
    };
    use crate::action::{AppAction, ViewAction};

    fn resolve_keys(keymap: &mut Keymap, keys: &[KeyEvent]) -> KeyResolution {
        let mut resolution = KeyResolution::Ignored;
        for &key in keys {
            resolution = keymap.resolve(key);
        }
        resolution
    }

    fn resolve_typed(keymap: &mut Keymap, typed: &str) -> KeyResolution {
        let keys: Vec<KeyEvent> = typed.chars().map(char_key).collect();
        resolve_keys(keymap, &keys)
    }

    fn resolve_fresh(typed: &str) -> KeyResolution {
        resolve_typed(&mut Keymap::default(), typed)
    }

    fn resolved_view(view_action: ViewAction) -> KeyResolution {
        KeyResolution::Action(view(view_action))
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
        let resolution = resolve_keys(&mut Keymap::default(), &[char_key('5'), ctrl('d')]);

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
        resolve_typed(&mut keymap, "gx");

        assert_eq!(
            resolve_typed(&mut keymap, "gg"),
            resolved_view(ViewAction::GoToLineNumber(1))
        );
    }

    #[test]
    fn given_nothing_pending_when_count_digit_or_g_is_pressed_then_last_keys_change() {
        assert_eq!(resolve_fresh("5"), KeyResolution::LastKeysChanged);
        assert_eq!(resolve_fresh("g"), KeyResolution::LastKeysChanged);
    }

    #[test]
    fn given_pending_count_when_escape_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5");

        assert_eq!(keymap.resolve(ESCAPE), KeyResolution::LastKeysChanged);
        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_view(ViewAction::MoveDown(1))
        );
    }

    #[test]
    fn given_started_g_when_escape_is_pressed_then_next_g_starts_a_new_sequence() {
        let mut keymap = Keymap::default();

        let resolution = resolve_keys(&mut keymap, &[char_key('g'), ESCAPE, char_key('g')]);

        assert_eq!(resolution, KeyResolution::LastKeysChanged);
    }

    #[test]
    fn given_nothing_shown_when_escape_is_pressed_then_it_is_ignored() {
        assert_eq!(Keymap::default().resolve(ESCAPE), KeyResolution::Ignored);
    }

    #[test]
    fn given_pending_count_when_unmapped_key_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();

        assert_eq!(
            resolve_typed(&mut keymap, "5x"),
            KeyResolution::LastKeysChanged
        );
        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_view(ViewAction::MoveDown(1))
        );
    }

    #[test]
    fn given_count_and_started_g_when_reading_last_keys_then_they_show_as_typed() {
        let mut keymap = Keymap::default();

        resolve_typed(&mut keymap, "12g");

        assert_eq!(keymap.last_keys_text(), "12g");
    }

    #[test]
    fn given_finished_command_when_reading_last_keys_then_they_stay_until_the_next_key() {
        let mut keymap = Keymap::default();

        resolve_typed(&mut keymap, "5j");
        assert_eq!(keymap.last_keys_text(), "5j");

        resolve_typed(&mut keymap, "k");
        assert_eq!(keymap.last_keys_text(), "k");
    }

    #[test]
    fn given_last_keys_when_escape_is_pressed_then_they_are_cleared() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5j");

        assert_eq!(keymap.resolve(ESCAPE), KeyResolution::LastKeysChanged);
        assert_eq!(keymap.last_keys_text(), "");
    }

    #[test]
    fn given_keys_with_modifiers_or_names_when_reading_last_keys_then_they_use_key_notation() {
        let last_keys_text = |key| {
            let mut keymap = Keymap::default();
            keymap.resolve(key);
            keymap.last_keys_text()
        };

        assert_eq!(last_keys_text(ctrl('d')), "<Ctrl-d>");
        assert_eq!(last_keys_text(plain(KeyCode::Down)), "<Down>");
        let alt_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(last_keys_text(alt_x), "<Alt-x>");
        let shift_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(last_keys_text(shift_g), "G");
    }

    #[test]
    fn given_nothing_pending_when_unmapped_key_is_pressed_then_only_the_last_keys_change() {
        assert_eq!(resolve_fresh("x"), KeyResolution::LastKeysChanged);
    }

    #[test]
    fn given_unmapped_key_shown_when_it_is_pressed_again_then_it_is_ignored() {
        assert_eq!(resolve_fresh("xx"), KeyResolution::Ignored);
    }

    #[test]
    fn given_mapped_key_with_other_modifier_when_resolving_then_no_action_runs() {
        assert_eq!(
            Keymap::default().resolve(ctrl('q')),
            KeyResolution::LastKeysChanged
        );
    }
}
