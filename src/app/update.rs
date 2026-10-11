use ratatui::{
    crossterm::event::KeyEvent,
    layout::{Rect, Size},
};

use super::{LogFileIndexingStatus, Model, ScreenAreas, StatusMessage};
use crate::{
    action::{Action, AppAction, CommandLineKind, LogViewAction, SearchAction},
    domain::search::{MatchJump, SearchDirection, SearchError, SearchQuery},
    jobs::{self, JobError, JobNotification, JobSlot, JobStatus},
};

// TODO consider split this file, divided by single responsibility or something

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
        JobStatus::Searching(matching_lines) => {
            model.search_state.add_matching_lines(matching_lines)
        }
        JobStatus::Done => mark_job_done(model, job_slot),
        JobStatus::Failed(job_error) => mark_job_failed(model, job_slot, job_error),
    }
    Effect::Redraw
}

pub fn handle_key(model: &mut Model, key: KeyEvent) -> Effect {
    model.status_message = None;
    match model.input_controller.resolve(key) {
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
            tracing::info!(
                line_count = model.log_file.line_count(),
                file_size = model.log_file.file_size(),
                "file indexed"
            );
            model.log_file_indexing_status = LogFileIndexingStatus::Done;
        }
        JobSlot::Search => {
            tracing::debug!(
                match_count = model.search_state.match_count(),
                "search done"
            );
            jump_to_next_match(model);
        }
    }
}

fn mark_job_failed(model: &mut Model, job_slot: JobSlot, job_error: JobError) {
    match job_slot {
        JobSlot::LogFileIndexing => {
            tracing::error!(error = %job_error, "indexing failed");
            model.log_file_indexing_status = LogFileIndexingStatus::Failed;
        }
        JobSlot::Search => {
            tracing::error!(error = %job_error, "search failed");
            clear_search(model);
            show_error(model, format!("Search failed: {job_error}"));
        }
    }
}

fn dispatch(model: &mut Model, action: Action) -> Effect {
    match action {
        Action::App(app_action) => dispatch_app(model, app_action),
        Action::LogView(log_view_action) => dispatch_log_view(model, log_view_action),
        Action::Search(search_action) => dispatch_search(model, search_action),
    }
}

