use super::{
    Effect,
    app::{show_error, show_info},
    log_view::{cursor_line, jump_cursor_to_line},
};
use crate::{
    action::SearchAction,
    app::Model,
    domain::{
        search::{MatchJump, SearchDirection, SearchError, SearchQuery},
        tabs::TabId,
    },
    jobs::{self, JobSlot},
};

pub(super) fn dispatch_search(model: &mut Model, search_action: SearchAction) -> Effect {
    match search_action {
        SearchAction::Start(search_query) => schedule_search(model, search_query),
        SearchAction::RepeatLast(direction) => {
            let active_tab = model.tabs.active_tab_mut();
            active_tab.search_state_mut().set_direction(direction);
            jump_to_next_match(model);
        }
        SearchAction::JumpToNextMatch => jump_to_next_match(model),
        SearchAction::JumpToPreviousMatch => jump_to_previous_match(model),
        SearchAction::Clear => {
            let active_tab_id = model.tabs.active_tab().tab_id();
            clear_search(model, active_tab_id);
        }
    }
    Effect::Redraw
}

fn schedule_search(model: &mut Model, search_query: SearchQuery) {
    let line_scanner = match model.log_file.line_scanner() {
        Ok(line_scanner) => line_scanner,
        Err(log_file_error) => {
            show_error(model, log_file_error.to_string());
            return;
        }
    };

    let active_tab_id = model.tabs.active_tab().tab_id();
    if model.job_board.has_job(JobSlot::Grepping(active_tab_id)) {
        show_error(model, SearchError::StillGrepping.to_string());
        return;
    }

    let start_line = cursor_line(model).unwrap_or_default();
    let active_tab = model.tabs.active_tab_mut();
    let phrases = active_tab.grep_phrases_with(search_query.phrase.clone());
    let searching_work = jobs::searching_work(line_scanner, model.log_file.file_size(), phrases);
    model
        .job_board
        .schedule(JobSlot::Searching(active_tab_id), searching_work);
    active_tab
        .search_state_mut()
        .replace_query(search_query, start_line);
}

pub(super) fn clear_search(model: &mut Model, tab_id: TabId) {
    model.job_board.cancel(JobSlot::Searching(tab_id));
    if let Some(search_state) = model.tabs.search_state_mut(tab_id) {
        search_state.clear();
    }
}

pub(super) fn jump_to_next_match(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let search_state = model.tabs.active_tab().search_state();
    let match_jump = search_state.next_match_from(cursor_line);
    apply_match_jump(model, match_jump);
}

fn jump_to_previous_match(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let search_state = model.tabs.active_tab().search_state();
    let match_jump = search_state.previous_match_from(cursor_line);
    apply_match_jump(model, match_jump);
}

pub(super) fn jump_to_first_match_when_settled(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let search_state = model.tabs.active_tab().search_state();
    if let Some(match_jump) = search_state.settled_first_jump(cursor_line) {
        jump_cursor_to_match(model, match_jump);
    }
}

