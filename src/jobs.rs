mod job;
mod log_file_indexing;
mod search;

use std::sync::mpsc::Sender;

use job::{Job, JobHandle};
pub use job::{JobContext, JobError, JobId, JobNotification, JobStatus, JobWork};
pub use log_file_indexing::log_file_indexing_work;
pub use search::search_work;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobSlot {
    LogFileIndexing,
    Search,
}

/// Every scheduled or running job, at most one per `JobSlot`. Scheduling a job cancels the one
/// already in its slot. A cancelled job stops at its next check, and the board drops whatever it
/// still sends. Every job ends with one `Done` or `Failed`, which takes it off the board. Only the
/// runtime spawns jobs, so `update` never starts threads.
#[derive(Default)]
pub struct JobBoard {
    job_handles: Vec<(JobSlot, JobHandle)>,
    scheduled_jobs: Vec<Job>,
    next_unused_job_id: u64,
}

impl JobBoard {
    pub fn schedule(&mut self, job_slot: JobSlot, job_work: JobWork) {
        self.cancel(job_slot);
        let job_id = self.take_unused_job_id();
        tracing::debug!(job_id = job_id.0, ?job_slot, "job scheduled");
        let (job, job_handle) = Job::new(job_id, job_work);
        self.job_handles.push((job_slot, job_handle));
        self.scheduled_jobs.push(job);
    }

    pub fn cancel(&mut self, job_slot: JobSlot) {
        let Some(job_handle) = self.take_job_off_board(job_slot) else {
            return;
        };

        tracing::debug!(job_id = job_handle.job_id().0, ?job_slot, "job cancelled");
        self.scheduled_jobs
            .retain(|job| job.job_id() != job_handle.job_id());
    }

    pub fn is_empty(&self) -> bool {
        self.job_handles.is_empty()
    }

    pub fn has_job(&self, job_slot: JobSlot) -> bool {
        self.job_handles
            .iter()
            .any(|(taken_slot, _)| *taken_slot == job_slot)
    }

    pub fn spawn_scheduled_jobs(&mut self, job_sender: &Sender<JobNotification>) {
        for job in self.scheduled_jobs.drain(..) {
            job.spawn(job_sender.clone());
        }
    }

    pub fn resolve(&mut self, job_notification: JobNotification) -> Option<(JobSlot, JobStatus)> {
        let JobNotification { job_id, job_status } = job_notification;
        let job_slot = self.slot_of(job_id)?;

        if job_status.is_final() {
            self.take_job_off_board(job_slot);
        }
        Some((job_slot, job_status))
    }

    fn take_unused_job_id(&mut self) -> JobId {
        let job_id = JobId(self.next_unused_job_id);
        self.next_unused_job_id += 1;
        job_id
    }

    fn slot_of(&self, job_id: JobId) -> Option<JobSlot> {
        self.job_handles
            .iter()
            .find(|(_, job_handle)| job_handle.job_id() == job_id)
            .map(|(job_slot, _)| *job_slot)
    }

    fn take_job_off_board(&mut self, job_slot: JobSlot) -> Option<JobHandle> {
        let position = self
            .job_handles
            .iter()
            .position(|(taken_slot, _)| *taken_slot == job_slot)?;
        Some(self.job_handles.remove(position).1)
    }
}

#[cfg(test)]
impl JobBoard {
    pub fn run_scheduled_jobs(&mut self) -> Vec<JobNotification> {
        let (job_sender, job_receiver) = std::sync::mpsc::channel();
        self.spawn_scheduled_jobs(&job_sender);
        drop(job_sender);
        job_receiver.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::mpsc::{self, Receiver},
        thread,
        time::Duration,
    };

    use super::*;
    use crate::domain::log_file::LineIndex;

    pub(super) fn work_that_notifies_once() -> JobWork {
        Box::new(|job_context: &JobContext| {
            job_context.notify(JobStatus::LogFileIndexing(LineIndex::default()));
            Ok(())
        })
    }

