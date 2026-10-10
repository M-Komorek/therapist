use ratatui::{
    crossterm::event::KeyEvent,
    layout::{Rect, Size},
};

use super::{LogFileIndexingStatus, Model, ScreenAreas, StatusMessage};
use crate::{
    action::{Action, AppAction, LogViewAction},
    jobs::{JobError, JobNotification, JobSlot, JobStatus},
};

#[derive(Debug, PartialEq)]
pub enum Effect {
    Nothing,
    Redraw,
    Suspend,
    Quit,
}

pub fn handle_job_notification(model: &mut Model, job_notification: JobNotification) -> Effect {
    let Some((job_slot, job_status)) = model.job_board.resolve(job_notification) else {
        return Effect::Nothing;
    };

    match job_status {
        JobStatus::LogFileIndexing(line_index) => model.log_file.set_line_index(line_index),
        JobStatus::Done => mark_job_done(model, job_slot),
        JobStatus::Failed(job_error) => mark_job_failed(model, job_slot, job_error),
    }
    Effect::Redraw
}

pub fn handle_key(model: &mut Model, key: KeyEvent) -> Effect {
    model.status_message = None;
    match model.control.resolve(key) {
        Some(action) => dispatch(model, action),
        None => Effect::Redraw,
    }
}

pub fn resize_log_view(model: &mut Model, screen_size: Size) -> Effect {
    let log_view_area = ScreenAreas::new(Rect::from(screen_size)).log_view;
    let log_view_state = &mut model.log_view_state;
    log_view_state.set_height(usize::from(log_view_area.height));
    log_view_state.set_width(usize::from(log_view_area.width));
    Effect::Redraw
}

fn mark_job_done(model: &mut Model, job_slot: JobSlot) {
    match job_slot {
        JobSlot::LogFileIndexing => {
            tracing::info!(line_count = model.log_file.line_count(), "file indexed");
            model.log_file_indexing_status = LogFileIndexingStatus::Done;
        }
    }
}

fn mark_job_failed(model: &mut Model, job_slot: JobSlot, job_error: JobError) {
    match job_slot {
        JobSlot::LogFileIndexing => {
            tracing::error!(error = %job_error, "indexing failed");
            model.log_file_indexing_status = LogFileIndexingStatus::Failed;
        }
    }
}

fn dispatch(model: &mut Model, action: Action) -> Effect {
    match action {
        Action::App(app_action) => dispatch_app(model, app_action),
        Action::LogView(log_view_action) => dispatch_log_view(model, log_view_action),
    }
}

fn dispatch_log_view(model: &mut Model, log_view_action: LogViewAction) -> Effect {
    let line_count = model.log_file.line_count();
    let log_view_state = &mut model.log_view_state;
    match log_view_action {
        LogViewAction::MoveUp(lines_to_move) => log_view_state.move_up(lines_to_move),
        LogViewAction::MoveDown(lines_to_move) => {
            log_view_state.move_down(lines_to_move, line_count)
        }
        LogViewAction::GoToLineNumber(line_number) => {
            log_view_state.go_to_line_number(line_number, line_count)
        }
        LogViewAction::GoToLastLine => log_view_state.go_to_last_line(line_count),
        LogViewAction::HalfPageUp => log_view_state.half_page_up(),
        LogViewAction::HalfPageDown => log_view_state.half_page_down(line_count),
        LogViewAction::PageUp => log_view_state.page_up(),
        LogViewAction::PageDown => log_view_state.page_down(line_count),
        LogViewAction::ScrollLeft(columns_to_scroll) => {
            log_view_state.scroll_left(columns_to_scroll)
        }
        LogViewAction::ScrollRight(columns_to_scroll) => {
            log_view_state.scroll_right(columns_to_scroll)
        }
        LogViewAction::ScrollLeftWide => log_view_state.scroll_left_wide(),
        LogViewAction::ScrollRightWide => log_view_state.scroll_right_wide(),
        LogViewAction::ScrollToFirstColumn => log_view_state.scroll_to_first_column(),
    }
    Effect::Redraw
}

fn dispatch_app(model: &mut Model, app_action: AppAction) -> Effect {
    match app_action {
        AppAction::Quit => Effect::Quit,
        AppAction::Suspend => Effect::Suspend,
        AppAction::OpenCommandLine => open_command_line(model),
        AppAction::ShowError(error_text) => show_error(model, error_text),
        #[cfg(debug_assertions)]
        AppAction::DebugPanic => panic!("Debug panic key pressed"),
    }
}

