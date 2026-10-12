use super::{
    Effect,
    app::{show_error, show_info},
    log_view::{cursor_line, jump_cursor_to_line},
};
use crate::{
    action::SearchAction,
    app::{LogFileIndexingStatus, Model},
    domain::{
        search::{MatchJump, SearchDirection, SearchError, SearchQuery},
        tabs::TabError,
    },
    jobs::{self, JobSlot},
};

pub(super) fn dispatch_search(model: &mut Model, search_action: SearchAction) -> Effect {
    match search_action {
        SearchAction::Start(search_query) => schedule_search(model, search_query),
        SearchAction::RepeatLast(direction) => {
            model.search_state.set_direction(direction);
            jump_to_next_match(model);
        }
        SearchAction::JumpToNextMatch | SearchAction::JumpToPreviousMatch
            if !model.tabs.active_tab().shows_all_lines() =>
        {
            show_error(model, TabError::NotAvailableInGrepTabs.to_string())
        }
        SearchAction::JumpToNextMatch => jump_to_next_match(model),
        SearchAction::JumpToPreviousMatch => jump_to_previous_match(model),
        SearchAction::Clear => clear_search(model),
    }
    Effect::Redraw
}

fn schedule_search(model: &mut Model, search_query: SearchQuery) {
    if model.log_file_indexing_status == LogFileIndexingStatus::Running {
        show_error(model, SearchError::StillIndexing.to_string());
        return;
    }

    let log_file = &model.log_file;
    let searching_work = jobs::searching_work(
        log_file.line_scanner(),
        log_file.file_size(),
        search_query.phrase.clone(),
    );
    model.job_board.schedule(JobSlot::Searching, searching_work);
    model.search_state.replace_query(search_query);
}

pub(super) fn clear_search(model: &mut Model) {
    model.job_board.cancel(JobSlot::Searching);
    model.search_state.clear();
}

pub(super) fn jump_to_next_match(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let match_jump = model.search_state.next_match_from(cursor_line);
    apply_match_jump(model, match_jump);
}

fn jump_to_previous_match(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
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
    jump_cursor_to_line(model, match_jump.line);
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

#[cfg(test)]
mod tests {
    use ratatui::{crossterm::event::KeyCode, layout::Size};

    use super::*;
    use crate::{
        app::{
            StatusMessage,
            update::{
                handle_job_notification,
                test_helpers::{SEARCHED_TEXT, make_model_with_text},
            },
        },
        domain::{log_file::LogFile, phrase_matching::MatchingLinesBatch},
        jobs::{JobNotification, JobStatus},
    };

    fn search(model: &mut Model, typed: &str) {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();
    }

    #[test]
    fn given_indexed_file_when_searching_forward_then_cursor_jumps_to_the_first_match_below() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "/ERROR");

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(model.search_state.match_count(), 2);
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_cursor_on_the_last_line_when_searching_backward_then_cursor_jumps_to_the_match_above()
    {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");

        search(&mut model, "?ERROR");

        assert_eq!(model.cursor_line(), Some(4));
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_cursor_above_every_match_when_searching_backward_then_it_wraps_to_the_bottom_with_a_message()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "?ERROR");

        assert_eq!(model.cursor_line(), Some(4));
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

        assert_eq!(model.cursor_line(), Some(0));
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
        assert!(!model.job_board.has_job(JobSlot::Searching));
    }

    #[test]
    fn given_scheduled_search_when_another_one_starts_then_only_the_new_one_runs() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);

        search(&mut model, "/WARN");

        assert_eq!(model.cursor_line(), Some(3));
        assert_eq!(model.search_state.match_count(), 1);
    }

    #[test]
    fn given_forward_search_when_n_then_capital_n_is_pressed_then_cursor_goes_to_the_next_then_previous_match()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");

        model.type_keys("n");
        assert_eq!(model.cursor_line(), Some(4));

        model.type_keys("N");
        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_backward_search_when_n_is_pressed_then_it_keeps_going_up() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");
        search(&mut model, "?ERROR");

        model.type_keys("n");

        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_cursor_on_the_last_match_when_n_is_pressed_then_it_wraps_to_the_top_with_a_message() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");
        model.type_keys("n");

        model.type_keys("n");

        assert_eq!(model.cursor_line(), Some(1));
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
        assert_eq!(model.cursor_line(), Some(1));

        model.type_keys("/");
        model.press(KeyCode::Enter);
        assert_eq!(model.cursor_line(), Some(4));

        model.type_keys("N");
        assert_eq!(model.cursor_line(), Some(1));
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
                job_status: JobStatus::Searching(MatchingLinesBatch {
                    matching_lines: vec![1],
                    percent_scanned: 20,
                }),
            },
        );

        model.type_keys("n");

        assert_eq!(model.cursor_line(), Some(1));
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

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                SearchError::NoPreviousSearch.to_string()
            ))
        );
    }

    #[test]
    fn given_search_done_in_the_all_tab_when_n_is_pressed_in_a_grep_tab_then_it_is_not_available_yet()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/WARN");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();

        model.type_keys("n");

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                TabError::NotAvailableInGrepTabs.to_string()
            ))
        );
    }

    #[test]
    fn given_grep_tab_when_escape_is_pressed_then_the_search_is_cleared() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/WARN");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);

        model.press(KeyCode::Esc);

        assert_eq!(model.search_state.query(), None);
    }

    #[test]
    fn given_search_running_when_a_grep_tab_becomes_active_then_its_done_leaves_the_grep_tab_cursor()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/WARN");
        model.press(KeyCode::Enter);
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);

        model.finish_scheduled_jobs();

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(model.search_state.match_count(), 1);
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
        assert_eq!(model.cursor_line(), Some(0));
        assert_eq!(model.search_state.match_count(), 0);
    }
}
