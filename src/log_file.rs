mod line_index;

use std::{
    borrow::Cow,
    fs::File,
    io,
    ops::{ControlFlow, Range},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use line_index::LineIndex;

const LINES_PER_CHECKPOINT: usize = 1000;
const BYTES_PER_CHECKPOINT: u64 = 64 * 1024;
const INDEX_CHUNK_SIZE: usize = 1024 * 1024;
const READ_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum LogFileError {
    #[error("cannot open {}: {source}", path.display())]
    CannotOpen { path: PathBuf, source: io::Error },
    #[error("cannot read {}: {source}", path.display())]
    CannotRead { path: PathBuf, source: io::Error },
}

/// A log file read with `read_at` instead of mmap, so a file truncated by another process gives
/// fewer lines instead of a SIGBUS crash.
pub struct LogFile {
    path: PathBuf,
    file: Arc<File>,
    file_size: u64,
    line_index: LineIndex,
    indexing: Indexing,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Indexing {
    Running,
    Done,
    Failed,
}

/// Builds a log file's line index on another thread. It reports a copy of the index so far after
/// the first chunk and each time the indexed percentage grows, then reports the finished index.
pub struct Indexer {
    path: PathBuf,
    file: Arc<File>,
    file_size: u64,
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
            line_index: LineIndex::new(LINES_PER_CHECKPOINT, BYTES_PER_CHECKPOINT),
            indexing: Indexing::Running,
        })
    }

    pub fn indexer(&self) -> Indexer {
        Indexer {
            path: self.path.clone(),
            file: Arc::clone(&self.file),
            file_size: self.file_size,
        }
    }

    pub fn set_line_index(&mut self, line_index: LineIndex) {
        self.line_index = line_index;
    }

    pub fn indexing(&self) -> Indexing {
        self.indexing
    }

    pub fn percent_indexed(&self) -> u64 {
        percent_indexed(self.line_index.bytes_indexed(), self.file_size)
    }

    pub fn set_indexing(&mut self, indexing: Indexing) {
        self.indexing = indexing;
    }

    pub fn file_name(&self) -> Cow<'_, str> {
        let file_name = self.path.file_name().unwrap_or(self.path.as_os_str());
        file_name.to_string_lossy()
    }

    pub fn line_count(&self) -> usize {
        self.line_index.line_count()
    }

    pub fn lines(&self, range: Range<usize>) -> Result<Vec<String>, LogFileError> {
        let range_end = range.end.min(self.line_count());
        if range.start >= range_end {
            return Ok(Vec::new());
        }
        let Some(checkpoint) = self.line_index.checkpoint_for(range.start) else {
            return Ok(Vec::new());
        };
        let mut line_collector =
            LineCollector::new(checkpoint.lines_to_skip, range_end - range.start);
        read_chunks(
            &self.file,
            checkpoint.byte_offset,
            READ_BUFFER_SIZE,
            |chunk| line_collector.add_chunk(chunk),
        )
        .map_err(|source| LogFileError::CannotRead {
            path: self.path.clone(),
            source,
        })?;
        Ok(line_collector.finish())
    }
}

impl Indexer {
    pub fn run(&self, on_progress: impl FnMut(LineIndex)) -> Result<(), LogFileError> {
        self.run_in_chunks(INDEX_CHUNK_SIZE, on_progress)
    }

    fn run_in_chunks(
        &self,
        chunk_size: usize,
        mut on_progress: impl FnMut(LineIndex),
    ) -> Result<(), LogFileError> {
        let mut line_index = LineIndex::new(LINES_PER_CHECKPOINT, BYTES_PER_CHECKPOINT);
        let mut reported_percent = None;
        read_chunks(&self.file, 0, chunk_size, |chunk| {
            line_index.add_chunk(chunk);
            let percent = percent_indexed(line_index.bytes_indexed(), self.file_size);
            if reported_percent != Some(percent) {
                reported_percent = Some(percent);
                on_progress(line_index.clone());
            }
            ControlFlow::Continue(())
        })
        .map_err(|source| LogFileError::CannotRead {
            path: self.path.clone(),
            source,
        })?;
        on_progress(line_index);
        Ok(())
    }
}

fn percent_indexed(bytes_indexed: u64, file_size: u64) -> u64 {
    if file_size == 0 {
        return 100;
    }
    (bytes_indexed * 100 / file_size).min(100)
}

fn read_chunks(
    file: &File,
    mut offset: u64,
    chunk_size: usize,
    mut on_chunk: impl FnMut(&[u8]) -> ControlFlow<()>,
) -> io::Result<()> {
    let mut buffer = vec![0; chunk_size];
    loop {
        let bytes_read = file.read_at(&mut buffer, offset)?;
        if bytes_read == 0 || on_chunk(&buffer[..bytes_read]).is_break() {
            return Ok(());
        }
        offset += bytes_read as u64;
    }
}

