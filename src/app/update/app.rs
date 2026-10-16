use super::Effect;
use crate::{
    action::AppAction,
    app::{Model, StatusMessage},
};

pub(super) fn dispatch_app(model: &mut Model, app_action: AppAction) -> Effect {
    match app_action {
        AppAction::Quit => Effect::Quit,
        AppAction::Suspend => Effect::Suspend,
        AppAction::OpenCommandLine(command_line_kind) => {
            model.input_controller.open_command_line(command_line_kind);
            Effect::Redraw
        }
        AppAction::OpenHelp => {
            model.input_controller.open_help();
            Effect::Redraw
        }
        #[cfg(debug_assertions)]
        AppAction::DebugPanic => panic!("Debug panic key pressed"),
    }
}

pub(super) fn show_error(model: &mut Model, error_text: String) {
    model.status_message = Some(StatusMessage::Error(error_text));
}

pub(super) fn show_info(model: &mut Model, info_text: String) {
    model.status_message = Some(StatusMessage::Info(info_text));
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::{
        app::update::test_helpers::{command_line_text, make_model},
        control::Mode,
    };

    fn is_help_open(model: &Model) -> bool {
        matches!(model.input_controller.mode(), Mode::Help)
    }

    #[test]
    fn given_app_when_slash_is_pressed_then_an_empty_search_line_opens() {
        let mut model = make_model();

        let effect = model.type_keys("/");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some(""));
    }

    #[test]
    fn given_grep_tab_when_a_search_grep_or_highlight_line_is_opened_then_it_opens() {
        let mut model = make_model();
        model.type_keys(" gfirst");
        model.press(KeyCode::Enter);

        for opening_keys in ["/", "?", " g", " h"] {
            model.type_keys(opening_keys);

            assert_eq!(command_line_text(&model), Some(""));
            model.press(KeyCode::Esc);
        }
    }

    #[test]
    fn given_open_command_line_when_motion_and_quit_keys_are_typed_then_they_go_to_it() {
        let mut model = make_model();

        let effect = model.type_keys("/jq");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some("jq"));
        assert_eq!(model.cursor_line(), Some(0));
    }

    #[test]
    fn given_app_when_f1_is_pressed_then_the_help_opens() {
        let mut model = make_model();

        let effect = model.press(KeyCode::F(1));

        assert_eq!(effect, Effect::Redraw);
        assert!(is_help_open(&model));
    }

    #[test]
    fn given_open_help_when_j_and_q_are_typed_then_it_stays_open_without_moving_or_quitting() {
        let mut model = make_model();
        model.press(KeyCode::F(1));

        let effect = model.type_keys("jq");

        assert_ne!(effect, Effect::Quit);
        assert!(is_help_open(&model));
        assert_eq!(model.cursor_line(), Some(0));
    }

    #[test]
    fn given_open_help_when_esc_is_pressed_then_it_closes() {
        let mut model = make_model();
        model.press(KeyCode::F(1));

        let effect = model.press(KeyCode::Esc);

        assert_eq!(effect, Effect::Redraw);
        assert!(!is_help_open(&model));
    }

    #[test]
    fn given_app_when_q_is_pressed_then_it_quits() {
        let mut model = make_model();

        let effect = model.type_keys("q");

        assert_eq!(effect, Effect::Quit);
    }

    #[test]
    fn given_app_when_ctrl_z_is_pressed_then_it_suspends() {
        let mut model = make_model();

        let effect = model.press_ctrl('z');

        assert_eq!(effect, Effect::Suspend);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut model = make_model();

        model.press_ctrl('p');
    }
}
