use super::{Effect, app::show_info};
use crate::{action::HighlightAction, app::Model};

pub(super) fn dispatch_highlight(model: &mut Model, highlight_action: HighlightAction) -> Effect {
    match highlight_action {
        HighlightAction::Toggle(phrase) => toggle_highlight(model, phrase),
        HighlightAction::RemoveAll => model.highlights.remove_all(),
    }
    Effect::Redraw
}

fn toggle_highlight(model: &mut Model, phrase: String) {
    let info_text = if model.highlights.contains(&phrase) {
        format!("Highlight removed: {phrase}")
    } else {
        format!("Highlighted: {phrase}")
    };

    model.highlights.toggle(phrase);
    show_info(model, info_text);
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::app::{
        StatusMessage,
        update::test_helpers::{SEARCHED_TEXT, make_model_with_text},
    };

    fn make_model() -> Model {
        make_model_with_text(SEARCHED_TEXT)
    }

    fn submit_highlight_line(model: &mut Model, phrase: &str) -> Effect {
        model.type_keys(&format!(" h{phrase}"));
        model.press(KeyCode::Enter)
    }

    fn highlighted_phrases(model: &Model) -> Vec<&str> {
        model
            .highlights
            .oldest_first()
            .map(|highlight| highlight.phrase.as_str())
            .collect()
    }

    #[test]
    fn given_no_highlights_when_a_phrase_is_toggled_then_it_is_highlighted() {
        let mut model = make_model();

        let effect = submit_highlight_line(&mut model, "ERROR");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(highlighted_phrases(&model), ["ERROR"]);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info("Highlighted: ERROR".to_owned()))
        );
    }

    #[test]
    fn given_two_highlights_when_one_is_toggled_again_then_only_that_one_is_removed() {
        let mut model = make_model();
        submit_highlight_line(&mut model, "ERROR");
        submit_highlight_line(&mut model, "WARN");

        submit_highlight_line(&mut model, "ERROR");

        assert_eq!(highlighted_phrases(&model), ["WARN"]);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info("Highlight removed: ERROR".to_owned()))
        );
    }

    #[test]
    fn given_highlight_when_nothing_is_typed_on_the_highlight_line_then_it_is_kept() {
        let mut model = make_model();
        submit_highlight_line(&mut model, "ERROR");

        submit_highlight_line(&mut model, "");

        assert_eq!(highlighted_phrases(&model), ["ERROR"]);
    }

    #[test]
    fn given_two_highlights_when_space_capital_h_is_pressed_then_both_are_removed() {
        let mut model = make_model();
        submit_highlight_line(&mut model, "ERROR");
        submit_highlight_line(&mut model, "WARN");

        model.type_keys(" H");

        assert!(highlighted_phrases(&model).is_empty());
    }
}
