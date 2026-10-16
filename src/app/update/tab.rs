use super::{
    Effect,
    app::show_error,
    log_view::{cursor_line, jump_cursor_to_line},
};
use crate::{
    action::TabAction,
    app::Model,
    domain::tabs::TabId,
    jobs::{self, JobSlot},
};

pub(super) fn dispatch_tab(model: &mut Model, tab_action: TabAction) -> Effect {
    match tab_action {
        TabAction::Grep(phrase) => schedule_grep(model, phrase),
        TabAction::SwitchToNext => model.tabs.switch_to_next(),
        TabAction::SwitchToPrevious => model.tabs.switch_to_previous(),
        TabAction::CloseActive => close_active_tab(model),
        TabAction::ShowCursorLineInAllLinesTab => show_cursor_line_in_all_lines_tab(model),
    }
    Effect::Redraw
}

fn schedule_grep(model: &mut Model, phrase: String) {
    let line_scanner = match model.log_file.line_scanner() {
        Ok(line_scanner) => line_scanner,
        Err(log_file_error) => {
            show_error(model, log_file_error.to_string());
            return;
        }
    };

    let phrases = model.tabs.active_tab().grep_phrases_with(phrase);
    let grepping_work =
        jobs::phrase_scanning_work(line_scanner, model.log_file.file_size(), phrases.clone());
    let tab_id = model.tabs.open_grep_tab(phrases);
    model
        .job_board
        .schedule(JobSlot::Grepping(tab_id), grepping_work);
}

fn close_active_tab(model: &mut Model) {
    let active_tab_id = model.tabs.active_tab().tab_id();
    close_tab(model, active_tab_id);
}

fn show_cursor_line_in_all_lines_tab(model: &mut Model) {
    let cursor_line = cursor_line(model);
    model.tabs.switch_to_all_lines_tab();
    if let Some(cursor_line) = cursor_line {
        jump_cursor_to_line(model, cursor_line);
    }
}

