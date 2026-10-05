use crate::action::{Action, ViewAction};

#[derive(Debug, PartialEq, thiserror::Error)]
pub(super) enum CommandError {
    #[error("Not a command: {0}")]
    NotACommand(String),
}

pub(super) fn parse_command(command_text: &str) -> Result<Option<Action>, CommandError> {
    let command_text = command_text.trim();
    if command_text.is_empty() {
        return Ok(None);
    }
    if is_line_number(command_text) {
        let line_number = saturating_line_number(command_text);
        return Ok(Some(Action::View(ViewAction::GoToLineNumber(line_number))));
    }
    Err(CommandError::NotACommand(command_text.to_owned()))
}

fn is_line_number(command_text: &str) -> bool {
    command_text.bytes().all(|byte| byte.is_ascii_digit())
}

fn saturating_line_number(digits: &str) -> usize {
    digits.parse().unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_to(line_number: usize) -> Result<Option<Action>, CommandError> {
        Ok(Some(Action::View(ViewAction::GoToLineNumber(line_number))))
    }

    #[test]
    fn given_line_number_when_parsing_then_it_goes_to_that_line() {
        assert_eq!(parse_command("5000"), go_to(5000));
        assert_eq!(parse_command("1"), go_to(1));
    }

    #[test]
    fn given_zero_when_parsing_then_it_goes_to_line_zero_for_the_tab_to_clamp() {
        assert_eq!(parse_command("0"), go_to(0));
    }

    #[test]
    fn given_number_too_big_for_usize_when_parsing_then_it_goes_to_the_largest_line_number() {
        assert_eq!(parse_command("99999999999999999999999"), go_to(usize::MAX));
    }

    #[test]
    fn given_surrounding_spaces_when_parsing_then_they_are_ignored() {
        assert_eq!(parse_command("  42 "), go_to(42));
    }

    #[test]
    fn given_empty_or_blank_text_when_parsing_then_there_is_no_command() {
        assert_eq!(parse_command(""), Ok(None));
        assert_eq!(parse_command("   "), Ok(None));
    }

    #[test]
    fn given_text_that_is_not_only_digits_when_parsing_then_it_is_not_a_command() {
        for command_text in ["abc", "+5", "-5", "5x", "1 2"] {
            assert_eq!(
                parse_command(command_text),
                Err(CommandError::NotACommand(command_text.to_owned()))
            );
        }
    }

    #[test]
    fn given_not_a_command_error_when_displayed_then_it_names_the_text() {
        let error = CommandError::NotACommand("abc".to_owned());

        assert_eq!(error.to_string(), "Not a command: abc");
    }
}
