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
        use std::io::Write;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(text.as_bytes()).unwrap();
        // Dropping temp_file unlinks it, but the open handle in LogFile keeps it readable.
        Self::open(temp_file.path()).unwrap()
    }
}
