use std::{
    panic::{self, AssertUnwindSafe},
    sync::mpsc::Sender,
    thread,
};

use crate::domain::log_file::{Indexer, LineIndex, LogFileError};

/// What a job thread sends to the run loop. Every job ends with exactly one `Done` or `Failed`.
/// Sending fails only once the run loop has stopped, which means the app is quitting, so jobs
/// ignore send errors.
#[derive(Debug)]
pub enum JobMessage {
    IndexProgress(LineIndex),
    Done,
    Failed(JobError),
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error(transparent)]
    LogFile(#[from] LogFileError),
    #[error("the job panicked")]
    Panicked,
}

pub fn spawn_indexer(job_sender: Sender<JobMessage>, indexer: Indexer) {
    spawn(job_sender, move |job_sender| {
        indexer.run(|line_index| {
            let _ = job_sender.send(JobMessage::IndexProgress(line_index));
        })?;
        Ok(())
    });
}

fn spawn(
    job_sender: Sender<JobMessage>,
    job: impl FnOnce(&Sender<JobMessage>) -> Result<(), JobError> + Send + 'static,
) {
    thread::spawn(move || {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| job(&job_sender)));
        let _ = job_sender.send(final_message(outcome));
    });
}

fn final_message(outcome: thread::Result<Result<(), JobError>>) -> JobMessage {
    match outcome {
        Ok(Ok(())) => JobMessage::Done,
        Ok(Err(error)) => JobMessage::Failed(error),
        Err(_panic_payload) => JobMessage::Failed(JobError::Panicked),
    }
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf, sync::mpsc};

    use super::*;
    use crate::domain::log_file::LogFile;

    fn run_job(
        job: impl FnOnce(&Sender<JobMessage>) -> Result<(), JobError> + Send + 'static,
    ) -> Vec<JobMessage> {
        let (job_sender, job_receiver) = mpsc::channel();
        spawn(job_sender, job);
        job_receiver.iter().collect()
    }

    #[test]
    fn given_log_file_when_indexer_job_runs_then_progress_arrives_before_done() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");
        let (job_sender, job_receiver) = mpsc::channel();

        spawn_indexer(job_sender, log_file.indexer());

        let messages: Vec<JobMessage> = job_receiver.iter().collect();
        assert!(matches!(
            messages.as_slice(),
            [
                JobMessage::IndexProgress(_),
                JobMessage::IndexProgress(_),
                JobMessage::Done
            ]
        ));
    }

    #[test]
    fn given_job_that_succeeds_when_it_finishes_then_done_is_the_only_message() {
        let messages = run_job(|_job_sender| Ok(()));

        assert!(matches!(messages.as_slice(), [JobMessage::Done]));
    }

    #[test]
    fn given_job_that_returns_an_error_when_it_finishes_then_failed_carries_the_error() {
        let messages = run_job(|_job_sender| {
            Err(JobError::LogFile(LogFileError::CannotRead {
                path: PathBuf::from("app.log"),
                source: io::Error::other("disk failed"),
            }))
        });

        assert!(matches!(
            messages.as_slice(),
            [JobMessage::Failed(JobError::LogFile(
                LogFileError::CannotRead { .. }
            ))]
        ));
    }

    #[test]
    fn given_job_that_panics_when_it_runs_then_failed_is_sent() {
        let messages = run_job(|_job_sender| panic!("job panicked on purpose"));

        assert!(matches!(
            messages.as_slice(),
            [JobMessage::Failed(JobError::Panicked)]
        ));
    }
}