/// Splits consecutive chunks into lines, skipping the first `lines_to_skip` and keeping the next
/// `lines_to_keep`.
struct LineCollector {
    lines_to_skip: usize,
    lines_to_keep: usize,
    lines: Vec<String>,
    partial_line: Vec<u8>,
}

impl LineCollector {
    fn new(lines_to_skip: usize, lines_to_keep: usize) -> Self {
        Self {
            lines_to_skip,
            lines_to_keep,
            lines: Vec::with_capacity(lines_to_keep),
            partial_line: Vec::new(),
        }
    }

    fn add_chunk(&mut self, chunk: &[u8]) -> ControlFlow<()> {
        let mut line_start = 0;
        for newline_position in memchr::memchr_iter(b'\n', chunk) {
            self.push_line_part(&chunk[line_start..newline_position]);
            self.finish_line();
            if self.lines.len() == self.lines_to_keep {
                return ControlFlow::Break(());
            }
            line_start = newline_position + 1;
        }
        self.push_line_part(&chunk[line_start..]);
        ControlFlow::Continue(())
    }

    fn push_line_part(&mut self, line_part: &[u8]) {
        if self.lines_to_skip == 0 {
            self.partial_line.extend_from_slice(line_part);
        }
    }

    fn finish_line(&mut self) {
        if self.lines_to_skip > 0 {
            self.lines_to_skip -= 1;
        } else {
            self.lines.push(to_text(&self.partial_line));
        }
        self.partial_line.clear();
    }

    fn finish(mut self) -> Vec<String> {
        if !self.partial_line.is_empty() && self.lines.len() < self.lines_to_keep {
            self.finish_line();
        }
        self.lines
    }
}

fn to_text(line: &[u8]) -> String {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    String::from_utf8_lossy(line).into_owned()
}

#[cfg(test)]
impl LogFile {
    pub fn index_synchronously(&mut self) -> Result<(), LogFileError> {
        self.indexer()
            .run(|line_index| self.line_index = line_index)?;
        self.indexing = Indexing::Done;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn open_unindexed_with(content: &[u8]) -> (NamedTempFile, LogFile) {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content).unwrap();
        let log_file = LogFile::open(temp_file.path()).unwrap();
        (temp_file, log_file)
    }

    fn open_with(content: &[u8]) -> (NamedTempFile, LogFile) {
        let (temp_file, mut log_file) = open_unindexed_with(content);
        log_file.index_synchronously().unwrap();
        (temp_file, log_file)
    }

    fn progress_in_chunks_of(log_file: &LogFile, chunk_size: usize) -> Vec<LineIndex> {
        let mut reported_indexes = Vec::new();
        log_file
            .indexer()
            .run_in_chunks(chunk_size, |line_index| reported_indexes.push(line_index))
            .unwrap();
        reported_indexes
    }

    fn lines_text(line_count: usize) -> String {
        (0..line_count)
            .map(|line_number| format!("line {line_number}\r\n"))
            .collect()
    }

    #[test]
    fn given_opened_file_when_not_indexed_yet_then_it_has_no_lines_and_indexing_runs() {
        let (_temp_file, log_file) = open_unindexed_with(b"a\nb\n");

        assert_eq!(log_file.line_count(), 0);
        assert!(log_file.lines(0..2).unwrap().is_empty());
        assert_eq!(log_file.indexing(), Indexing::Running);
    }

    #[test]
    fn given_index_from_small_chunks_when_set_then_every_line_reads_back() {
        let (_temp_file, mut log_file) = open_unindexed_with(lines_text(3000).as_bytes());
        let final_index = progress_in_chunks_of(&log_file, 7).pop().unwrap();

        log_file.set_line_index(final_index);

        assert_eq!(log_file.line_count(), 3000);
        assert_eq!(
            log_file.lines(1999..2001).unwrap(),
            ["line 1999", "line 2000"]
        );
    }

    #[test]
    fn given_index_of_the_first_chunk_when_set_then_its_unfinished_line_reads_back_whole() {
        let (_temp_file, mut log_file) = open_unindexed_with(lines_text(3000).as_bytes());
        let first_index = progress_in_chunks_of(&log_file, 4).remove(0);

        log_file.set_line_index(first_index);

        assert_eq!(log_file.line_count(), 1);
        assert_eq!(log_file.lines(0..2).unwrap(), ["line 0"]);
    }

    #[test]
    fn given_first_tenth_of_the_file_indexed_when_asking_progress_then_it_is_10_percent() {
        let (_temp_file, mut log_file) = open_unindexed_with("x\n".repeat(50).as_bytes());
        let first_index = progress_in_chunks_of(&log_file, 10).remove(0);

        log_file.set_line_index(first_index);

        assert_eq!(log_file.percent_indexed(), 10);
    }

    #[test]
    fn given_file_read_in_one_byte_chunks_when_indexed_then_progress_is_reported_when_the_percentage_changes()
     {
        let (_temp_file, log_file) = open_unindexed_with("x\n".repeat(500).as_bytes());

        let reported_indexes = progress_in_chunks_of(&log_file, 1);

        let first_chunk_and_final_report = 2;
        assert_eq!(reported_indexes.len(), 100 + first_chunk_and_final_report);
    }

