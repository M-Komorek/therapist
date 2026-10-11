mod command_line;
mod keymap;

use ratatui::crossterm::event::KeyEvent;

use crate::action::{Action, CommandLineKind};

pub use command_line::CommandLine;
use command_line::CommandLineOutcome;
use keymap::Keymap;

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    CommandLine(CommandLine),
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
        }
    }

    pub fn open_command_line(&mut self, command_line_kind: CommandLineKind) {
        self.keymap.reset();
        self.mode = Mode::CommandLine(CommandLine::new(command_line_kind));
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
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::{action::LogViewAction, domain::search::SearchDirection};

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
        input_controller.open_command_line(CommandLineKind::Command);
        type_keys(&mut input_controller, typed);
        input_controller
    }

    fn command_line_text(input_controller: &InputController) -> Option<&str> {
        match input_controller.mode() {
            Mode::CommandLine(command_line) => Some(command_line.text()),
            Mode::Normal => None,
        }
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

        input_controller.open_command_line(CommandLineKind::Command);

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
    fn given_line_number_on_the_command_line_when_enter_is_pressed_then_its_action_comes_out_and_it_closes()
     {
        let mut input_controller = make_open_command_line("2");

        let resolution = press(&mut input_controller, KeyCode::Enter);

        assert_eq!(
            resolution,
            Some(Action::LogView(LogViewAction::MoveCursorToLineNumber(2)))
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
        type_keys(&mut input_controller, "2:");
        input_controller.open_command_line(CommandLineKind::Command);
        press(&mut input_controller, KeyCode::Esc);

        assert_eq!(input_controller.last_keys_text(), "");
        assert_eq!(
            type_keys(&mut input_controller, "j"),
            Some(Action::LogView(LogViewAction::MoveCursorDown(1)))
        );
    }
}
