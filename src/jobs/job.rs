use std::{
    panic::{self, AssertUnwindSafe},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
};

use crate::domain::log_file::{LineIndex, LogFileError};

pub type JobWork = Box<dyn FnOnce(&JobContext) -> Result<(), JobError> + Send>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobId(pub(super) u64);

/// What a job thread sends to the run loop. Every job ends with exactly one `Done` or `Failed`.
/// Sending fails only once the run loop has stopped, which means the app is quitting, so jobs
/// ignore send errors.
#[derive(Debug)]
pub struct JobNotification {
    pub job_id: JobId,
    pub job_status: JobStatus,
}

#[derive(Debug)]
pub enum JobStatus {
    LogFileIndexing(LineIndex),
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

/// What a job's work receives: it notifies the app through it, and checks it for cancelling.
pub struct JobContext {
    job_id: JobId,
    job_sender: Sender<JobNotification>,
    cancel_flag: Arc<AtomicBool>,
}

pub(super) struct Job {
    job_id: JobId,
    cancel_flag: Arc<AtomicBool>,
    job_work: JobWork,
}

/// Dropping it cancels its job. The job sees that at its next check, so it may still send a few
/// notifications.
pub(super) struct JobHandle {
    job_id: JobId,
    cancel_flag: Arc<AtomicBool>,
}

impl JobStatus {
    pub(super) fn is_final(&self) -> bool {
        matches!(self, Self::Done | Self::Failed(_))
    }
}

impl JobContext {
    pub fn notify(&self, job_status: JobStatus) {
        let job_notification = JobNotification {
            job_id: self.job_id,
            job_status,
        };
        let _ = self.job_sender.send(job_notification);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

impl Job {
    pub(super) fn new(job_id: JobId, job_work: JobWork) -> (Self, JobHandle) {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let job = Self {
            job_id,
            cancel_flag: Arc::clone(&cancel_flag),
            job_work,
        };
        (
            job,
            JobHandle {
                job_id,
                cancel_flag,
            },
        )
    }

    pub(super) fn job_id(&self) -> JobId {
        self.job_id
    }

    pub(super) fn spawn(self, job_sender: Sender<JobNotification>) {
        thread::spawn(move || {
            let job_context = JobContext {
                job_id: self.job_id,
                job_sender,
                cancel_flag: self.cancel_flag,
            };
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| (self.job_work)(&job_context)));
            job_context.notify(final_status(outcome));
        });
    }
}

impl JobHandle {
    pub(super) fn job_id(&self) -> JobId {
        self.job_id
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }
}

fn final_status(outcome: thread::Result<Result<(), JobError>>) -> JobStatus {
    match outcome {
        Ok(Ok(())) => JobStatus::Done,
        Ok(Err(job_error)) => JobStatus::Failed(job_error),
        Err(_panic_payload) => JobStatus::Failed(JobError::Panicked),
    }
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf, sync::mpsc, time::Duration};

    use super::*;
    use crate::jobs::tests::{work_that_notifies_once, work_that_waits_until_cancelled};

    fn run_job(job_id: JobId, job_work: JobWork) -> Vec<JobNotification> {
        let (job, _job_handle) = Job::new(job_id, job_work);
        let (job_sender, job_receiver) = mpsc::channel();
        job.spawn(job_sender);
        job_receiver.iter().collect()
    }

    fn job_statuses(job_notifications: Vec<JobNotification>) -> Vec<JobStatus> {
        job_notifications
            .into_iter()
            .map(|job_notification| job_notification.job_status)
            .collect()
    }

    #[test]
    fn given_job_that_succeeds_when_it_finishes_then_done_is_the_only_notification() {
        let job_notifications = run_job(JobId(0), Box::new(|_job_context| Ok(())));

        assert!(matches!(
            job_statuses(job_notifications).as_slice(),
            [JobStatus::Done]
        ));
    }

    #[test]
    fn given_job_that_returns_an_error_when_it_finishes_then_failed_carries_the_error() {
        let job_notifications = run_job(
            JobId(0),
            Box::new(|_job_context| {
                Err(JobError::LogFile(LogFileError::CannotRead {
                    path: PathBuf::from("app.log"),
                    source: io::Error::other("disk failed"),
                }))
            }),
        );

        assert!(matches!(
            job_statuses(job_notifications).as_slice(),
            [JobStatus::Failed(JobError::LogFile(
                LogFileError::CannotRead { .. }
            ))]
        ));
    }

    #[test]
    fn given_job_that_panics_when_it_runs_then_failed_is_sent() {
        let job_notifications = run_job(
            JobId(0),
            Box::new(|_job_context| panic!("job panicked on purpose")),
        );

        assert!(matches!(
            job_statuses(job_notifications).as_slice(),
            [JobStatus::Failed(JobError::Panicked)]
        ));
    }

    #[test]
    fn given_running_job_when_its_handle_is_dropped_then_the_job_sees_it_is_cancelled() {
        let (job, job_handle) = Job::new(JobId(0), work_that_waits_until_cancelled());
        let (job_sender, job_receiver) = mpsc::channel();
        job.spawn(job_sender);

        drop(job_handle);

        let job_notification = job_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(job_notification.job_status, JobStatus::Done));
    }

    #[test]
    fn given_job_that_notifies_when_it_runs_then_every_notification_carries_its_id() {
        let job_notifications = run_job(JobId(7), work_that_notifies_once());

        assert_eq!(job_notifications.len(), 2);
        assert!(
            job_notifications
                .iter()
                .all(|job_notification| job_notification.job_id == JobId(7))
        );
    }
}
