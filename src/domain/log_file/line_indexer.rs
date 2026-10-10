use std::{fs::File, ops::ControlFlow, path::PathBuf, sync::Arc};

use super::{LineIndex, LogFileError, read_chunks};

const INDEX_CHUNK_SIZE: usize = 1024 * 1024;

/// Builds the file's `LineIndex`. Each time another percent of the file is indexed, it reports a
/// copy of the index so far, and at the end the finished index. If the report callback returns
/// `Break`, it stops without the finished index.
pub struct LineIndexer {
    path: PathBuf,
    file: Arc<File>,
    file_size: u64,
}

impl LineIndexer {
    pub(super) fn new(path: PathBuf, file: Arc<File>, file_size: u64) -> Self {
        Self {
            path,
            file,
            file_size,
        }
    }

    pub fn run(
        &self,
        on_progress: impl FnMut(LineIndex) -> ControlFlow<()>,
    ) -> Result<(), LogFileError> {
        self.run_in_chunks(INDEX_CHUNK_SIZE, on_progress)
    }

    fn run_in_chunks(
        &self,
        chunk_size: usize,
        mut on_progress: impl FnMut(LineIndex) -> ControlFlow<()>,
    ) -> Result<(), LogFileError> {
        let mut line_index = LineIndex::default();
        let mut reported_percent = None;
        let mut progress_flow = ControlFlow::Continue(());

        read_chunks(&self.file, 0, chunk_size, |chunk| {
            line_index.add_chunk(chunk);
            let percent = line_index.percent_indexed(self.file_size);
            if reported_percent != Some(percent) {
                reported_percent = Some(percent);
                progress_flow = on_progress(line_index.clone());
            }
            progress_flow
        })
        .map_err(|source| LogFileError::CannotRead {
            path: self.path.clone(),
            source,
        })?;
        if progress_flow.is_continue() {
            let _ = on_progress(line_index);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::domain::log_file::{
        LogFile,
        tests::{open_unindexed_with, open_with},
    };

    fn progress_in_chunks_of(log_file: &LogFile, chunk_size: usize) -> Vec<LineIndex> {
        let mut reported_indexes = Vec::new();
        log_file
            .line_indexer()
            .run_in_chunks(chunk_size, |line_index| {
                reported_indexes.push(line_index);
                ControlFlow::Continue(())
            })
            .unwrap();
        reported_indexes
    }

    fn lines_text(line_count: usize) -> String {
        (0..line_count)
            .map(|line_number| format!("line {line_number}\r\n"))
            .collect()
    }

    #[test]
    fn given_index_from_small_chunks_when_set_then_every_line_reads_back() {
        let (_temp_file, mut log_file) = open_unindexed_with(lines_text(3000).as_bytes());
        let final_index = progress_in_chunks_of(&log_file, 7).pop().unwrap();

        log_file.set_line_index(final_index);

        assert_eq!(log_file.line_count(), 3000);
        assert_eq!(
            log_file.read_lines(1999..2001).unwrap(),
            ["line 1999", "line 2000"]
        );
    }

    #[test]
    fn given_index_of_the_first_chunk_when_set_then_its_unfinished_line_reads_back_whole() {
        let (_temp_file, mut log_file) = open_unindexed_with(lines_text(3000).as_bytes());
        let first_index = progress_in_chunks_of(&log_file, 4).remove(0);

        log_file.set_line_index(first_index);

        assert_eq!(log_file.line_count(), 1);
        assert_eq!(log_file.read_lines(0..2).unwrap(), ["line 0"]);
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
    fn given_progress_callback_that_breaks_when_indexing_then_it_stops_without_the_final_report() {
        let (_temp_file, log_file) = open_unindexed_with("x\n".repeat(500).as_bytes());
        let mut report_count = 0;

        log_file
            .line_indexer()
            .run_in_chunks(1, |_line_index| {
                report_count += 1;
                ControlFlow::Break(())
            })
            .unwrap();

        assert_eq!(report_count, 1);
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
        let line_indexer = LineIndexer {
            path: temp_file.path().to_path_buf(),
            file: Arc::new(File::create(temp_file.path()).unwrap()),
            file_size: 0,
        };

        let result = line_indexer.run(|_line_index| ControlFlow::Continue(()));

        assert!(matches!(result, Err(LogFileError::CannotRead { .. })));
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
            log_file.read_lines(123_999..124_001).unwrap(),
            ["line 123999", "line 124000"]
        );
        assert_eq!(
            log_file.read_lines(199_999..200_000).unwrap(),
            ["line 199999"]
        );
    }
}
