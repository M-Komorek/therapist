use crate::{
    control::Control,
    domain::{log_file::LogFile, log_view_state::LogViewState},
};

#[derive(Debug, PartialEq)]
pub enum StatusMessage {
    Error(String),
}

/// All of the app's state, as plain data. Only `update` changes it; rendering only reads it.
/// The fields are public so that `update` can borrow several of them at once.
pub struct Model {
    pub log_file: LogFile,
    pub log_view_state: LogViewState,
    pub control: Control,
    pub status_message: Option<StatusMessage>,
}

impl Model {
    pub fn new(log_file: LogFile) -> Self {
        Self {
            log_file,
            log_view_state: LogViewState::default(),
            control: Control::default(),
            status_message: None,
        }
    }
}