pub(super) fn jump_to_first_match_when_done(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let search_state = model.tabs.active_tab().search_state();
    if search_state.started_from(cursor_line) {
        jump_to_next_match(model);
    }
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
                test_helpers::{SEARCHED_TEXT, grep, make_model_with_text, search, start_grep},
            },
        },
        domain::{
            log_file::{LogFile, LogFileError},
            phrase_matching::MatchingLinesBatch,
            search::SearchState,
        },
        jobs::{JobId, JobNotification, JobStatus},
    };

    fn active_search_state(model: &Model) -> &SearchState {
        model.tabs.active_tab().search_state()
    }

    #[test]
    fn given_indexed_file_when_searching_forward_then_cursor_jumps_to_the_first_match_below() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search(&mut model, "/ERROR");

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(active_search_state(&model).match_count(), 2);
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
        assert_eq!(active_search_state(&model).match_count(), 0);
        assert!(active_search_state(&model).query().is_some());
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
            Some(StatusMessage::Error(
                LogFileError::StillIndexing.to_string()
            ))
        );
        assert_eq!(active_search_state(&model).query(), None);
        let all_lines_tab_id = model.tabs.active_tab().tab_id();
        assert!(
            !model
                .job_board
                .has_job(JobSlot::Searching(all_lines_tab_id))
        );
    }

    #[test]
    fn given_scheduled_search_when_another_one_starts_then_only_the_new_one_runs() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);

        search(&mut model, "/WARN");

        assert_eq!(model.cursor_line(), Some(3));
        assert_eq!(active_search_state(&model).match_count(), 1);
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

    fn start_search(model: &mut Model, typed: &str) -> JobId {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
        model.job_board.run_scheduled_jobs()[0].job_id
    }

    fn send_search_batch(model: &mut Model, search_job_id: JobId, matching_lines: &[usize]) {
        handle_job_notification(
            model,
            JobNotification {
                job_id: search_job_id,
                job_status: JobStatus::Searching(MatchingLinesBatch {
                    matching_lines: matching_lines.to_vec(),
                    percent_scanned: 20,
                }),
            },
        );
    }

    fn search_holding_back_done(model: &mut Model, typed: &str) -> JobNotification {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
        let mut job_notifications = model.job_board.run_scheduled_jobs();
        let done_notification = job_notifications.pop().unwrap();
        model.handle_job_notifications(job_notifications);
        done_notification
    }

    #[test]
    fn given_search_still_running_when_n_is_pressed_then_it_uses_the_matches_found_so_far() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        let search_job_id = start_search(&mut model, "?ERROR");
        send_search_batch(&mut model, search_job_id, &[1]);
        assert_eq!(model.cursor_line(), Some(0));

        model.type_keys("n");

        assert_eq!(model.cursor_line(), Some(1));
        assert!(!model.job_board.is_empty());
    }

    #[test]
    fn given_forward_search_running_when_a_match_below_the_cursor_arrives_then_the_cursor_jumps_to_it()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        search_holding_back_done(&mut model, "/ERROR");

        assert_eq!(model.cursor_line(), Some(1));
        assert!(!model.job_board.is_empty());
    }

    #[test]
    fn given_cursor_on_the_first_match_already_when_the_search_is_done_then_the_cursor_stays() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        let done_notification = search_holding_back_done(&mut model, "/ERROR");

        handle_job_notification(&mut model, done_notification);

        assert_eq!(model.cursor_line(), Some(1));
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_backward_search_running_when_its_matches_arrive_then_the_cursor_waits_for_done() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");
        let done_notification = search_holding_back_done(&mut model, "?ERROR");
        assert_eq!(model.cursor_line(), Some(5));

        handle_job_notification(&mut model, done_notification);

        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_forward_search_with_matches_only_above_when_it_is_done_then_it_wraps_to_the_first() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");
        let done_notification = search_holding_back_done(&mut model, "/ERROR");
        assert_eq!(model.cursor_line(), Some(5));

        handle_job_notification(&mut model, done_notification);

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info(
                "search hit BOTTOM, continuing at TOP".to_owned()
            ))
        );
    }

    #[test]
    fn given_n_pressed_while_the_search_runs_when_a_match_below_arrives_then_the_cursor_stays() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("3G");
        let search_job_id = start_search(&mut model, "/ERROR");
        send_search_batch(&mut model, search_job_id, &[1]);
        model.type_keys("n");
        assert_eq!(model.cursor_line(), Some(1));

        send_search_batch(&mut model, search_job_id, &[4]);

        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_cursor_moved_while_the_search_runs_when_its_matches_and_done_arrive_then_the_cursor_stays()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/ERROR");
        model.press(KeyCode::Enter);
        let job_notifications = model.job_board.run_scheduled_jobs();

        model.type_keys("G");
        model.handle_job_notifications(job_notifications);

        assert_eq!(model.cursor_line(), Some(5));
        assert_eq!(active_search_state(&model).match_count(), 2);
    }

    #[test]
    fn given_search_results_when_escape_is_pressed_then_the_query_and_matches_are_gone() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");

        model.press(KeyCode::Esc);

        assert_eq!(active_search_state(&model).query(), None);
        assert_eq!(active_search_state(&model).match_count(), 0);
    }

    #[test]
    fn given_count_typed_after_a_search_when_escape_is_pressed_then_the_search_stays() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/ERROR");
        model.type_keys("5");

        model.press(KeyCode::Esc);

        assert!(active_search_state(&model).query().is_some());
        assert_eq!(active_search_state(&model).match_count(), 2);
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
    fn given_search_done_in_the_all_tab_when_n_is_pressed_in_a_grep_tab_then_there_is_no_previous_search()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/WARN");
        grep(&mut model, "ERROR");

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
    fn given_grep_tab_when_searching_then_only_its_lines_match() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");

        search(&mut model, "/t");

        assert_eq!(model.cursor_line(), Some(4));
        assert_eq!(active_search_state(&model).match_count(), 1);
        assert_eq!(active_search_state(&model).match_count_at_or_above(4), 1);
    }

    #[test]
    fn given_searches_in_two_tabs_when_switching_tabs_then_each_tab_keeps_its_own_search() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/WARN");
        grep(&mut model, "ERROR");
        search(&mut model, "/three");
        let active_phrase = |model: &Model| {
            active_search_state(model)
                .query()
                .map(|search_query| search_query.phrase.clone())
        };

        model.type_keys("tn");
        assert_eq!(active_phrase(&model), Some("WARN".to_owned()));
        assert_eq!(model.cursor_line(), Some(3));

        model.type_keys("tn");
        assert_eq!(active_phrase(&model), Some("three".to_owned()));
        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_searches_in_two_tabs_when_escape_is_pressed_in_the_grep_tab_then_only_its_search_is_cleared()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        search(&mut model, "/WARN");
        grep(&mut model, "ERROR");
        search(&mut model, "/three");

        model.press(KeyCode::Esc);

        assert_eq!(active_search_state(&model).query(), None);
        let all_lines_tab = &model.tabs.tab_states()[0];
        assert!(all_lines_tab.search_state().query().is_some());
    }

    #[test]
    fn given_grep_still_running_when_searching_in_its_tab_then_it_is_refused() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        start_grep(&mut model, "ERROR");
        let grep_tab_id = model.tabs.active_tab().tab_id();

        model.type_keys("/three");
        model.press(KeyCode::Enter);

        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(SearchError::StillGrepping.to_string()))
        );
        assert_eq!(active_search_state(&model).query(), None);
        assert!(!model.job_board.has_job(JobSlot::Searching(grep_tab_id)));
    }

    #[test]
    fn given_search_running_when_a_grep_tab_becomes_active_then_its_done_leaves_the_grep_tab_cursor()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("/WARN");
        model.press(KeyCode::Enter);
        start_grep(&mut model, "ERROR");

        model.finish_scheduled_jobs();

        assert_eq!(model.cursor_line(), Some(1));
        let all_lines_tab = &model.tabs.tab_states()[0];
        assert_eq!(all_lines_tab.search_state().match_count(), 1);
    }

    #[test]
    fn given_search_running_in_a_grep_tab_when_the_tab_is_closed_then_the_search_is_cancelled() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");
        let grep_tab_id = model.tabs.active_tab().tab_id();
        model.type_keys("/three");
        model.press(KeyCode::Enter);
        let late_job_notifications = model.job_board.run_scheduled_jobs();

        model.type_keys("tc");
        model.handle_job_notifications(late_job_notifications);

        assert!(!model.job_board.has_job(JobSlot::Searching(grep_tab_id)));
        assert_eq!(model.cursor_line(), Some(0));
        assert_eq!(active_search_state(&model).query(), None);
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
        assert_eq!(active_search_state(&model).match_count(), 0);
    }
}
