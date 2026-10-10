use crate::{
    control::InputController,
    domain::{log_file::LogFile, log_view_state::LogViewState},
    jobs::{self, JobBoard, JobSlot},
};

#[derive(Debug, PartialEq)]
pub enum StatusMessage {
    Error(String),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LogFileIndexingStatus {
    Running,
    Done,
    Failed,
}

/// All of the app's state. Only `update` changes it, and rendering only reads it. The fields are
/// public so that `update` can borrow several of them at once.
pub struct Model {
    pub log_file: LogFile,
    pub log_view_state: LogViewState,
    pub input_controller: InputController,
    pub status_message: Option<StatusMessage>,
    pub job_board: JobBoard,
    pub log_file_indexing_status: LogFileIndexingStatus,
}

impl Model {
    pub fn new(log_file: LogFile) -> Self {
        let mut job_board = JobBoard::default();
        job_board.schedule(
            JobSlot::LogFileIndexing,
            jobs::log_file_indexing_work(log_file.indexer()),
        );

        Self {
            log_file,
            log_view_state: LogViewState::default(),
            input_controller: InputController::default(),
            status_message: None,
            job_board,
            log_file_indexing_status: LogFileIndexingStatus::Running,
        }
    }
}
