use std::ops::ControlFlow;

use super::{JobContext, JobStatus, JobWork};
use crate::domain::{
    log_file::LineScanner,
    phrase_matching::{MatchingLinesBatch, find_matching_lines},
};

pub fn phrase_scanning_work(
    line_scanner: LineScanner,
    file_size: u64,
    phrases: Vec<String>,
) -> JobWork {
    Box::new(move |job_context: &JobContext| {
        let mut bytes_scanned = 0;
        line_scanner.scan(|chunk, chunk_first_line| {
            bytes_scanned += chunk.len() as u64;
            job_context.notify(JobStatus::PhraseScanning(MatchingLinesBatch {
                matching_lines: find_matching_lines(chunk, chunk_first_line, &phrases),
                percent_scanned: percent_scanned(bytes_scanned, file_size),
            }));
            if job_context.is_cancelled() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        Ok(())
    })
}

fn percent_scanned(bytes_scanned: u64, file_size: u64) -> u64 {
    if file_size == 0 {
        return 100;
    }
    (bytes_scanned * 100 / file_size).min(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{log_file::LogFile, tabs::Tabs},
        jobs::{JobBoard, JobSlot},
    };

    fn phrase_scanning_statuses(text: &str, phrase: &str) -> Vec<JobStatus> {
        let log_file = LogFile::from_text(text);
        let tab_id = Tabs::default().active_tab().tab_id();
        let mut job_board = JobBoard::default();
        job_board.schedule(
            JobSlot::Searching(tab_id),
            phrase_scanning_work(
                log_file.line_scanner().unwrap(),
                log_file.file_size(),
                vec![phrase.to_owned()],
            ),
        );

        job_board
            .run_scheduled_jobs()
            .into_iter()
            .map(|job_notification| job_notification.job_status)
            .collect()
    }

    #[test]
    fn given_phrase_in_the_file_when_its_phrase_scanning_work_runs_then_the_matching_lines_arrive_before_done()
     {
        let job_statuses = phrase_scanning_statuses("ok\nERROR a\nok\nERROR b\n", "ERROR");

        assert!(matches!(
            job_statuses.as_slice(),
            [
                JobStatus::PhraseScanning(MatchingLinesBatch { matching_lines, percent_scanned: 100 }),
                JobStatus::Done
            ] if matching_lines == &[1, 3]
        ));
    }

    #[test]
    fn given_phrase_not_in_the_file_when_its_phrase_scanning_work_runs_then_only_the_progress_arrives()
     {
        let job_statuses = phrase_scanning_statuses("ok\nok\n", "ERROR");

        assert!(matches!(
            job_statuses.as_slice(),
            [
                JobStatus::PhraseScanning(MatchingLinesBatch { matching_lines, percent_scanned: 100 }),
                JobStatus::Done
            ] if matching_lines.is_empty()
        ));
    }

    #[test]
    fn given_bytes_scanned_when_computing_the_percent_then_it_is_rounded_down_and_stops_at_100() {
        assert_eq!(percent_scanned(0, 0), 100);
        assert_eq!(percent_scanned(62, 100), 62);
        assert_eq!(percent_scanned(999, 1000), 99);
        assert_eq!(percent_scanned(150, 100), 100);
    }
}
