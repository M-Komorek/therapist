use std::path::Path;

use crate::{
    log_file::{LogFile, LogFileError},
    tabs::Tab,
};

pub struct Session {
    pub log_file: LogFile,
    pub tab: Tab,
}

impl Session {
    pub fn open(path: &Path) -> Result<Self, LogFileError> {
        Ok(Self {
            log_file: LogFile::open(path)?,
            tab: Tab::default(),
        })
    }
}

#[cfg(test)]
impl Session {
    pub fn from_text(text: &str) -> Self {
        let temp_directory = tempfile::tempdir().unwrap();
        let path = temp_directory.path().join("app.log");
        std::fs::write(&path, text).unwrap();
        // Dropping temp_directory deletes the file, but the open handle in LogFile keeps it readable.
        Self::open(&path).unwrap()
    }
}
