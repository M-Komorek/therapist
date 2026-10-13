mod line_collector;
mod line_index;
mod line_indexer;
mod line_scanner;

use std::{
    borrow::Cow,
    fs::File,
    io,
    ops::{ControlFlow, Range},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use line_collector::LineCollector;
pub use line_index::LineIndex;
pub use line_indexer::LineIndexer;
pub use line_scanner::LineScanner;

const READ_LINES_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum LogFileError {
    #[error("cannot open {}: {source}", path.display())]
    CannotOpen { path: PathBuf, source: io::Error },
    #[error("cannot read {}: {source}", path.display())]
    CannotRead { path: PathBuf, source: io::Error },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IndexingStatus {
    Running,
    Done,
    Failed,
}

/// The browsed log file. It is read with `read_at`, so if another process truncates the file,
/// reading returns fewer lines instead of crashing.
pub struct LogFile {
    path: PathBuf,
    file: Arc<File>,
    file_size: u64,
    line_index: LineIndex,
    indexing_status: IndexingStatus,
}

impl LogFile {
    pub fn open(path: &Path) -> Result<Self, LogFileError> {
        let cannot_open = |source| LogFileError::CannotOpen {
            path: path.to_path_buf(),
            source,
        };
        let file = File::open(path).map_err(cannot_open)?;
        let file_size = file.metadata().map_err(cannot_open)?.len();
        Ok(Self {
            path: path.to_path_buf(),
            file: Arc::new(file),
            file_size,
            line_index: LineIndex::default(),
            indexing_status: IndexingStatus::Running,
        })
    }

    pub fn line_indexer(&self) -> LineIndexer {
        LineIndexer::new(self.path.clone(), Arc::clone(&self.file), self.file_size)
    }

    pub fn line_scanner(&self) -> LineScanner {
        LineScanner::new(self.path.clone(), Arc::clone(&self.file))
    }

    pub fn set_line_index(&mut self, line_index: LineIndex) {
        self.line_index = line_index;
    }

    pub fn mark_indexing_done(&mut self) {
        self.indexing_status = IndexingStatus::Done;
    }

    pub fn mark_indexing_failed(&mut self) {
        self.indexing_status = IndexingStatus::Failed;
    }

    pub fn indexing_status(&self) -> IndexingStatus {
        self.indexing_status
    }

    pub fn percent_indexed(&self) -> u64 {
        self.line_index.percent_indexed(self.file_size)
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn file_name(&self) -> Cow<'_, str> {
        let file_name = self.path.file_name().unwrap_or(self.path.as_os_str());
        file_name.to_string_lossy()
    }

    pub fn line_count(&self) -> usize {
        self.line_index.line_count()
    }

    pub fn read_lines(&self, range: Range<usize>) -> Result<Vec<String>, LogFileError> {
        let range_end = range.end.min(self.line_count());
        if range.start >= range_end {
            return Ok(Vec::new());
        }
        let Some(checkpoint) = self.line_index.checkpoint_for(range.start) else {
            return Ok(Vec::new());
        };
        let lines_to_skip = range.start - checkpoint.line;
        let mut line_collector = LineCollector::new(lines_to_skip, range_end - range.start);
        read_chunks(
            &self.file,
            checkpoint.byte_offset,
            READ_LINES_CHUNK_SIZE,
            |chunk| line_collector.add_chunk(chunk),
        )
        .map_err(|source| LogFileError::CannotRead {
            path: self.path.clone(),
            source,
        })?;
        Ok(line_collector.finish())
    }
}

fn read_chunks(
    file: &File,
    mut byte_offset: u64,
    chunk_size: usize,
    mut on_chunk: impl FnMut(&[u8]) -> ControlFlow<()>,
) -> io::Result<()> {
    let mut buffer = vec![0; chunk_size];
    loop {
        let bytes_read = file.read_at(&mut buffer, byte_offset)?;
        if bytes_read == 0 || on_chunk(&buffer[..bytes_read]).is_break() {
            return Ok(());
        }
        byte_offset += bytes_read as u64;
    }
}

#[cfg(test)]
impl LogFile {
    pub fn from_text(text: &str) -> Self {
        let mut log_file = Self::unindexed_from_text(text);
        log_file.index_synchronously().unwrap();
        log_file
    }

    pub fn unindexed_from_text(text: &str) -> Self {
        let temp_directory = tempfile::tempdir().unwrap();
        let path = temp_directory.path().join("app.log");
        std::fs::write(&path, text).unwrap();
        // Dropping temp_directory deletes the file, but the open handle keeps it readable.
        Self::open(&path).unwrap()
    }

    pub fn index_synchronously(&mut self) -> Result<(), LogFileError> {
        self.line_indexer().run(|line_index| {
            self.line_index = line_index;
            ControlFlow::Continue(())
        })?;
        self.mark_indexing_done();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    pub(super) fn open_unindexed_with(content: &[u8]) -> (NamedTempFile, LogFile) {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content).unwrap();
        let log_file = LogFile::open(temp_file.path()).unwrap();
        (temp_file, log_file)
    }

    pub(super) fn open_with(content: &[u8]) -> (NamedTempFile, LogFile) {
        let (temp_file, mut log_file) = open_unindexed_with(content);
        log_file.index_synchronously().unwrap();
        (temp_file, log_file)
    }

    #[test]
    fn given_opened_file_when_not_indexed_yet_then_it_has_no_lines() {
        let (_temp_file, log_file) = open_unindexed_with(b"a\nb\n");

        assert_eq!(log_file.line_count(), 0);
        assert!(log_file.read_lines(0..2).unwrap().is_empty());
    }

    #[test]
    fn given_missing_file_when_opening_then_it_cannot_be_opened() {
        let temp_directory = tempfile::tempdir().unwrap();

        let result = LogFile::open(&temp_directory.path().join("missing.log"));

        assert!(matches!(result, Err(LogFileError::CannotOpen { .. })));
    }

    #[test]
    fn given_empty_file_when_reading_then_there_are_no_lines() {
        let (_temp_file, log_file) = open_with(b"");

        assert_eq!(log_file.line_count(), 0);
        assert!(log_file.read_lines(0..10).unwrap().is_empty());
    }

    #[test]
    fn given_range_past_the_end_when_reading_then_it_stops_at_the_last_line() {
        let (_temp_file, log_file) = open_with(b"a\nb\nc");

        assert_eq!(log_file.read_lines(1..10).unwrap(), ["b", "c"]);
        assert!(log_file.read_lines(3..10).unwrap().is_empty());
    }

    #[test]
    fn given_invalid_utf8_line_when_reading_then_it_is_replaced_and_numbering_holds() {
        let (_temp_file, log_file) = open_with(b"first\nbad \xff\nthird\n");

        assert_eq!(
            log_file.read_lines(0..3).unwrap(),
            ["first", "bad \u{fffd}", "third"]
        );
        assert_eq!(log_file.read_lines(2..3).unwrap(), ["third"]);
    }

    #[test]
    fn given_crlf_endings_when_reading_then_only_the_trailing_cr_is_stripped() {
        let (_temp_file, log_file) = open_with(b"a\r\nb\rc\r\n");

        assert_eq!(log_file.read_lines(0..2).unwrap(), ["a", "b\rc"]);
    }

    #[test]
    fn given_line_longer_than_read_buffer_when_reading_then_it_comes_back_whole() {
        let long_line = "x".repeat(READ_LINES_CHUNK_SIZE * 2 + 1);
        let (_temp_file, log_file) = open_with(format!("short\n{long_line}\nafter").as_bytes());

        assert_eq!(
            log_file.read_lines(1..3).unwrap(),
            [long_line.as_str(), "after"]
        );
    }

    #[test]
    fn given_skipped_line_longer_than_read_buffer_when_reading_then_the_next_line_comes_back_whole()
    {
        let long_line = "x".repeat(READ_LINES_CHUNK_SIZE * 2 + 1);
        let (_temp_file, log_file) = open_with(format!("{long_line}\nafter\nlast").as_bytes());

        assert_eq!(log_file.read_lines(1..3).unwrap(), ["after", "last"]);
    }

    #[test]
    fn given_lines_that_reach_the_byte_limit_first_when_reading_then_lines_after_checkpoints_are_found()
     {
        let line_body = "x".repeat(10_000);
        let text: String = (0..100)
            .map(|line_number| format!("{line_number} {line_body}\n"))
            .collect();
        let (_temp_file, log_file) = open_with(text.as_bytes());

        let lines = log_file.read_lines(50..52).unwrap();

        assert_eq!(
            lines,
            [format!("50 {line_body}"), format!("51 {line_body}")]
        );
    }

    #[test]
    fn given_file_truncated_after_opening_when_reading_then_fewer_lines_return() {
        let (temp_file, log_file) = open_with(b"a\nb\nc\n");

        temp_file.as_file().set_len(3).unwrap();

        assert_eq!(log_file.read_lines(0..3).unwrap(), ["a", "b"]);
        assert!(log_file.read_lines(2..3).unwrap().is_empty());
    }
}