    #[test]
    fn given_empty_file_when_indexed_then_only_the_final_index_is_reported() {
        let (_temp_file, log_file) = open_unindexed_with(b"");

        let reported_indexes = progress_in_chunks_of(&log_file, 1);

        assert_eq!(reported_indexes.len(), 1);
        assert_eq!(reported_indexes[0].line_count(), 0);
    }

    #[test]
    fn given_unreadable_file_when_indexed_then_it_cannot_be_read() {
        let temp_file = NamedTempFile::new().unwrap();
        let indexer = Indexer {
            path: temp_file.path().to_path_buf(),
            file: Arc::new(File::create(temp_file.path()).unwrap()),
            file_size: 0,
        };

        let result = indexer.run(|_line_index| {});

        assert!(matches!(result, Err(LogFileError::CannotRead { .. })));
    }

    #[test]
    fn given_empty_file_when_computing_percent_indexed_then_it_is_complete() {
        assert_eq!(percent_indexed(0, 0), 100);
    }

    #[test]
    fn given_file_that_grew_while_indexing_when_computing_percent_then_it_stops_at_100() {
        assert_eq!(percent_indexed(150, 100), 100);
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
        assert!(log_file.lines(0..10).unwrap().is_empty());
    }

    #[test]
    fn given_range_past_the_end_when_reading_then_it_stops_at_the_last_line() {
        let (_temp_file, log_file) = open_with(b"a\nb\nc");

        assert_eq!(log_file.lines(1..10).unwrap(), ["b", "c"]);
        assert!(log_file.lines(3..10).unwrap().is_empty());
    }

    #[test]
    fn given_invalid_utf8_line_when_reading_then_it_is_replaced_and_numbering_holds() {
        let (_temp_file, log_file) = open_with(b"first\nbad \xff\nthird\n");

        assert_eq!(
            log_file.lines(0..3).unwrap(),
            ["first", "bad \u{fffd}", "third"]
        );
        assert_eq!(log_file.lines(2..3).unwrap(), ["third"]);
    }

    #[test]
    fn given_crlf_endings_when_reading_then_only_the_trailing_cr_is_stripped() {
        let (_temp_file, log_file) = open_with(b"a\r\nb\rc\r\n");

        assert_eq!(log_file.lines(0..2).unwrap(), ["a", "b\rc"]);
    }

    #[test]
    fn given_line_longer_than_read_buffer_when_reading_then_it_comes_back_whole() {
        let long_line = "x".repeat(READ_BUFFER_SIZE * 2 + 1);
        let (_temp_file, log_file) = open_with(format!("short\n{long_line}\nafter").as_bytes());

        assert_eq!(log_file.lines(1..3).unwrap(), [long_line.as_str(), "after"]);
    }

    #[test]
    fn given_skipped_line_longer_than_read_buffer_when_reading_then_the_next_line_comes_back_whole()
    {
        let long_line = "x".repeat(READ_BUFFER_SIZE * 2 + 1);
        let (_temp_file, log_file) = open_with(format!("{long_line}\nafter\nlast").as_bytes());

        assert_eq!(log_file.lines(1..3).unwrap(), ["after", "last"]);
    }

    #[test]
    fn given_file_larger_than_index_chunk_when_reading_then_lines_after_checkpoints_are_found() {
        let text: String = (0..200_000)
            .map(|line_number| format!("line {line_number}\n"))
            .collect();
        assert!(text.len() > INDEX_CHUNK_SIZE);
        let (_temp_file, log_file) = open_with(text.as_bytes());

        assert_eq!(log_file.line_count(), 200_000);
        assert_eq!(
            log_file.lines(123_999..124_001).unwrap(),
            ["line 123999", "line 124000"]
        );
        assert_eq!(log_file.lines(199_999..200_000).unwrap(), ["line 199999"]);
    }

    #[test]
    fn given_lines_that_reach_the_byte_limit_first_when_reading_then_lines_after_checkpoints_are_found()
     {
        let line_body = "x".repeat(10_000);
        let text: String = (0..100)
            .map(|line_number| format!("{line_number} {line_body}\n"))
            .collect();
        let (_temp_file, log_file) = open_with(text.as_bytes());

        let lines = log_file.lines(50..52).unwrap();

        assert_eq!(
            lines,
            [format!("50 {line_body}"), format!("51 {line_body}")]
        );
    }

    #[test]
    fn given_file_truncated_after_opening_when_reading_then_fewer_lines_return() {
        let (temp_file, log_file) = open_with(b"a\nb\nc\n");

        temp_file.as_file().set_len(3).unwrap();

        assert_eq!(log_file.lines(0..3).unwrap(), ["a", "b"]);
        assert!(log_file.lines(2..3).unwrap().is_empty());
    }
}
