use crate::action::{Action, AppAction, HighlightAction, LogViewAction};

#[derive(Debug, PartialEq, thiserror::Error)]
pub(super) enum CommandError {
    #[error("Not a command: {0}")]
    NotACommand(String),
}

pub const COMMAND_DESCRIPTIONS: &[(&str, &str)] = &[
    (":{line}", "Go to line {line}"),
    (":hl [phrase]", "Highlight a phrase, or the search phrase"),
    (":unhl [phrase]", "Remove a highlight, or all of them"),
    (":help :h", "Show this help"),
];

pub(super) fn parse_command(command_text: &str) -> Result<Option<Action>, CommandError> {
    let command_text = command_text.trim();
    if command_text.is_empty() {
        return Ok(None);
    }
    if is_line_number(command_text) {
        let line_number = saturating_line_number(command_text);
        return Ok(Some(Action::LogView(
            LogViewAction::MoveCursorToLineNumber(line_number),
        )));
    }
    if matches!(command_text, "help" | "h") {
        return Ok(Some(Action::App(AppAction::OpenHelp)));
    }
    if let Some(highlight_action) = parse_highlight_command(command_text) {
        return Ok(Some(Action::Highlight(highlight_action)));
    }
    Err(CommandError::NotACommand(command_text.to_owned()))
}

fn parse_highlight_command(command_text: &str) -> Option<HighlightAction> {
    let (command_name, phrase) = command_text.split_once(' ').unwrap_or((command_text, ""));
    let phrase = phrase.trim_start();

    match command_name {
        "hl" if phrase.is_empty() => Some(HighlightAction::AddSearchPhrase),
        "hl" => Some(HighlightAction::Add(phrase.to_owned())),
        "unhl" if phrase.is_empty() => Some(HighlightAction::RemoveAll),
        "unhl" => Some(HighlightAction::Remove(phrase.to_owned())),
        _ => None,
    }
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

    fn move_cursor_to(line_number: usize) -> Result<Option<Action>, CommandError> {
        Ok(Some(Action::LogView(
            LogViewAction::MoveCursorToLineNumber(line_number),
        )))
    }

    #[test]
    fn given_line_number_when_parsing_then_it_goes_to_that_line() {
        assert_eq!(parse_command("5000"), move_cursor_to(5000));
        assert_eq!(parse_command("1"), move_cursor_to(1));
    }

    #[test]
    fn given_zero_when_parsing_then_it_goes_to_line_zero_for_the_tab_to_clamp() {
        assert_eq!(parse_command("0"), move_cursor_to(0));
    }

    #[test]
    fn given_number_too_big_for_usize_when_parsing_then_it_goes_to_the_largest_line_number() {
        assert_eq!(
            parse_command("99999999999999999999999"),
            move_cursor_to(usize::MAX)
        );
    }

    #[test]
    fn given_surrounding_spaces_when_parsing_then_they_are_ignored() {
        assert_eq!(parse_command("  42 "), move_cursor_to(42));
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

    fn highlight(highlight_action: HighlightAction) -> Result<Option<Action>, CommandError> {
        Ok(Some(Action::Highlight(highlight_action)))
    }

    #[test]
    fn given_hl_with_a_phrase_when_parsing_then_it_adds_that_highlight() {
        assert_eq!(
            parse_command("hl ERROR"),
            highlight(HighlightAction::Add("ERROR".to_owned()))
        );
    }

    #[test]
    fn given_hl_with_spaces_in_the_phrase_when_parsing_then_the_inner_spaces_are_kept() {
        assert_eq!(
            parse_command("hl took=1 ms"),
            highlight(HighlightAction::Add("took=1 ms".to_owned()))
        );
        assert_eq!(
            parse_command("hl   ERROR  "),
            highlight(HighlightAction::Add("ERROR".to_owned()))
        );
    }

    #[test]
    fn given_hl_alone_when_parsing_then_it_highlights_the_search_phrase() {
        assert_eq!(
            parse_command("hl"),
            highlight(HighlightAction::AddSearchPhrase)
        );
        assert_eq!(
            parse_command(" hl  "),
            highlight(HighlightAction::AddSearchPhrase)
        );
    }

    #[test]
    fn given_unhl_with_a_phrase_when_parsing_then_it_removes_that_highlight() {
        assert_eq!(
            parse_command("unhl user=42"),
            highlight(HighlightAction::Remove("user=42".to_owned()))
        );
    }

    #[test]
    fn given_unhl_alone_when_parsing_then_it_removes_every_highlight() {
        assert_eq!(parse_command("unhl"), highlight(HighlightAction::RemoveAll));
    }

    #[test]
    fn given_text_that_only_starts_like_a_highlight_command_when_parsing_then_it_is_not_a_command()
    {
        for command_text in ["hlERROR", "unhlx", "highlight ERROR", "HL ERROR"] {
            assert_eq!(
                parse_command(command_text),
                Err(CommandError::NotACommand(command_text.to_owned()))
            );
        }
    }

    #[test]
    fn given_help_or_h_when_parsing_then_the_help_opens() {
        let open_help = Ok(Some(Action::App(AppAction::OpenHelp)));

        assert_eq!(parse_command("help"), open_help);
        assert_eq!(parse_command(" h "), open_help);
    }

    #[test]
    fn given_help_with_a_topic_when_parsing_then_it_is_not_a_command() {
        for command_text in ["help j", "h hl", "helpme"] {
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
