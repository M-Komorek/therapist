use crate::{
    control::InputController,
    domain::{
        bookmarks::Bookmarks, log_file::LogFile, log_view_state::LogViewState, search::SearchState,
    },
    jobs::{self, JobBoard, JobSlot},
};

#[derive(Debug, PartialEq)]
pub enum StatusMessage {
    Info(String),
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
    // TODO Consider adding a bag for States and a bag for Statuses
    pub bookmarks: Bookmarks,
    pub input_controller: InputController,
    pub job_board: JobBoard,
    pub log_file: LogFile,
    pub log_view_state: LogViewState,
    pub search_state: SearchState,
    pub status_message: Option<StatusMessage>,
    pub log_file_indexing_status: LogFileIndexingStatus,
}

impl Model {
    pub fn new(log_file: LogFile) -> Self {
        let mut job_board = JobBoard::default();
        job_board.schedule(
            JobSlot::LogFileIndexing,
            jobs::log_file_indexing_work(log_file.line_indexer()),
        );

        Self {
            bookmarks: Bookmarks::default(),
            input_controller: InputController::default(),
            job_board,
            log_file,
            log_view_state: LogViewState::default(),
            search_state: SearchState::default(),
            status_message: None,
            log_file_indexing_status: LogFileIndexingStatus::Running,
        }
    }
}