pub(super) fn close_tab(model: &mut Model, tab_id: TabId) {
    match model.tabs.close(tab_id) {
        Ok(()) => {
            model.job_board.cancel(JobSlot::Grepping(tab_id));
            model.job_board.cancel(JobSlot::Searching(tab_id));
        }
        Err(tab_error) => show_error(model, tab_error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Size;

    use super::*;
    use crate::{
        app::update::{
            handle_job_notification, resize_log_view,
            test_helpers::{SEARCHED_TEXT, error_message, grep, make_model_with_text, start_grep},
        },
        domain::{
            log_file::{LogFile, LogFileError},
            tabs::{GrepError, TabError, TabState},
        },
        jobs::{JobError, JobNotification, JobStatus},
    };

    fn tab_names(model: &Model) -> Vec<String> {
        model.tabs.tab_states().iter().map(TabState::name).collect()
    }

    fn active_row_count(model: &Model) -> usize {
        let line_count = model.log_file.line_count();
        model.tabs.active_tab().row_count(line_count)
    }

    #[test]
    fn given_indexed_file_when_grepping_then_a_tab_named_after_the_phrase_shows_the_matching_lines()
    {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        grep(&mut model, "ERROR");

        assert_eq!(tab_names(&model), ["all", "ERROR"]);
        assert_eq!(model.tabs.active_position(), 1);
        assert_eq!(active_row_count(&model), 2);
        assert_eq!(model.cursor_line(), Some(1));
        model.type_keys("j");
        assert_eq!(model.cursor_line(), Some(4));
        assert!(model.job_board.is_empty());
    }

    #[test]
    fn given_grep_submitted_when_its_job_has_not_run_yet_then_the_tab_is_open_and_its_job_scheduled()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        start_grep(&mut model, "ERROR");

        let grep_tab_id = model.tabs.active_tab().tab_id();
        assert!(model.job_board.has_job(JobSlot::Grepping(grep_tab_id)));
        assert_eq!(active_row_count(&model), 0);
    }

    #[test]
    fn given_file_still_indexing_when_grepping_then_it_is_refused() {
        let mut model = Model::with_screen_size(
            LogFile::unindexed_from_text(SEARCHED_TEXT),
            Size::new(20, 5),
        );

        start_grep(&mut model, "ERROR");

        assert_eq!(
            model.status_message,
            error_message(LogFileError::StillIndexing)
        );
        assert_eq!(tab_names(&model), ["all"]);
    }

    #[test]
    fn given_two_greps_when_their_batches_arrive_together_then_each_fills_its_own_tab() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        start_grep(&mut model, "ERROR");
        model.type_keys("tp");
        start_grep(&mut model, "WARN");

        model.finish_scheduled_jobs();

        let row_counts: Vec<usize> = model
            .tabs
            .tab_states()
            .iter()
            .map(|tab_state| tab_state.row_count(6))
            .collect();
        assert_eq!(tab_names(&model), ["all", "WARN", "ERROR"]);
        assert_eq!(row_counts, [6, 1, 2]);
    }

    #[test]
    fn given_grep_tab_when_grepping_in_it_then_a_child_tab_right_after_it_shows_lines_with_both_phrases()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "WARN");
        model.type_keys("tp");
        grep(&mut model, "ERROR");

        grep(&mut model, "three");

        assert_eq!(tab_names(&model), ["all", "ERROR", "ERROR › three", "WARN"]);
        assert_eq!(model.tabs.active_position(), 2);
        assert_eq!(active_row_count(&model), 1);
        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_parent_tab_still_grepping_when_grepping_in_it_then_the_child_tab_greps_on_its_own() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        start_grep(&mut model, "ERROR");
        let parent_tab_id = model.tabs.active_tab().tab_id();

        start_grep(&mut model, "three");
        let child_tab_id = model.tabs.active_tab().tab_id();

        assert!(model.job_board.has_job(JobSlot::Grepping(parent_tab_id)));
        assert!(model.job_board.has_job(JobSlot::Grepping(child_tab_id)));
        model.finish_scheduled_jobs();
        assert_eq!(tab_names(&model), ["all", "ERROR", "ERROR › three"]);
        assert_eq!(active_row_count(&model), 1);
    }

    #[test]
    fn given_child_tab_when_its_parent_is_closed_then_the_child_keeps_its_lines_and_grep() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");
        let parent_tab_id = model.tabs.active_tab().tab_id();
        start_grep(&mut model, "three");
        let child_tab_id = model.tabs.active_tab().tab_id();

        close_tab(&mut model, parent_tab_id);

        assert_eq!(tab_names(&model), ["all", "ERROR › three"]);
        assert!(model.job_board.has_job(JobSlot::Grepping(child_tab_id)));
        model.finish_scheduled_jobs();
        assert_eq!(model.tabs.active_tab().tab_id(), child_tab_id);
        assert_eq!(active_row_count(&model), 1);
        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_grep_tab_when_a_cascade_finds_nothing_then_the_child_closes_and_its_parent_is_active()
    {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");

        grep(&mut model, "WARN");

        assert_eq!(tab_names(&model), ["all", "ERROR"]);
        assert_eq!(model.tabs.active_tab().name(), "ERROR");
        assert_eq!(
            model.status_message,
            error_message("Pattern not found: WARN")
        );
    }

    #[test]
    fn given_cursor_in_a_grep_tab_when_ta_is_pressed_then_the_all_tab_shows_the_same_line() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");
        model.type_keys("j");

        model.type_keys("ta");

        assert!(model.tabs.active_tab().shows_all_lines());
        assert_eq!(model.cursor_line(), Some(4));
        model.type_keys("tn");
        assert_eq!(model.cursor_line(), Some(4));
        assert_eq!(model.tabs.active_tab().log_view_state().cursor_row(), 1);
    }

    #[test]
    fn given_all_tab_when_ta_is_pressed_then_the_cursor_and_scroll_stay() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("2j");
        let top_row = model.tabs.active_tab().log_view_state().top_row();

        model.type_keys("ta");

        assert_eq!(model.cursor_line(), Some(2));
        assert_eq!(model.tabs.active_tab().log_view_state().top_row(), top_row);
    }

    #[test]
    fn given_grep_tab_without_lines_yet_when_ta_is_pressed_then_the_all_tab_is_active_and_its_cursor_stays()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("2j");
        start_grep(&mut model, "ERROR");

        model.type_keys("ta");

        assert!(model.tabs.active_tab().shows_all_lines());
        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_cursors_moved_in_two_tabs_when_switching_between_them_then_each_keeps_its_cursor() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("G");
        grep(&mut model, "ERROR");
        model.type_keys("j");

        model.type_keys("tp");
        assert_eq!(model.cursor_line(), Some(5));

        model.type_keys("tn");
        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_grep_tab_when_the_terminal_is_resized_then_every_tab_gets_the_new_height() {
        let text: String = (0..100).map(|line| format!("ERROR {line}\n")).collect();
        let mut model = make_model_with_text(&text);
        grep(&mut model, "ERROR");
        resize_log_view(&mut model, Size::new(20, 12));

        model.type_keys("tp");
        model.press_ctrl('d');

        assert_eq!(model.tabs.active_tab().log_view_state().top_row(), 5);
    }

    #[test]
    fn given_grep_tab_when_going_to_a_line_number_with_g_that_it_does_not_show_then_the_cursor_goes_to_the_next_shown_line()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");

        model.type_keys("3G");

        assert_eq!(model.cursor_line(), Some(4));
    }

    #[test]
    fn given_cursor_in_a_grep_tab_when_bb_is_pressed_then_its_file_line_is_bookmarked() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");
        model.type_keys("j");

        model.type_keys("bb");

        assert!(model.bookmarks.contains(4));
    }

    #[test]
    fn given_grep_tab_when_tc_is_pressed_then_it_closes_and_the_all_tab_is_active() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        grep(&mut model, "ERROR");

        model.type_keys("tc");

        assert_eq!(tab_names(&model), ["all"]);
        assert!(model.tabs.active_tab().shows_all_lines());
    }

    #[test]
    fn given_all_tab_active_when_tc_is_pressed_then_an_error_says_it_cannot_be_closed() {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        model.type_keys("tc");

        assert_eq!(
            model.status_message,
            error_message(TabError::CannotCloseAllLinesTab)
        );
    }

    #[test]
    fn given_running_grep_when_its_tab_is_closed_then_the_job_is_cancelled_and_its_late_notifications_are_dropped()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        start_grep(&mut model, "ERROR");
        let late_job_notifications = model.job_board.run_scheduled_jobs();

        model.type_keys("tc");
        model.handle_job_notifications(late_job_notifications);

        assert!(model.job_board.is_empty());
        assert_eq!(tab_names(&model), ["all"]);
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_phrase_not_in_the_file_when_the_grep_is_done_then_its_tab_closes_and_an_error_says_so()
    {
        let mut model = make_model_with_text(SEARCHED_TEXT);

        grep(&mut model, "zzz");

        assert_eq!(tab_names(&model), ["all"]);
        assert_eq!(
            model.status_message,
            error_message(GrepError::PatternNotFound("zzz".to_owned()))
        );
    }

    #[test]
    fn given_running_grep_when_it_fails_then_its_tab_closes_and_an_error_says_so() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        start_grep(&mut model, "ERROR");
        let grep_job_id = model.job_board.run_scheduled_jobs()[0].job_id;

        handle_job_notification(
            &mut model,
            JobNotification {
                job_id: grep_job_id,
                job_status: JobStatus::Failed(JobError::Panicked),
            },
        );

        assert_eq!(tab_names(&model), ["all"]);
        assert_eq!(
            model.status_message,
            error_message("Grep failed: the job panicked")
        );
        assert!(model.job_board.is_empty());
    }
}
