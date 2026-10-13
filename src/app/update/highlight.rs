use super::{Effect, app::show_error};
use crate::{action::HighlightAction, app::Model, domain::search::SearchError};

pub(super) fn dispatch_highlight(model: &mut Model, highlight_action: HighlightAction) -> Effect {
    match highlight_action {
        HighlightAction::Add(phrase) => add_highlight(model, phrase),
        HighlightAction::AddSearchPhrase => add_search_phrase_highlight(model),
        HighlightAction::Remove(phrase) => remove_highlight(model, &phrase),
        HighlightAction::RemoveAll => model.highlights.remove_all(),
    }
    Effect::Redraw
}

fn add_highlight(model: &mut Model, phrase: String) {
    if let Err(highlight_error) = model.highlights.add(phrase) {
        show_error(model, highlight_error.to_string());
    }
}

fn add_search_phrase_highlight(model: &mut Model) {
    let search_query = model.tabs.active_tab().search_state().query();
    let Some(search_phrase) = search_query.map(|search_query| search_query.phrase.clone()) else {
        show_error(model, SearchError::NoPreviousSearch.to_string());
        return;
    };

    add_highlight(model, search_phrase);
}

fn remove_highlight(model: &mut Model, phrase: &str) {
    if let Err(highlight_error) = model.highlights.remove(phrase) {
        show_error(model, highlight_error.to_string());
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::{
        app::{
            StatusMessage,
            update::test_helpers::{SEARCHED_TEXT, grep, make_model_with_text, run_command},
        },
        domain::highlights::HighlightError,
    };

    fn make_model() -> Model {
        make_model_with_text(SEARCHED_TEXT)
    }

    fn search(model: &mut Model, typed: &str) {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();
    }

    fn highlighted_phrases(model: &Model) -> Vec<&str> {
        model
            .highlights
            .oldest_first()
            .map(|highlight| highlight.phrase.as_str())
            .collect()
    }

    fn error_message(error_text: impl ToString) -> Option<StatusMessage> {
        Some(StatusMessage::Error(error_text.to_string()))
    }

    #[test]
    fn given_no_highlights_when_hl_is_run_with_a_phrase_then_the_phrase_is_highlighted() {
        let mut model = make_model();

        let effect = run_command(&mut model, "hl ERROR");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(highlighted_phrases(&model), ["ERROR"]);
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_highlighted_phrase_when_hl_is_run_with_it_again_then_an_error_says_so() {
        let mut model = make_model();
        run_command(&mut model, "hl ERROR");

        run_command(&mut model, "hl ERROR");

        assert_eq!(highlighted_phrases(&model), ["ERROR"]);
        assert_eq!(
            model.status_message,
            error_message(HighlightError::AlreadyHighlighted("ERROR".to_owned()))
        );
    }

    #[test]
    fn given_search_when_hl_is_run_alone_then_the_search_phrase_is_highlighted() {
        let mut model = make_model();
        search(&mut model, "/WARN");

        run_command(&mut model, "hl");

        assert_eq!(highlighted_phrases(&model), ["WARN"]);
    }

    #[test]
    fn given_no_search_when_hl_is_run_alone_then_an_error_says_so() {
        let mut model = make_model();

        run_command(&mut model, "hl");

        assert!(highlighted_phrases(&model).is_empty());
        assert_eq!(
            model.status_message,
            error_message(SearchError::NoPreviousSearch)
        );
    }

    #[test]
    fn given_searches_in_two_tabs_when_hl_is_run_alone_then_the_active_tabs_search_phrase_is_highlighted()
     {
        let mut model = make_model();
        search(&mut model, "/WARN");
        grep(&mut model, "ERROR");
        search(&mut model, "/three");

        run_command(&mut model, "hl");

        assert_eq!(highlighted_phrases(&model), ["three"]);
    }

    #[test]
    fn given_two_highlights_when_unhl_is_run_with_one_then_only_that_one_is_removed() {
        let mut model = make_model();
        run_command(&mut model, "hl ERROR");
        run_command(&mut model, "hl WARN");

        run_command(&mut model, "unhl ERROR");

        assert_eq!(highlighted_phrases(&model), ["WARN"]);
    }

    #[test]
    fn given_phrase_that_is_not_highlighted_when_unhl_is_run_with_it_then_an_error_says_so() {
        let mut model = make_model();

        run_command(&mut model, "unhl ERROR");

        assert_eq!(
            model.status_message,
            error_message(HighlightError::NotHighlighted("ERROR".to_owned()))
        );
    }

    #[test]
    fn given_two_highlights_when_unhl_is_run_alone_then_both_are_removed() {
        let mut model = make_model();
        run_command(&mut model, "hl ERROR");
        run_command(&mut model, "hl WARN");

        run_command(&mut model, "unhl");

        assert!(highlighted_phrases(&model).is_empty());
        assert_eq!(model.status_message, None);
    }
}
