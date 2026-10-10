mod parser;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction};

use parser::parse_command;

#[derive(Debug, PartialEq)]
pub(super) enum CommandLineOutcome {
    Editing,
    Closed,
    Submitted(Action),
}

#[derive(Default)]
pub struct CommandLine {
    text: String,
    cursor_byte_offset: usize,
}

impl CommandLine {
    pub(super) fn resolve(&mut self, key: KeyEvent) -> CommandLineOutcome {
        match key.code {
            KeyCode::Enter => submission_outcome(&self.text),
            KeyCode::Esc => CommandLineOutcome::Closed,
            KeyCode::Backspace if self.text.is_empty() => CommandLineOutcome::Closed,
            _ => {
                self.edit(key);
                CommandLineOutcome::Editing
            }
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_before_cursor(&self) -> &str {
        &self.text[..self.cursor_byte_offset]
    }

    fn edit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => self.delete_before_cursor(),
            KeyCode::Left => self.cursor_byte_offset = self.previous_char_start(),
            KeyCode::Right => self.cursor_byte_offset = self.next_char_start(),
            KeyCode::Home => self.cursor_byte_offset = 0,
            KeyCode::End => self.cursor_byte_offset = self.text.len(),
            KeyCode::Char(character) if is_text_input(key.modifiers) => self.insert(character),
            _ => {}
        }
    }

    fn insert(&mut self, character: char) {
        self.text.insert(self.cursor_byte_offset, character);
        self.cursor_byte_offset += character.len_utf8();
    }

    fn delete_before_cursor(&mut self) {
        let previous_char_start = self.previous_char_start();
        self.text
            .drain(previous_char_start..self.cursor_byte_offset);
        self.cursor_byte_offset = previous_char_start;
    }

    fn previous_char_start(&self) -> usize {
        self.text
            .floor_char_boundary(self.cursor_byte_offset.saturating_sub(1))
    }

    fn next_char_start(&self) -> usize {
        self.text.ceil_char_boundary(self.cursor_byte_offset + 1)
    }
}

fn submission_outcome(command_text: &str) -> CommandLineOutcome {
    match parse_command(command_text) {
        Ok(Some(action)) => CommandLineOutcome::Submitted(action),
        Ok(None) => CommandLineOutcome::Closed,
        Err(error) => {
            CommandLineOutcome::Submitted(Action::App(AppAction::ShowError(error.to_string())))
        }
    }
}

fn is_text_input(modifiers: KeyModifiers) -> bool {
    !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::LogViewAction;

    fn type_text(command_line: &mut CommandLine, typed: &str) {
        for character in typed.chars() {
            command_line.resolve(KeyCode::Char(character).into());
        }
    }

    fn press(command_line: &mut CommandLine, code: KeyCode) -> CommandLineOutcome {
        command_line.resolve(code.into())
    }

    fn make_command_line(typed: &str) -> CommandLine {
        let mut command_line = CommandLine::default();
        type_text(&mut command_line, typed);
        command_line
    }

    #[test]
    fn given_empty_command_line_when_typing_then_text_is_inserted_and_cursor_follows_it() {
        let command_line = make_command_line("123");

        assert_eq!(command_line.text(), "123");
        assert_eq!(command_line.text_before_cursor(), "123");
    }

    #[test]
    fn given_multibyte_text_when_moving_left_then_typing_inserts_between_whole_characters() {
        let mut command_line = make_command_line("żółw");
        press(&mut command_line, KeyCode::Left);
        press(&mut command_line, KeyCode::Left);

        type_text(&mut command_line, "x");

        assert_eq!(command_line.text(), "żóxłw");
    }

    #[test]
    fn given_multibyte_text_when_backspace_in_the_middle_then_the_whole_character_before_is_deleted()
     {
        let mut command_line = make_command_line("żółw");
        press(&mut command_line, KeyCode::Left);

        press(&mut command_line, KeyCode::Backspace);

        assert_eq!(command_line.text(), "żów");
        assert_eq!(command_line.text_before_cursor(), "żó");
    }

    #[test]
    fn given_cursor_at_the_start_when_moving_left_then_it_stays_there() {
        let mut command_line = make_command_line("ab");
        press(&mut command_line, KeyCode::Home);

        press(&mut command_line, KeyCode::Left);
        type_text(&mut command_line, "x");

        assert_eq!(command_line.text(), "xab");
    }

    #[test]
    fn given_cursor_at_the_end_when_moving_right_then_it_stays_there() {
        let mut command_line = make_command_line("ab");

        press(&mut command_line, KeyCode::Right);
        type_text(&mut command_line, "x");

        assert_eq!(command_line.text(), "abx");
    }

    #[test]
    fn given_cursor_at_the_start_when_end_is_pressed_then_cursor_goes_after_the_text() {
        let mut command_line = make_command_line("ab");
        press(&mut command_line, KeyCode::Home);
        assert_eq!(command_line.text_before_cursor(), "");

        press(&mut command_line, KeyCode::End);

        assert_eq!(command_line.text_before_cursor(), "ab");
    }

    #[test]
    fn given_cursor_at_the_start_of_text_when_backspace_then_nothing_changes_and_editing_goes_on() {
        let mut command_line = make_command_line("ab");
        press(&mut command_line, KeyCode::Home);

        let outcome = press(&mut command_line, KeyCode::Backspace);

        assert_eq!(outcome, CommandLineOutcome::Editing);
        assert_eq!(command_line.text(), "ab");
    }

    #[test]
    fn given_one_character_when_backspace_twice_then_it_is_deleted_and_then_the_command_line_closes()
     {
        let mut command_line = make_command_line("a");

        assert_eq!(
            press(&mut command_line, KeyCode::Backspace),
            CommandLineOutcome::Editing
        );
        assert_eq!(
            press(&mut command_line, KeyCode::Backspace),
            CommandLineOutcome::Closed
        );
    }

    #[test]
    fn given_line_number_when_enter_is_pressed_then_it_is_submitted_as_going_to_that_line() {
        let mut command_line = make_command_line("5000");

        let outcome = press(&mut command_line, KeyCode::Enter);

        assert_eq!(
            outcome,
            CommandLineOutcome::Submitted(Action::LogView(LogViewAction::MoveCursorToLineNumber(
                5000
            )))
        );
    }

    #[test]
    fn given_garbage_when_enter_is_pressed_then_showing_the_error_is_submitted() {
        let mut command_line = make_command_line("abc");

        let outcome = press(&mut command_line, KeyCode::Enter);

        assert_eq!(
            outcome,
            CommandLineOutcome::Submitted(Action::App(AppAction::ShowError(
                "Not a command: abc".to_owned()
            )))
        );
    }

    #[test]
    fn given_blank_text_when_enter_is_pressed_then_the_command_line_closes() {
        let mut command_line = make_command_line("  ");

        assert_eq!(
            press(&mut command_line, KeyCode::Enter),
            CommandLineOutcome::Closed
        );
    }

    #[test]
    fn given_text_when_esc_is_pressed_then_the_command_line_closes() {
        let mut command_line = make_command_line("50");

        assert_eq!(
            press(&mut command_line, KeyCode::Esc),
            CommandLineOutcome::Closed
        );
    }

    #[test]
    fn given_modified_characters_when_typed_then_only_shifted_ones_are_inserted() {
        let mut command_line = CommandLine::default();

        command_line.resolve(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
        command_line.resolve(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        command_line.resolve(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT));

        assert_eq!(command_line.text(), "A");
    }
}
