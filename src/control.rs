mod command_line;
mod keymap;

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::action::{Action, CommandLineKind};

pub use command_line::CommandLine;
use command_line::CommandLineOutcome;
use keymap::Keymap;
pub use keymap::{KEY_BINDING_GROUPS, KeyBinding, KeyBindingGroup, key_sequence_text};

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    CommandLine(CommandLine),
    Help,
}

#[derive(Default)]
pub struct InputController {
    keymap: Keymap,
    mode: Mode,
}

impl InputController {
    pub fn resolve(&mut self, key: KeyEvent) -> Option<Action> {
        match &mut self.mode {
            Mode::Normal => self.keymap.resolve(key),
            Mode::CommandLine(command_line) => {
                let outcome = command_line.resolve(key);
                self.apply_command_line_outcome(outcome)
            }
            Mode::Help => {
                self.resolve_help_key(key);
                None
            }
        }
    }

    pub fn open_command_line(&mut self, command_line_kind: CommandLineKind) {
        self.keymap.reset();
        self.mode = Mode::CommandLine(CommandLine::new(command_line_kind));
    }

    pub fn open_help(&mut self) {
        self.keymap.reset();
        self.mode = Mode::Help;
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn last_keys_text(&self) -> String {
        self.keymap.last_keys_text()
    }

    fn apply_command_line_outcome(&mut self, outcome: CommandLineOutcome) -> Option<Action> {
        match outcome {
            CommandLineOutcome::Editing => None,
            CommandLineOutcome::Closed => {
                self.mode = Mode::Normal;
                None
            }
            CommandLineOutcome::Submitted(action) => {
                self.mode = Mode::Normal;
                Some(action)
            }
        }
    }

    fn resolve_help_key(&mut self, key: KeyEvent) {
        if is_help_closing_key(key) {
            self.mode = Mode::Normal;
        }
    }
}

fn is_help_closing_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        action::{LogViewAction, TabAction},
        domain::search::SearchDirection,
    };

    fn type_keys(input_controller: &mut InputController, typed: &str) -> Option<Action> {
        let mut resolution = None;
        for character in typed.chars() {
            resolution = input_controller.resolve(KeyCode::Char(character).into());
        }
        resolution
    }

    fn press(input_controller: &mut InputController, code: KeyCode) -> Option<Action> {
        input_controller.resolve(code.into())
    }

    fn make_open_command_line(typed: &str) -> InputController {
        let mut input_controller = InputController::default();
        input_controller.open_command_line(CommandLineKind::Grep);
        type_keys(&mut input_controller, typed);
        input_controller
    }

    fn command_line_text(input_controller: &InputController) -> Option<&str> {
        match input_controller.mode() {
            Mode::CommandLine(command_line) => Some(command_line.text()),
            Mode::Normal | Mode::Help => None,
        }
    }

    fn make_open_help() -> InputController {
        let mut input_controller = InputController::default();
        input_controller.open_help();
        input_controller
    }

    fn is_help_open(input_controller: &InputController) -> bool {
        matches!(input_controller.mode(), Mode::Help)
    }

    #[test]
    fn given_normal_mode_when_a_bound_key_is_pressed_then_the_keymap_resolves_it() {
        let mut input_controller = InputController::default();

        assert_eq!(
            type_keys(&mut input_controller, "j"),
            Some(Action::LogView(LogViewAction::MoveCursorDown(1)))
        );
    }

    #[test]
    fn given_normal_mode_when_the_command_line_opens_then_it_is_empty() {
        let mut input_controller = InputController::default();

        input_controller.open_command_line(CommandLineKind::Grep);

        assert_eq!(command_line_text(&input_controller), Some(""));
    }

    #[test]
    fn given_normal_mode_when_a_search_line_opens_then_it_is_empty_with_the_search_prefix() {
        let mut input_controller = InputController::default();

        input_controller.open_command_line(CommandLineKind::Search(SearchDirection::Backward));

        let Mode::CommandLine(command_line) = input_controller.mode() else {
            panic!("the search line is not open");
        };
        assert_eq!(command_line.prefix(), '?');
        assert_eq!(command_line.text(), "");
    }

    #[test]
    fn given_open_command_line_when_keys_are_typed_then_they_edit_it_instead_of_the_keymap() {
        let mut input_controller = make_open_command_line("");

        let resolution = type_keys(&mut input_controller, "jq");

        assert_eq!(resolution, None);
        assert_eq!(command_line_text(&input_controller), Some("jq"));
    }

    #[test]
    fn given_phrase_on_the_command_line_when_enter_is_pressed_then_its_action_comes_out_and_it_closes()
     {
        let mut input_controller = make_open_command_line("ERROR");

        let resolution = press(&mut input_controller, KeyCode::Enter);

        assert_eq!(
            resolution,
            Some(Action::Tab(TabAction::Grep("ERROR".to_owned())))
        );
        assert_eq!(command_line_text(&input_controller), None);
    }

    #[test]
    fn given_text_on_the_command_line_when_esc_is_pressed_then_it_closes_without_an_action() {
        let mut input_controller = make_open_command_line("2");

        assert_eq!(press(&mut input_controller, KeyCode::Esc), None);
        assert_eq!(command_line_text(&input_controller), None);
    }

    #[test]
    fn given_count_typed_when_the_command_line_opens_and_closes_then_the_count_and_last_keys_are_gone()
     {
        let mut input_controller = InputController::default();
        type_keys(&mut input_controller, "2/");
        input_controller.open_command_line(CommandLineKind::Grep);
        press(&mut input_controller, KeyCode::Esc);

        assert_eq!(input_controller.last_keys_text(), "");
        assert_eq!(
            type_keys(&mut input_controller, "j"),
            Some(Action::LogView(LogViewAction::MoveCursorDown(1)))
        );
    }

    #[test]
    fn given_open_help_when_esc_is_pressed_then_it_closes() {
        let mut input_controller = make_open_help();

        assert_eq!(press(&mut input_controller, KeyCode::Esc), None);
        assert!(!is_help_open(&input_controller));
    }

    #[test]
    fn given_open_help_when_q_or_f1_is_pressed_then_it_stays_open() {
        for other_code in [KeyCode::Char('q'), KeyCode::F(1)] {
            let mut input_controller = make_open_help();

            assert_eq!(press(&mut input_controller, other_code), None);
            assert!(is_help_open(&input_controller));
        }
    }

    #[test]
    fn given_open_help_when_bound_keys_are_typed_then_no_action_comes_out_and_it_stays_open() {
        let mut input_controller = make_open_help();

        assert_eq!(type_keys(&mut input_controller, "j/G"), None);
        assert!(is_help_open(&input_controller));
    }

    #[test]
    fn given_count_typed_when_the_help_opens_and_closes_then_the_count_is_gone() {
        let mut input_controller = InputController::default();
        type_keys(&mut input_controller, "5");
        input_controller.open_help();
        press(&mut input_controller, KeyCode::Esc);

        assert_eq!(
            type_keys(&mut input_controller, "j"),
            Some(Action::LogView(LogViewAction::MoveCursorDown(1)))
        );
    }
}
