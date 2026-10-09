mod command_line;
mod keymap;

use ratatui::crossterm::event::KeyEvent;

use crate::action::Action;

pub use command_line::CommandLine;
use command_line::CommandLineOutcome;
use keymap::Keymap;

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    CommandLine(CommandLine),
}

/// Sends each key press to the keymap in normal mode, or to the command line while it is open.
#[derive(Default)]
pub struct Control {
    keymap: Keymap,
    mode: Mode,
}

impl Control {
    pub fn resolve(&mut self, key: KeyEvent) -> Option<Action> {
        match &mut self.mode {
            Mode::Normal => self.keymap.resolve(key),
            Mode::CommandLine(command_line) => {
                let outcome = command_line.handle_key(key);
                self.handle_command_line_outcome(outcome)
            }
        }
    }

    pub fn open_command_line(&mut self) {
        self.keymap.reset();
        self.mode = Mode::CommandLine(CommandLine::default());
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn last_keys_text(&self) -> String {
        self.keymap.last_keys_text()
    }

    fn handle_command_line_outcome(&mut self, outcome: CommandLineOutcome) -> Option<Action> {
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
    use crate::action::LogViewAction;

    fn type_keys(control: &mut Control, typed: &str) -> Option<Action> {
        let mut resolution = None;
        for character in typed.chars() {
            resolution = control.resolve(KeyCode::Char(character).into());
        }
        resolution
    }

    fn press(control: &mut Control, code: KeyCode) -> Option<Action> {
        control.resolve(code.into())
    }

    fn make_open_command_line(typed: &str) -> Control {
        let mut control = Control::default();
        control.open_command_line();
        type_keys(&mut control, typed);
        control
    }

    fn command_line_text(control: &Control) -> Option<&str> {
        match control.mode() {
            Mode::CommandLine(command_line) => Some(command_line.text()),
            Mode::Normal => None,
        }
    }

    #[test]
    fn given_normal_mode_when_a_bound_key_is_pressed_then_the_keymap_resolves_it() {
        let mut control = Control::default();

        assert_eq!(
            type_keys(&mut control, "j"),
            Some(Action::LogView(LogViewAction::MoveDown(1)))
        );
    }

    #[test]
    fn given_normal_mode_when_the_command_line_opens_then_it_is_empty() {
        let mut control = Control::default();

        control.open_command_line();

        assert_eq!(command_line_text(&control), Some(""));
    }

    #[test]
    fn given_open_command_line_when_keys_are_typed_then_they_edit_it_instead_of_the_keymap() {
        let mut control = make_open_command_line("");

        let resolution = type_keys(&mut control, "jq");

        assert_eq!(resolution, None);
        assert_eq!(command_line_text(&control), Some("jq"));
    }

    #[test]
    fn given_line_number_on_the_command_line_when_enter_is_pressed_then_its_action_comes_out_and_it_closes()
     {
        let mut control = make_open_command_line("2");

        let resolution = press(&mut control, KeyCode::Enter);

        assert_eq!(
            resolution,
            Some(Action::LogView(LogViewAction::GoToLineNumber(2)))
        );
        assert_eq!(command_line_text(&control), None);
    }

    #[test]
    fn given_text_on_the_command_line_when_esc_is_pressed_then_it_closes_without_an_action() {
        let mut control = make_open_command_line("2");

        assert_eq!(press(&mut control, KeyCode::Esc), None);
        assert_eq!(command_line_text(&control), None);
    }

    #[test]
    fn given_count_typed_when_the_command_line_opens_and_closes_then_the_count_and_last_keys_are_gone()
     {
        let mut control = Control::default();
        type_keys(&mut control, "2:");
        control.open_command_line();
        press(&mut control, KeyCode::Esc);

        assert_eq!(control.last_keys_text(), "");
        assert_eq!(
            type_keys(&mut control, "j"),
            Some(Action::LogView(LogViewAction::MoveDown(1)))
        );
    }
}
