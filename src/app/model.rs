use crate::{
    control::InputController,
    domain::{bookmarks::Bookmarks, log_file::LogFile, tabs::Tabs},
    jobs::{self, JobBoard, JobSlot},
};

#[derive(Debug, PartialEq)]
pub enum StatusMessage {
    Info(String),
    Error(String),
}

/// All of the app's state. Only `update` changes it, and rendering only reads it. The fields are
/// public so that `update` can borrow several of them at once.
pub struct Model {
    pub bookmarks: Bookmarks,
    pub input_controller: InputController,
    pub job_board: JobBoard,
    pub log_file: LogFile,
    pub tabs: Tabs,
    pub status_message: Option<StatusMessage>,
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
            tabs: Tabs::default(),
            status_message: None,
        }
    }
}
