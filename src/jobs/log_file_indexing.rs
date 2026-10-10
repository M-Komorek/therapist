use std::ops::ControlFlow;

use super::{JobContext, JobStatus, JobWork};
use crate::domain::log_file::LineIndexer;

pub fn log_file_indexing_work(line_indexer: LineIndexer) -> JobWork {
    Box::new(move |job_context: &JobContext| {
        line_indexer.run(|line_index| {
            job_context.notify(JobStatus::LogFileIndexing(line_index));
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

    #[test]
    fn given_log_file_when_its_indexing_work_runs_then_progress_arrives_before_done() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");
        let mut job_board = JobBoard::default();
        job_board.schedule(
            JobSlot::LogFileIndexing,
            log_file_indexing_work(log_file.line_indexer()),
        );

        let job_notifications = job_board.run_scheduled_jobs();

        let job_statuses: Vec<&JobStatus> = job_notifications
            .iter()
            .map(|job_notification| &job_notification.job_status)
            .collect();
        assert!(matches!(
            job_statuses.as_slice(),
            [
                JobStatus::LogFileIndexing(_),
                JobStatus::LogFileIndexing(_),
                JobStatus::Done
            ]
        ));
    }
}