fn open_command_line(model: &mut Model) -> Effect {
    model.control.open_command_line();
    Effect::Redraw
}

fn show_error(model: &mut Model, error_text: String) -> Effect {
    model.status_message = Some(StatusMessage::Error(error_text));
    Effect::Redraw
}

#[cfg(test)]
mod test_helpers {
    use ratatui::{
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        layout::Size,
    };

    use super::{Effect, handle_job_notification, handle_key, resize_log_view};
    use crate::{app::Model, domain::log_file::LogFile, jobs::JobNotification};

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
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::{control::Mode, domain::log_file::LogFile};

    fn make_model() -> Model {
        let mut model = Model::with_screen_size(
            LogFile::from_text("first\nsecond\nthird\n"),
            Size::new(20, 4),
        );
        let job_notifications = model.job_board.run_scheduled_jobs();
        model.handle_job_notifications(job_notifications);
        model
    }

    fn make_indexing_model(text: &str) -> (Model, Vec<JobNotification>) {
        let mut model =
            Model::with_screen_size(LogFile::unindexed_from_text(text), Size::new(20, 4));
        let job_notifications = model.job_board.run_scheduled_jobs();
        (model, job_notifications)
    }

    fn command_line_text(model: &Model) -> Option<&str> {
        match model.control.mode() {
            Mode::CommandLine(command_line) => Some(command_line.text()),
            Mode::Normal => None,
        }
    }

    fn run_command(model: &mut Model, command_text: &str) -> Effect {
        model.type_keys(&format!(":{command_text}"));
        model.press(KeyCode::Enter)
    }

