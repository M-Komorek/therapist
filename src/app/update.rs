mod app;
mod bookmark;
mod job_notification;
mod log_view;
mod search;
mod tab;

use ratatui::crossterm::event::KeyEvent;

pub use job_notification::handle_job_notification;
pub use log_view::resize_log_view;

use super::Model;
use crate::action::Action;

#[derive(Debug, PartialEq)]
pub enum Effect {
    Nothing,
    Redraw,
    Suspend,
    Quit,
}

pub fn handle_key(model: &mut Model, key: KeyEvent) -> Effect {
    model.status_message = None;
    match model.input_controller.resolve(key) {
        Some(action) => dispatch(model, action),
        None => Effect::Redraw,
    }
}

fn dispatch(model: &mut Model, action: Action) -> Effect {
    match action {
        Action::App(app_action) => app::dispatch_app(model, app_action),
        Action::LogView(log_view_action) => log_view::dispatch_log_view(model, log_view_action),
        Action::Search(search_action) => search::dispatch_search(model, search_action),
        Action::Bookmark(bookmark_action) => bookmark::dispatch_bookmark(model, bookmark_action),
        Action::Tab(tab_action) => tab::dispatch_tab(model, tab_action),
    }
}

#[cfg(test)]
mod test_helpers {
    use ratatui::{
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        layout::Size,
    };

    use super::{Effect, handle_job_notification, handle_key, log_view, resize_log_view};
    use crate::{app::Model, control::Mode, domain::log_file::LogFile, jobs::JobNotification};

    pub(super) const SEARCHED_TEXT: &str = "ok\nERROR one\nok\nWARN two\nERROR three\nok\n";

    impl Model {
        pub fn with_screen_size(log_file: LogFile, screen_size: Size) -> Self {
            let mut model = Self::new(log_file);
            resize_log_view(&mut model, screen_size);
            model
        }

        pub fn handle_job_notifications(&mut self, job_notifications: Vec<JobNotification>) {
            for job_notification in job_notifications {
                handle_job_notification(self, job_notification);
            }
        }

        pub fn finish_scheduled_jobs(&mut self) {
            let job_notifications = self.job_board.run_scheduled_jobs();
            self.handle_job_notifications(job_notifications);
        }

        pub fn type_keys(&mut self, typed: &str) -> Effect {
            let mut effect = Effect::Nothing;

            for character in typed.chars() {
                let modifiers = if character.is_uppercase() {
                    KeyModifiers::SHIFT
                } else {
                    KeyModifiers::NONE
                };
                effect = handle_key(self, KeyEvent::new(KeyCode::Char(character), modifiers));
            }
            effect
        }

        pub fn press_ctrl(&mut self, character: char) -> Effect {
            handle_key(
                self,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL),
            )
        }

        pub fn press(&mut self, code: KeyCode) -> Effect {
            handle_key(self, code.into())
        }

        pub fn cursor_line(&self) -> Option<usize> {
            log_view::cursor_line(self)
        }
    }

    pub(super) fn make_model() -> Model {
        make_model_with_text("first\nsecond\nthird\n")
    }

    pub(super) fn make_model_with_text(text: &str) -> Model {
        let mut model = Model::with_screen_size(LogFile::from_text(text), Size::new(20, 5));
        model.finish_scheduled_jobs();
        model
    }

    pub(super) fn command_line_text(model: &Model) -> Option<&str> {
        match model.input_controller.mode() {
            Mode::CommandLine(command_line) => Some(command_line.text()),
            Mode::Normal => None,
        }
    }

    pub(super) fn run_command(model: &mut Model, command_text: &str) -> Effect {
        model.type_keys(&format!(":{command_text}"));
        model.press(KeyCode::Enter)
    }

    pub(super) fn start_grep(model: &mut Model, phrase: &str) {
        model.type_keys(&format!("&{phrase}"));
        model.press(KeyCode::Enter);
    }

    pub(super) fn grep(model: &mut Model, phrase: &str) {
        start_grep(model, phrase);
        model.finish_scheduled_jobs();
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::{
        Effect,
        test_helpers::{make_model, run_command},
    };

    #[test]
    fn given_error_message_when_esc_is_pressed_then_it_is_cleared_and_redrawn() {
        let mut model = make_model();
        run_command(&mut model, "abc");

        let effect = model.press(KeyCode::Esc);

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.status_message.as_ref(), None);
    }

    #[test]
    fn given_error_message_when_a_motion_key_is_pressed_then_it_is_cleared_and_the_motion_runs() {
        let mut model = make_model();
        run_command(&mut model, "abc");

        model.type_keys("j");

        assert_eq!(model.status_message.as_ref(), None);
        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_unmapped_key_shown_when_it_is_pressed_again_then_the_screen_is_still_redrawn() {
        let mut model = make_model();
        model.type_keys("x");

        let effect = model.type_keys("x");

        assert_eq!(effect, Effect::Redraw);
    }

    #[test]
    fn given_app_when_count_digit_is_pressed_then_screen_is_redrawn_without_moving() {
        let mut model = make_model();

        let effect = model.type_keys("2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.cursor_line(), Some(0));
    }
}