    pub(super) fn work_that_waits_until_cancelled() -> JobWork {
        Box::new(|job_context: &JobContext| {
            while !job_context.is_cancelled() {
                thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        })
    }

    fn spawn_scheduled_jobs_on_new_channel(job_board: &mut JobBoard) -> Receiver<JobNotification> {
        let (job_sender, job_receiver) = mpsc::channel();
        job_board.spawn_scheduled_jobs(&job_sender);
        job_receiver
    }

    fn receive_job_status(job_receiver: &Receiver<JobNotification>) -> JobStatus {
        let job_notification = job_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        job_notification.job_status
    }

    fn make_board_with_scheduled_job() -> JobBoard {
        let mut job_board = JobBoard::default();
        job_board.schedule(JobSlot::LogFileIndexing, work_that_notifies_once());
        job_board
    }

    #[test]
    fn given_scheduled_job_when_its_notification_is_resolved_then_it_gives_the_slot_and_status() {
        let mut job_board = make_board_with_scheduled_job();
        let mut job_notifications = job_board.run_scheduled_jobs();

        let resolution = job_board.resolve(job_notifications.remove(0));

        assert!(matches!(
            resolution,
            Some((JobSlot::LogFileIndexing, JobStatus::LogFileIndexing(_)))
        ));
        assert!(!job_board.is_empty());
    }

    #[test]
    fn given_running_job_when_done_is_resolved_then_the_board_is_empty() {
        let mut job_board = make_board_with_scheduled_job();
        let mut job_notifications = job_board.run_scheduled_jobs();
        let done_notification = job_notifications.pop().unwrap();

        let resolution = job_board.resolve(done_notification);

        assert!(matches!(
            resolution,
            Some((JobSlot::LogFileIndexing, JobStatus::Done))
        ));
        assert!(job_board.is_empty());
    }

    #[test]
    fn given_job_scheduled_in_one_slot_when_asking_each_slot_then_only_that_slot_has_a_job() {
        let job_board = make_board_with_scheduled_job();

        assert!(job_board.has_job(JobSlot::LogFileIndexing));
        assert!(!job_board.has_job(JobSlot::Search));
    }

    #[test]
    fn given_running_job_when_done_is_resolved_then_its_slot_has_no_job() {
        let mut job_board = make_board_with_scheduled_job();
        let done_notification = job_board.run_scheduled_jobs().pop().unwrap();

        job_board.resolve(done_notification);

        assert!(!job_board.has_job(JobSlot::LogFileIndexing));
    }

    #[test]
    fn given_replaced_job_when_its_notifications_arrive_then_they_are_dropped() {
        let mut job_board = make_board_with_scheduled_job();
        let replaced_job_notifications = job_board.run_scheduled_jobs();

        job_board.schedule(JobSlot::LogFileIndexing, work_that_notifies_once());

        for job_notification in replaced_job_notifications {
            assert!(job_board.resolve(job_notification).is_none());
        }
        assert!(!job_board.is_empty());
    }

    #[test]
    fn given_cancelled_job_when_its_notifications_arrive_then_they_are_dropped_and_the_board_is_empty()
     {
        let mut job_board = make_board_with_scheduled_job();
        let job_notifications = job_board.run_scheduled_jobs();

        job_board.cancel(JobSlot::LogFileIndexing);

        for job_notification in job_notifications {
            assert!(job_board.resolve(job_notification).is_none());
        }
        assert!(job_board.is_empty());
    }

    #[test]
    fn given_job_cancelled_before_it_was_spawned_when_scheduled_jobs_run_then_it_never_runs() {
        let mut job_board = make_board_with_scheduled_job();

        job_board.cancel(JobSlot::LogFileIndexing);

        assert!(job_board.run_scheduled_jobs().is_empty());
        assert!(job_board.is_empty());
    }

    #[test]
    fn given_running_job_when_cancelled_then_it_stops() {
        let mut job_board = JobBoard::default();
        job_board.schedule(JobSlot::LogFileIndexing, work_that_waits_until_cancelled());
        let job_receiver = spawn_scheduled_jobs_on_new_channel(&mut job_board);

        job_board.cancel(JobSlot::LogFileIndexing);

        assert!(matches!(receive_job_status(&job_receiver), JobStatus::Done));
    }

    #[test]
    fn given_running_job_when_replaced_then_it_stops() {
        let mut job_board = JobBoard::default();
        job_board.schedule(JobSlot::LogFileIndexing, work_that_waits_until_cancelled());
        let job_receiver = spawn_scheduled_jobs_on_new_channel(&mut job_board);

        job_board.schedule(JobSlot::LogFileIndexing, work_that_notifies_once());

        assert!(matches!(receive_job_status(&job_receiver), JobStatus::Done));
    }

    #[test]
    fn given_two_jobs_scheduled_in_one_slot_when_they_notify_then_their_ids_differ() {
        let mut job_board = make_board_with_scheduled_job();
        let first_job_id = job_board.run_scheduled_jobs()[0].job_id;

        job_board.schedule(JobSlot::LogFileIndexing, work_that_notifies_once());
        let second_job_id = job_board.run_scheduled_jobs()[0].job_id;

        assert_ne!(first_job_id, second_job_id);
    }
}