    #[test]
    fn given_app_when_colon_is_pressed_then_the_command_line_opens_empty() {
        let mut model = make_model();

        let effect = model.type_keys(":");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some(""));
    }

    #[test]
    fn given_open_command_line_when_motion_and_quit_keys_are_typed_then_they_go_to_it() {
        let mut model = make_model();

        let effect = model.type_keys(":jq");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(command_line_text(&model), Some("jq"));
        assert_eq!(model.log_view_state.cursor(), 0);
    }

    #[test]
    fn given_line_number_when_submitted_then_cursor_goes_there_and_the_command_line_closes() {
        let mut model = make_model();

        let effect = run_command(&mut model, "2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor(), 1);
        assert_eq!(command_line_text(&model), None);
    }

    #[test]
    fn given_number_past_the_end_when_submitted_then_cursor_goes_to_the_last_line() {
        let mut model = make_model();

        run_command(&mut model, "99");

        assert_eq!(model.log_view_state.cursor(), 2);
    }

    #[test]
    fn given_cursor_below_first_line_when_zero_is_submitted_then_cursor_goes_to_the_first_line() {
        let mut model = make_model();
        model.type_keys("j");

        run_command(&mut model, "0");

        assert_eq!(model.log_view_state.cursor(), 0);
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
        assert_eq!(model.log_view_state.cursor(), 1);
    }

    #[test]
    fn given_open_command_line_when_terminal_is_resized_then_it_stays_open_with_its_text() {
        let mut model = make_model();
        model.type_keys(":5");

        resize_log_view(&mut model, Size::new(40, 10));

        assert_eq!(command_line_text(&model), Some("5"));
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

    #[test]
    fn given_unmapped_key_shown_when_it_is_pressed_again_then_the_screen_is_still_redrawn() {
        let mut model = make_model();
        model.type_keys("x");

        let effect = model.type_keys("x");

        assert_eq!(effect, Effect::Redraw);
    }

    #[test]
    fn given_app_when_j_is_pressed_then_cursor_moves_down_and_screen_is_redrawn() {
        let mut model = make_model();

        let effect = model.type_keys("j");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor(), 1);
    }

    #[test]
    fn given_cursor_on_second_line_when_k_is_pressed_then_cursor_moves_up() {
        let mut model = make_model();
        model.type_keys("j");

        model.type_keys("k");

        assert_eq!(model.log_view_state.cursor(), 0);
    }

    #[test]
    fn given_app_when_count_digit_is_pressed_then_screen_is_redrawn_without_moving() {
        let mut model = make_model();

        let effect = model.type_keys("2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor(), 0);
    }

    #[test]
    fn given_pending_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("j");

        assert_eq!(model.log_view_state.cursor(), 2);
    }

    #[test]
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut model = make_model();
        model.type_keys("jj");

        model.type_keys("j");

        assert_eq!(model.log_view_state.cursor(), 2);
    }

    #[test]
    fn given_new_app_when_j_is_pressed_then_the_view_does_not_scroll() {
        let mut model = make_model();

        model.type_keys("j");

        assert_eq!(model.log_view_state.top_line(), 0);
    }

    #[test]
    fn given_app_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        let mut model = make_model();

        model.type_keys("G");

        assert_eq!(model.log_view_state.cursor(), 2);
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line_number() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("G");

        assert_eq!(model.log_view_state.cursor(), 1);
    }

    #[test]
    fn given_app_when_ctrl_d_is_pressed_then_cursor_moves_half_a_page() {
        let mut model = make_model();

        let effect = model.press_ctrl('d');

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor(), 1);
    }

    #[test]
    fn given_count_when_l_then_h_are_pressed_then_view_scrolls_right_then_left() {
        let mut model = make_model();
        model.type_keys("3");
        model.type_keys("l");

        model.type_keys("h");

        assert_eq!(model.log_view_state.left_column(), 2);
    }

    #[test]
    fn given_terminal_resized_to_100_columns_when_capital_l_is_pressed_then_view_scrolls_20_columns()
     {
        let mut model = make_model();
        resize_log_view(&mut model, Size::new(100, 10));

        model.type_keys("L");

        assert_eq!(model.log_view_state.left_column(), 20);
    }

    #[test]
    fn given_app_when_terminal_is_resized_then_screen_is_redrawn() {
        let mut model = make_model();

        let effect = resize_log_view(&mut model, Size::new(30, 10));

        assert_eq!(effect, Effect::Redraw);
    }

    #[test]
    fn given_cursor_on_last_line_when_window_shrinks_then_view_scrolls_to_keep_it_visible() {
        let mut model = make_model();
        model.type_keys("jj");

        resize_log_view(&mut model, Size::new(20, 2));

        assert_eq!(model.log_view_state.top_line(), 2);
    }

    #[test]
    fn given_unindexed_file_when_app_starts_then_indexing_runs() {
        let model = Model::new(LogFile::unindexed_from_text("a\nb\n"));

        assert_eq!(
            model.log_file_indexing_status,
            LogFileIndexingStatus::Running
        );
        assert!(!model.job_board.is_empty());
    }

    #[test]
    fn given_indexing_app_when_progress_arrives_then_the_screen_is_redrawn() {
        let (mut model, mut job_notifications) = make_indexing_model("a\nb\n");

        let effect = handle_job_notification(&mut model, job_notifications.remove(0));

        assert_eq!(effect, Effect::Redraw);
    }

    #[test]
    fn given_progress_delivered_when_capital_g_is_pressed_then_cursor_goes_to_last_indexed_line() {
        let (mut model, mut job_notifications) = make_indexing_model("first\nsecond\nthird\n");
        let _done_notification = job_notifications.pop();
        model.handle_job_notifications(job_notifications);

        model.type_keys("G");

        assert_eq!(model.log_view_state.cursor(), 2);
    }

    #[test]
    fn given_indexing_app_when_done_arrives_then_indexing_is_done_and_no_job_is_left() {
        let (mut model, job_notifications) = make_indexing_model("a\nb\n");

        model.handle_job_notifications(job_notifications);

        assert_eq!(model.log_file_indexing_status, LogFileIndexingStatus::Done);
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_indexing_app_when_indexing_fails_then_it_is_marked_failed_and_indexed_lines_stay() {
        let (mut model, mut job_notifications) = make_indexing_model("a\nb\n");
        let done_notification = job_notifications.pop().unwrap();
        model.handle_job_notifications(job_notifications);

        handle_job_notification(
            &mut model,
            JobNotification {
                job_id: done_notification.job_id,
                job_status: JobStatus::Failed(JobError::Panicked),
            },
        );

        assert_eq!(
            model.log_file_indexing_status,
            LogFileIndexingStatus::Failed
        );
        assert_eq!(model.log_file.line_count(), 2);
    }

    #[test]
    fn given_finished_indexing_when_a_late_notification_arrives_then_it_is_ignored_without_a_redraw()
     {
        let (mut model, job_notifications) = make_indexing_model("a\nb\n");
        let job_id = job_notifications[0].job_id;
        model.handle_job_notifications(job_notifications);

        let effect = handle_job_notification(
            &mut model,
            JobNotification {
                job_id,
                job_status: JobStatus::Failed(JobError::Panicked),
            },
        );

        assert_eq!(effect, Effect::Nothing);
        assert_eq!(model.log_file_indexing_status, LogFileIndexingStatus::Done);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut model = make_model();

        model.press_ctrl('p');
    }
}
