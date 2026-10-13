use super::{
    Effect,
    app::show_error,
    search::{clear_search, jump_to_next_match},
    tab::close_tab,
};
use crate::{
    app::Model,
    domain::tabs::{GrepError, TabId},
    jobs::{JobError, JobNotification, JobSlot, JobStatus},
};

pub fn handle_job_notification(model: &mut Model, job_notification: JobNotification) -> Effect {
    let Some((job_slot, job_status)) = model.job_board.resolve(job_notification) else {
        return Effect::Nothing;
    };

    match job_status {
        JobStatus::LogFileIndexing(line_index) => model.log_file.set_line_index(line_index),
        JobStatus::Searching(matching_lines_batch) => {
            if let JobSlot::Searching(tab_id) = job_slot
                && let Some(search_state) = model.tabs.search_state_mut(tab_id)
            {
                search_state.add_batch(matching_lines_batch);
            }
        }
        JobStatus::Grepping(matching_lines_batch) => {
            if let JobSlot::Grepping(tab_id) = job_slot {
                model.tabs.add_grep_batch(tab_id, matching_lines_batch);
            }
        }
        JobStatus::Done => mark_job_done(model, job_slot),
        JobStatus::Failed(job_error) => mark_job_failed(model, job_slot, job_error),
    }
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
            model.log_file.mark_indexing_done();
        }
        JobSlot::Searching(tab_id) => mark_search_done(model, tab_id),
        JobSlot::Grepping(tab_id) => mark_grep_done(model, tab_id),
    }
}

fn mark_job_failed(model: &mut Model, job_slot: JobSlot, job_error: JobError) {
    match job_slot {
        JobSlot::LogFileIndexing => {
            tracing::error!(error = %job_error, "indexing failed");
            model.log_file.mark_indexing_failed();
        }
        JobSlot::Searching(tab_id) => {
            tracing::error!(error = %job_error, "search failed");
            clear_search(model, tab_id);
            show_error(model, format!("Search failed: {job_error}"));
        }
        JobSlot::Grepping(tab_id) => {
            tracing::error!(error = %job_error, "grep failed");
            close_tab(model, tab_id);
            show_error(model, format!("Grep failed: {job_error}"));
        }
    }
}

fn mark_search_done(model: &mut Model, tab_id: TabId) {
    let Some(search_state) = model.tabs.search_state(tab_id) else {
        return;
    };

    tracing::debug!(match_count = search_state.match_count(), "search done");
    if model.tabs.active_tab().tab_id() == tab_id {
        jump_to_next_match(model);
    }
}

fn mark_grep_done(model: &mut Model, tab_id: TabId) {
    let Some(grep_state) = model.tabs.grep_state(tab_id) else {
        return;
    };

    tracing::debug!(match_count = grep_state.match_count(), "grep done");
    if grep_state.match_count() == 0 {
        let last_phrase = grep_state.phrases().last().cloned().unwrap_or_default();
        let pattern_not_found = GrepError::PatternNotFound(last_phrase);
        close_tab(model, tab_id);
        show_error(model, pattern_not_found.to_string());
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{crossterm::event::KeyCode, layout::Size};

    use super::*;
    use crate::{
        app::{
            StatusMessage,
            update::test_helpers::{SEARCHED_TEXT, make_model_with_text},
        },
        domain::log_file::{IndexingStatus, LogFile},
    };

    fn make_indexing_model(text: &str) -> (Model, Vec<JobNotification>) {
        let mut model =
            Model::with_screen_size(LogFile::unindexed_from_text(text), Size::new(20, 5));
        let job_notifications = model.job_board.run_scheduled_jobs();
        (model, job_notifications)
    }

    #[test]
    fn given_unindexed_file_when_app_starts_then_indexing_runs() {
        let model = Model::new(LogFile::unindexed_from_text("a\nb\n"));

        assert_eq!(model.log_file.indexing_status(), IndexingStatus::Running);
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

        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_indexing_app_when_done_arrives_then_indexing_is_done_and_no_job_is_left() {
        let (mut model, job_notifications) = make_indexing_model("a\nb\n");

        model.handle_job_notifications(job_notifications);

        assert_eq!(model.log_file.indexing_status(), IndexingStatus::Done);
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

        assert_eq!(model.log_file.indexing_status(), IndexingStatus::Failed);
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
        assert_eq!(model.log_file.indexing_status(), IndexingStatus::Done);
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
        let search_state = model.tabs.active_tab().search_state();
        assert_eq!(search_state.query(), None);
        assert_eq!(search_state.match_count(), 0);
        assert!(model.job_board.is_empty());
    }
}