fn dispatch_log_view(model: &mut Model, log_view_action: LogViewAction) -> Effect {
    let line_count = model.log_file.line_count();
    let log_view_state = &mut model.log_view_state;
    match log_view_action {
        LogViewAction::MoveCursorUp(lines_to_move) => log_view_state.move_cursor_up(lines_to_move),
        LogViewAction::MoveCursorDown(lines_to_move) => {
            log_view_state.move_cursor_down(lines_to_move, line_count)
        }
        LogViewAction::MoveCursorToLineNumber(line_number) => {
            log_view_state.move_cursor_to_line_number(line_number, line_count)
        }
        LogViewAction::MoveCursorToLastLine => log_view_state.move_cursor_to_last_line(line_count),
        LogViewAction::ScrollHalfPageUp => log_view_state.scroll_half_page_up(),
        LogViewAction::ScrollHalfPageDown => log_view_state.scroll_half_page_down(line_count),
        LogViewAction::ScrollPageUp => log_view_state.scroll_page_up(),
        LogViewAction::ScrollPageDown => log_view_state.scroll_page_down(line_count),
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

fn dispatch_search(model: &mut Model, search_action: SearchAction) -> Effect {
    match search_action {
        SearchAction::Start(search_query) => schedule_search(model, search_query),
        SearchAction::RepeatLast(direction) => {
            model.search_state.set_direction(direction);
            jump_to_next_match(model);
        }
        SearchAction::JumpToNextMatch => jump_to_next_match(model),
        SearchAction::JumpToPreviousMatch => jump_to_previous_match(model),
        SearchAction::Clear => clear_search(model),
    }
    Effect::Redraw
}

// review: logic below is about updateing the app but in real it is all about change the model
// maybe it should be some abstraction to make changes in the model
fn schedule_search(model: &mut Model, search_query: SearchQuery) {
    if model.log_file_indexing_status == LogFileIndexingStatus::Running {
        show_error(model, SearchError::StillIndexing.to_string());
        return;
    }

    let search_work = jobs::search_work(model.log_file.line_scanner(), search_query.phrase.clone());
    model.job_board.schedule(JobSlot::Search, search_work);
    model.search_state.replace_query(search_query);
}

fn clear_search(model: &mut Model) {
    model.job_board.cancel(JobSlot::Search);
    model.search_state.clear();
}

fn jump_to_next_match(model: &mut Model) {
    let cursor_line = model.log_view_state.cursor_line();
    let match_jump = model.search_state.next_match_from(cursor_line);
    apply_match_jump(model, match_jump);
}

fn jump_to_previous_match(model: &mut Model) {
    let cursor_line = model.log_view_state.cursor_line();
    let match_jump = model.search_state.previous_match_from(cursor_line);
    apply_match_jump(model, match_jump);
}

fn apply_match_jump(model: &mut Model, match_jump: Result<Option<MatchJump>, SearchError>) {
    match match_jump {
        Ok(Some(match_jump)) => jump_cursor_to_match(model, match_jump),
        Ok(None) => {}
        Err(search_error) => show_error(model, search_error.to_string()),
    }
}

fn jump_cursor_to_match(model: &mut Model, match_jump: MatchJump) {
    let line_count = model.log_file.line_count();
    model
        .log_view_state
        .jump_cursor_to_line(match_jump.line, line_count);
    if match_jump.wrapped {
        show_info(model, wrap_message(match_jump.direction).to_owned());
    }
}

fn wrap_message(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "search hit BOTTOM, continuing at TOP",
        SearchDirection::Backward => "search hit TOP, continuing at BOTTOM",
    }
}

fn show_error(model: &mut Model, error_text: String) {
    model.status_message = Some(StatusMessage::Error(error_text));
}

fn show_info(model: &mut Model, info_text: String) {
    model.status_message = Some(StatusMessage::Info(info_text));
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
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::{control::Mode, domain::log_file::LogFile};

    const SEARCHED_TEXT: &str = "ok\nERROR one\nok\nWARN two\nERROR three\nok\n";

    fn make_model() -> Model {
        make_model_with_text("first\nsecond\nthird\n")
    }

    fn make_model_with_text(text: &str) -> Model {
        let mut model = Model::with_screen_size(LogFile::from_text(text), Size::new(20, 4));
        model.finish_scheduled_jobs();
        model
    }

    fn search(model: &mut Model, typed: &str) {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();
    }

    fn make_indexing_model(text: &str) -> (Model, Vec<JobNotification>) {
        let mut model =
            Model::with_screen_size(LogFile::unindexed_from_text(text), Size::new(20, 4));
        let job_notifications = model.job_board.run_scheduled_jobs();
        (model, job_notifications)
    }

    fn command_line_text(model: &Model) -> Option<&str> {
        match model.input_controller.mode() {
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
    fn given_line_number_when_submitted_then_cursor_goes_there_and_the_command_line_closes() {
        let mut model = make_model();

        let effect = run_command(&mut model, "2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert_eq!(command_line_text(&model), None);
    }

    #[test]
    fn given_number_past_the_end_when_submitted_then_cursor_goes_to_the_last_line() {
        let mut model = make_model();

        run_command(&mut model, "99");

        assert_eq!(model.log_view_state.cursor_line(), 2);
    }

    #[test]
    fn given_cursor_below_first_line_when_zero_is_submitted_then_cursor_goes_to_the_first_line() {
        let mut model = make_model();
        model.type_keys("j");

        run_command(&mut model, "0");

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
        assert_eq!(model.log_view_state.cursor_line(), 1);
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
        assert_eq!(model.log_view_state.cursor_line(), 1);
    }

    #[test]
    fn given_cursor_on_second_line_when_k_is_pressed_then_cursor_moves_up() {
        let mut model = make_model();
        model.type_keys("j");

        model.type_keys("k");

        assert_eq!(model.log_view_state.cursor_line(), 0);
    }

    #[test]
    fn given_app_when_count_digit_is_pressed_then_screen_is_redrawn_without_moving() {
        let mut model = make_model();

        let effect = model.type_keys("2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor_line(), 0);
    }

    #[test]
    fn given_pending_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("j");

        assert_eq!(model.log_view_state.cursor_line(), 2);
    }

    #[test]
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut model = make_model();
        model.type_keys("jj");

        model.type_keys("j");

        assert_eq!(model.log_view_state.cursor_line(), 2);
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

        assert_eq!(model.log_view_state.cursor_line(), 2);
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line_number() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("G");

        assert_eq!(model.log_view_state.cursor_line(), 1);
    }

    #[test]
    fn given_app_when_ctrl_d_is_pressed_then_cursor_moves_half_a_page() {
        let mut model = make_model();

        let effect = model.press_ctrl('d');

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.log_view_state.cursor_line(), 1);
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

        assert_eq!(model.log_view_state.cursor_line(), 2);
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

    #[test]
    fn given_indexed_file_when_searching_forward_then_cursor_jumps_to_the_first_match_below() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "/ERROR");

        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert_eq!(model.search_state.match_count(), 2);
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_cursor_on_the_last_line_when_searching_backward_then_cursor_jumps_to_the_match_above()
    {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");

        search(&mut model, "?ERROR");

        assert_eq!(model.log_view_state.cursor_line(), 4);
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_cursor_above_every_match_when_searching_backward_then_it_wraps_to_the_bottom_with_a_message()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "?ERROR");

        assert_eq!(model.log_view_state.cursor_line(), 4);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info(
                "search hit TOP, continuing at BOTTOM".to_owned()
            ))
        );
    }

    #[test]
    fn given_phrase_not_in_the_file_when_searching_then_cursor_stays_and_no_line_matches() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "/zzzz");

        assert_eq!(model.log_view_state.cursor_line(), 0);
        assert_eq!(model.search_state.match_count(), 0);
        assert!(model.search_state.query().is_some());
    }

    #[test]
    fn given_file_still_indexing_when_searching_then_it_is_refused() {
        let mut model = Model::with_screen_size(
            LogFile::unindexed_from_text(SEARCHED_TEXT),
            Size::new(20, 4),
        );

        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);

        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(SearchError::StillIndexing.to_string()))
        );
        assert_eq!(model.search_state.query(), None);
        assert!(!model.job_board.has_job(JobSlot::Search));
    }

    #[test]
    fn given_scheduled_search_when_another_one_starts_then_only_the_new_one_runs() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);

        search(&mut model, "/WARN");

        assert_eq!(model.log_view_state.cursor_line(), 3);
        assert_eq!(model.search_state.match_count(), 1);
    }

    #[test]
    fn given_running_search_when_it_fails_then_an_error_shows_and_the_search_is_cleared() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);
        let search_job_id = model.job_board.run_scheduled_jobs()[0].job_id;

        let effect = handle_job_notification(
            &mut model,
            JobNotification {
                job_id: search_job_id,
                job_status: JobStatus::Failed(JobError::Panicked),
            },
        );

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                "Search failed: the job panicked".to_owned()
            ))
        );
        assert_eq!(model.search_state.query(), None);
        assert_eq!(model.search_state.match_count(), 0);
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_forward_search_when_n_then_capital_n_is_pressed_then_cursor_goes_to_the_next_then_previous_match()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");

        model.type_keys("n");
        assert_eq!(model.log_view_state.cursor_line(), 4);

        model.type_keys("N");
        assert_eq!(model.log_view_state.cursor_line(), 1);
    }

    #[test]
    fn given_backward_search_when_n_is_pressed_then_it_keeps_going_up() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");
        search(&mut model, "?ERROR");

        model.type_keys("n");

        assert_eq!(model.log_view_state.cursor_line(), 1);
    }

    #[test]
    fn given_cursor_on_the_last_match_when_n_is_pressed_then_it_wraps_to_the_top_with_a_message() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");
        model.type_keys("n");

        model.type_keys("n");

        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info(
                "search hit BOTTOM, continuing at TOP".to_owned()
            ))
        );
    }

    #[test]
    fn given_no_search_yet_when_n_or_capital_n_is_pressed_then_an_error_says_so() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        let no_previous_search = Some(StatusMessage::Error(
            SearchError::NoPreviousSearch.to_string(),
        ));

        model.type_keys("n");
        assert_eq!(model.status_message, no_previous_search);

        model.type_keys("N");
        assert_eq!(model.status_message, no_previous_search);
    }

    #[test]
    fn given_backward_search_when_an_empty_forward_search_is_submitted_then_it_jumps_down_and_n_follows()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("3G");
        search(&mut model, "?ERROR");
        assert_eq!(model.log_view_state.cursor_line(), 1);

        model.type_keys("/");
        model.press(KeyCode::Enter);
        assert_eq!(model.log_view_state.cursor_line(), 4);

        model.type_keys("N");
        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_no_search_yet_when_an_empty_search_is_submitted_then_an_error_says_so() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        model.type_keys("?");
        model.press(KeyCode::Enter);

        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                SearchError::NoPreviousSearch.to_string()
            ))
        );
    }

    #[test]
    fn given_search_still_running_when_n_is_pressed_then_it_uses_the_matches_found_so_far() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);
        let search_job_id = model.job_board.run_scheduled_jobs()[0].job_id;
        handle_job_notification(
            &mut model,
            JobNotification {
                job_id: search_job_id,
                job_status: JobStatus::Searching(vec![1]),
            },
        );

        model.type_keys("n");

        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert!(!model.job_board.is_empty());
    }

    #[test]
    fn given_search_results_when_escape_is_pressed_then_the_query_and_matches_are_gone() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");

        model.press(KeyCode::Esc);

        assert_eq!(model.search_state.query(), None);
        assert_eq!(model.search_state.match_count(), 0);
    }

    #[test]
    fn given_count_typed_after_a_search_when_escape_is_pressed_then_the_search_stays() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");
        model.type_keys("5");

        model.press(KeyCode::Esc);

        assert!(model.search_state.query().is_some());
        assert_eq!(model.search_state.match_count(), 2);
    }

    #[test]
    fn given_search_cleared_with_escape_when_n_is_pressed_then_there_is_no_previous_search() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");
        model.press(KeyCode::Esc);

        model.type_keys("n");

        assert_eq!(model.log_view_state.cursor_line(), 1);
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                SearchError::NoPreviousSearch.to_string()
            ))
        );
    }

    #[test]
    fn given_running_search_when_escape_is_pressed_then_it_is_cancelled_and_its_late_notifications_are_dropped()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);
        let late_job_notifications = model.job_board.run_scheduled_jobs();

        model.press(KeyCode::Esc);
        model.handle_job_notifications(late_job_notifications);

        assert!(model.job_board.is_empty());
        assert_eq!(model.log_view_state.cursor_line(), 0);
        assert_eq!(model.search_state.match_count(), 0);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut model = make_model();

        model.press_ctrl('p');
    }
}
