use std::ops::ControlFlow;

use super::{JobContext, JobStatus, JobWork};
use crate::domain::{log_file::LineScanner, search::find_matching_lines};

pub fn search_work(line_scanner: LineScanner, phrase: String) -> JobWork {
    Box::new(move |job_context: &JobContext| {
        line_scanner.scan(|chunk, chunk_first_line| {
            let matching_lines = find_matching_lines(chunk, chunk_first_line, &phrase);
            if !matching_lines.is_empty() {
                job_context.notify(JobStatus::Searching(matching_lines));
            }
            if job_context.is_cancelled() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::log_file::LogFile,
        jobs::{JobBoard, JobSlot},
    };

    fn search_statuses(text: &str, phrase: &str) -> Vec<JobStatus> {
        let log_file = LogFile::from_text(text);
        let mut job_board = JobBoard::default();
        job_board.schedule(
            JobSlot::Search,
            search_work(log_file.line_scanner(), phrase.to_owned()),
        );

        job_board
            .run_scheduled_jobs()
            .into_iter()
            .map(|job_notification| job_notification.job_status)
            .collect()
    }

    #[test]
    fn given_phrase_in_the_file_when_its_search_work_runs_then_the_matching_lines_arrive_before_done()
     {
        let job_statuses = search_statuses("ok\nERROR a\nok\nERROR b\n", "ERROR");

        assert!(matches!(
            job_statuses.as_slice(),
            [JobStatus::Searching(matching_lines), JobStatus::Done] if matching_lines == &[1, 3]
        ));
    }

    #[test]
    fn given_phrase_not_in_the_file_when_its_search_work_runs_then_only_done_arrives() {
        let job_statuses = search_statuses("ok\nok\n", "ERROR");

        assert!(matches!(job_statuses.as_slice(), [JobStatus::Done]));
    }
}
