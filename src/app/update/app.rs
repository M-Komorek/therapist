use super::Effect;
use crate::{
    action::{AppAction, CommandLineKind},
    app::{Model, StatusMessage},
};

pub(super) fn dispatch_app(model: &mut Model, app_action: AppAction) -> Effect {
    match app_action {
        AppAction::Quit => Effect::Quit,
        AppAction::Suspend => Effect::Suspend,
        AppAction::OpenCommandLine(command_line_kind) => {
            open_command_line(model, command_line_kind)
        }
        AppAction::ShowError(error_text) => {
            show_error(model, error_text);
            Effect::Redraw
        }
        #[cfg(debug_assertions)]
        AppAction::DebugPanic => panic!("Debug panic key pressed"),
    }
}

fn open_command_line(model: &mut Model, command_line_kind: CommandLineKind) -> Effect {
    model.input_controller.open_command_line(command_line_kind);
    Effect::Redraw
}

pub(super) fn show_error(model: &mut Model, error_text: String) {
    model.status_message = Some(StatusMessage::Error(error_text));
}

pub(super) fn show_info(model: &mut Model, info_text: String) {
    model.status_message = Some(StatusMessage::Info(info_text));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::update::test_helpers::{command_line_text, make_model, run_command};

    #[test]
    fn given_app_when_colon_is_pressed_then_the_command_line_opens_empty() {
        let mut model = make_model();

        let effect = model.type_keys(":");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some(""));
    }

    #[test]
    fn given_app_when_slash_is_pressed_then_an_empty_search_line_opens() {
        let mut model = make_model();

        let effect = model.type_keys("/");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some(""));
    }

    #[test]
    fn given_open_command_line_when_motion_and_quit_keys_are_typed_then_they_go_to_it() {
        let mut model = make_model();

        let effect = model.type_keys(":jq");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some("jq"));
        assert_eq!(model.log_view_state.cursor_line(), 0);
    }

    #[test]
    fn given_garbage_when_submitted_then_an_error_message_is_set_and_the_command_line_closes() {
        let mut model = make_model();

        run_command(&mut model, "abc");

        assert_eq!(
            model.status_message.as_ref(),
            Some(&StatusMessage::Error("Not a command: abc".to_owned()))
        );
        assert_eq!(command_line_text(&model), None);
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
