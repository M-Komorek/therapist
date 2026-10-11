mod bindings;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

use bindings::{BindingMatch, apply_count, find_binding, plain};

/// Turns normal-mode keys into actions. It remembers a count typed so far (`5` in `5j`), which
/// replaces the number in the bound action, and the start of a key sequence (the first `g` of
/// `gg`). It also keeps the last command's keys, which the status bar shows.
#[derive(Default)]
pub(super) struct Keymap {
    count: Option<usize>,
    started_sequence: Vec<KeyEvent>,
    last_keys: Vec<KeyEvent>,
}

impl Keymap {
    pub(super) fn resolve(&mut self, key: KeyEvent) -> Option<Action> {
        if key == ESCAPE && self.is_pending() {
            self.reset();
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
                self.clear_pending();
                Some(action)
            }
            BindingMatch::Partial => None,
            BindingMatch::NotBound => {
                self.clear_pending();
                None
            }
        }
    }

    pub(super) fn last_keys_text(&self) -> String {
        self.last_keys.iter().map(key_name).collect()
    }

    pub(super) fn reset(&mut self) {
        self.clear_pending();
        self.last_keys.clear();
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

    fn clear_pending(&mut self) {
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
        bindings::{app, char_key, ctrl, log_view, search},
        *,
    };
    use crate::{
        action::{AppAction, CommandLineKind, LogViewAction, SearchAction},
        domain::search::SearchDirection,
    };

    fn resolve_keys(keymap: &mut Keymap, keys: &[KeyEvent]) -> Option<Action> {
        let mut resolution = None;
        for &key in keys {
            resolution = keymap.resolve(key);
        }
        resolution
    }

    fn resolve_typed(keymap: &mut Keymap, typed: &str) -> Option<Action> {
        let keys: Vec<KeyEvent> = typed.chars().map(char_key).collect();
        resolve_keys(keymap, &keys)
    }

    fn resolve_fresh(typed: &str) -> Option<Action> {
        resolve_typed(&mut Keymap::default(), typed)
    }

    fn resolved_log_view(log_view_action: LogViewAction) -> Option<Action> {
        Some(log_view(log_view_action))
    }

    #[test]
    fn given_q_or_ctrl_c_when_resolving_then_app_quits() {
        let quit = Some(app(AppAction::Quit));

        assert_eq!(resolve_fresh("q"), quit);
        assert_eq!(Keymap::default().resolve(ctrl('c')), quit);
    }

    #[test]
    fn given_colon_when_resolving_then_the_command_line_opens() {
        assert_eq!(
            resolve_fresh(":"),
            Some(app(AppAction::OpenCommandLine(CommandLineKind::Command)))
        );
    }

    #[test]
    fn given_slash_or_question_mark_when_resolving_then_a_search_line_opens() {
        let open_search_line = |search_direction| {
            Some(app(AppAction::OpenCommandLine(CommandLineKind::Search(
                search_direction,
            ))))
        };

        assert_eq!(
            resolve_fresh("/"),
            open_search_line(SearchDirection::Forward)
        );
        assert_eq!(
            resolve_fresh("?"),
            open_search_line(SearchDirection::Backward)
        );
    }

    #[test]
    fn given_n_or_capital_n_when_resolving_then_it_jumps_to_the_next_or_previous_match() {
        assert_eq!(
            resolve_fresh("n"),
            Some(search(SearchAction::JumpToNextMatch))
        );
        assert_eq!(
            resolve_fresh("N"),
            Some(search(SearchAction::JumpToPreviousMatch))
        );
    }

    #[test]
    fn given_count_and_last_keys_when_reset_then_both_are_forgotten() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5");

        keymap.reset();

        assert_eq!(keymap.last_keys_text(), "");
        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_log_view(LogViewAction::MoveCursorDown(1))
        );
    }

    #[test]
    fn given_ctrl_z_when_resolving_then_app_suspends() {
        assert_eq!(
            Keymap::default().resolve(ctrl('z')),
            Some(app(AppAction::Suspend))
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn given_ctrl_p_in_debug_build_when_resolving_then_app_panics_on_purpose() {
        assert_eq!(
            Keymap::default().resolve(ctrl('p')),
            Some(app(AppAction::DebugPanic))
        );
    }

    #[test]
    fn given_j_or_down_arrow_when_resolving_then_cursor_moves_down_one_line() {
        let move_cursor_down = resolved_log_view(LogViewAction::MoveCursorDown(1));

        assert_eq!(resolve_fresh("j"), move_cursor_down);
        assert_eq!(
            Keymap::default().resolve(plain(KeyCode::Down)),
            move_cursor_down
        );
    }

    #[test]
    fn given_k_or_up_arrow_when_resolving_then_cursor_moves_up_one_line() {
        let move_cursor_up = resolved_log_view(LogViewAction::MoveCursorUp(1));

        assert_eq!(resolve_fresh("k"), move_cursor_up);
        assert_eq!(
            Keymap::default().resolve(plain(KeyCode::Up)),
            move_cursor_up
        );
    }

    #[test]
    fn given_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        assert_eq!(
            resolve_fresh("5j"),
            resolved_log_view(LogViewAction::MoveCursorDown(5))
        );
    }

    #[test]
    fn given_count_when_h_or_l_is_pressed_then_view_scrolls_that_many_columns() {
        assert_eq!(
            resolve_fresh("3h"),
            resolved_log_view(LogViewAction::ScrollLeft(3))
        );
        assert_eq!(
            resolve_fresh("3l"),
            resolved_log_view(LogViewAction::ScrollRight(3))
        );
    }

    #[test]
    fn given_left_or_right_arrow_when_resolving_then_view_scrolls_one_column() {
        let resolve = |code| Keymap::default().resolve(plain(code));

        assert_eq!(
            resolve(KeyCode::Left),
            resolved_log_view(LogViewAction::ScrollLeft(1))
        );
        assert_eq!(
            resolve(KeyCode::Right),
            resolved_log_view(LogViewAction::ScrollRight(1))
        );
    }

    #[test]
    fn given_finished_motion_with_count_when_next_motion_is_pressed_then_count_does_not_carry_over()
    {
        assert_eq!(
            resolve_fresh("5jj"),
            resolved_log_view(LogViewAction::MoveCursorDown(1))
        );
    }

    #[test]
    fn given_g_pressed_twice_when_resolving_then_cursor_goes_to_first_line() {
        assert_eq!(
            resolve_fresh("gg"),
            resolved_log_view(LogViewAction::MoveCursorToLineNumber(1))
        );
    }

    #[test]
    fn given_count_when_gg_is_pressed_then_cursor_goes_to_that_line() {
        assert_eq!(
            resolve_fresh("5gg"),
            resolved_log_view(LogViewAction::MoveCursorToLineNumber(5))
        );
    }

    #[test]
    fn given_no_count_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        assert_eq!(
            resolve_fresh("G"),
            resolved_log_view(LogViewAction::MoveCursorToLastLine)
        );
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line() {
        assert_eq!(
            resolve_fresh("5G"),
            resolved_log_view(LogViewAction::MoveCursorToLineNumber(5))
        );
    }

    #[test]
    fn given_terminal_sends_shift_with_capital_letter_when_resolving_then_it_still_matches() {
        let shift_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);

        assert_eq!(
            Keymap::default().resolve(shift_g),
            resolved_log_view(LogViewAction::MoveCursorToLastLine)
        );
    }

    #[test]
    fn given_page_keys_when_resolving_then_they_move_half_or_full_pages() {
        let resolve = |key| Keymap::default().resolve(key);

        assert_eq!(
            resolve(ctrl('d')),
            resolved_log_view(LogViewAction::ScrollHalfPageDown)
        );
        assert_eq!(
            resolve(ctrl('u')),
            resolved_log_view(LogViewAction::ScrollHalfPageUp)
        );
        assert_eq!(
            resolve(ctrl('f')),
            resolved_log_view(LogViewAction::ScrollPageDown)
        );
        assert_eq!(
            resolve(ctrl('b')),
            resolved_log_view(LogViewAction::ScrollPageUp)
        );
    }

    #[test]
    fn given_count_when_page_key_is_pressed_then_count_is_ignored() {
        let resolution = resolve_keys(&mut Keymap::default(), &[char_key('5'), ctrl('d')]);

        assert_eq!(
            resolution,
            resolved_log_view(LogViewAction::ScrollHalfPageDown)
        );
    }

    #[test]
    fn given_capital_h_or_l_when_resolving_then_view_scrolls_wide() {
        assert_eq!(
            resolve_fresh("H"),
            resolved_log_view(LogViewAction::ScrollLeftWide)
        );
        assert_eq!(
            resolve_fresh("L"),
            resolved_log_view(LogViewAction::ScrollRightWide)
        );
    }

    #[test]
    fn given_no_count_when_0_is_pressed_then_view_scrolls_to_first_column() {
        assert_eq!(
            resolve_fresh("0"),
            resolved_log_view(LogViewAction::ScrollToFirstColumn)
        );
    }

    #[test]
    fn given_count_digit_when_0_is_pressed_then_it_extends_the_count() {
        assert_eq!(
            resolve_fresh("10k"),
            resolved_log_view(LogViewAction::MoveCursorUp(10))
        );
    }

    #[test]
    fn given_huge_count_when_j_is_pressed_then_count_saturates() {
        assert_eq!(
            resolve_fresh("99999999999999999999999j"),
            resolved_log_view(LogViewAction::MoveCursorDown(usize::MAX))
        );
    }

    #[test]
    fn given_started_g_when_j_is_pressed_then_cursor_moves_down() {
        assert_eq!(
            resolve_fresh("gj"),
            resolved_log_view(LogViewAction::MoveCursorDown(1))
        );
    }

    #[test]
    fn given_count_and_started_g_when_j_is_pressed_then_count_still_applies() {
        assert_eq!(
            resolve_fresh("3gj"),
            resolved_log_view(LogViewAction::MoveCursorDown(3))
        );
    }

    #[test]
    fn given_started_g_when_digit_is_pressed_then_g_is_dropped_and_digit_starts_a_count() {
        assert_eq!(
            resolve_fresh("g5j"),
            resolved_log_view(LogViewAction::MoveCursorDown(5))
        );
    }

    #[test]
    fn given_started_g_when_unmapped_key_is_pressed_then_next_g_starts_a_new_sequence() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "gx");

        assert_eq!(
            resolve_typed(&mut keymap, "gg"),
            resolved_log_view(LogViewAction::MoveCursorToLineNumber(1))
        );
    }

    #[test]
    fn given_nothing_pending_when_count_digit_or_g_is_pressed_then_no_action_runs_yet() {
        assert_eq!(resolve_fresh("5"), None);
        assert_eq!(resolve_fresh("g"), None);
    }

    #[test]
    fn given_pending_count_when_escape_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5");

        keymap.resolve(ESCAPE);

        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_log_view(LogViewAction::MoveCursorDown(1))
        );
    }

    #[test]
    fn given_started_g_when_escape_is_pressed_then_next_g_starts_a_new_sequence() {
        let mut keymap = Keymap::default();

        let resolution = resolve_keys(&mut keymap, &[char_key('g'), ESCAPE, char_key('g')]);

        assert_eq!(resolution, None);
    }

    #[test]
    fn given_pending_count_when_unmapped_key_is_pressed_then_count_is_dropped() {
        let mut keymap = Keymap::default();

        resolve_typed(&mut keymap, "5x");

        assert_eq!(
            resolve_typed(&mut keymap, "j"),
            resolved_log_view(LogViewAction::MoveCursorDown(1))
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
    fn given_pending_count_when_escape_is_pressed_then_nothing_resolves_and_last_keys_are_cleared()
    {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5");

        let resolution = keymap.resolve(ESCAPE);

        assert_eq!(resolution, None);
        assert_eq!(keymap.last_keys_text(), "");
    }

    #[test]
    fn given_started_g_when_escape_is_pressed_then_nothing_resolves() {
        let mut keymap = Keymap::default();

        let resolution = resolve_keys(&mut keymap, &[char_key('g'), ESCAPE]);

        assert_eq!(resolution, None);
    }

    #[test]
    fn given_nothing_pending_when_escape_is_pressed_then_the_search_is_cleared() {
        let mut keymap = Keymap::default();
        resolve_typed(&mut keymap, "5j");

        let resolution = keymap.resolve(ESCAPE);

        assert_eq!(resolution, Some(search(SearchAction::Clear)));
        assert_eq!(keymap.last_keys_text(), "<Esc>");
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
    fn given_nothing_pending_when_unmapped_key_is_pressed_then_no_action_runs_and_the_key_is_shown()
    {
        let mut keymap = Keymap::default();

        assert_eq!(resolve_typed(&mut keymap, "x"), None);
        assert_eq!(keymap.last_keys_text(), "x");
    }

    #[test]
    fn given_mapped_key_with_other_modifier_when_resolving_then_no_action_runs() {
        assert_eq!(Keymap::default().resolve(ctrl('q')), None);
    }
}
