use std::path::Path;

use memmap2::Mmap;

use super::{indexer::SparseLineIndexer, source::LogSource};

pub struct RawLogs {
    mmap: Mmap,
    indexer: SparseLineIndexer,
}

impl RawLogs {
    pub fn open(path: &Path) -> color_eyre::Result<Self> {
        let file = std::fs::File::open(path).map_err(|error| {
            color_eyre::eyre::eyre!("Cannot open '{}': {}", path.display(), error)
        })?;

        let mmap = unsafe { Mmap::map(&file) }.map_err(|error| {
            color_eyre::eyre::eyre!("Cannot mmap '{}': {}", path.display(), error)
        })?;

        let indexer = SparseLineIndexer::build(&mmap);

        Ok(Self { mmap, indexer })
    }
}

impl LogSource for RawLogs {
    fn get_line(&self, line_idx: usize) -> color_eyre::Result<String> {
        self.indexer
            .get_line(&self.mmap, line_idx)
            .map(|s| s.trim_end_matches('\n').trim_end_matches('\r').to_owned())
            .ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "Line index {} out of bounds (total: {})",
                    line_idx,
                    self.indexer.total_lines()
                )
            })
    }

    fn get_lines(&self, start: usize, end: usize) -> Vec<String> {
        self.indexer
            .get_lines(&self.mmap, start, end)
            .into_iter()
            .map(|s| s.trim_end_matches('\n').trim_end_matches('\r').to_owned())
            .collect()
    }

    fn line_count(&self) -> usize {
        self.indexer.total_lines()
    }

    fn memory_usage(&self) -> usize {
        self.indexer.memory_usage()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn write_temp_file(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("failed to create temp file");
        file.write_all(content).expect("failed to write temp file");
        file
    }

    #[test]
    fn given_nonexistent_path_when_opened_then_returns_error() {
        let result = RawLogs::open(Path::new("/nonexistent/path/file.log"));
        assert!(result.is_err());
    }

    #[test]
    fn given_file_with_lines_when_opened_then_line_count_matches() {
        let file = write_temp_file(b"alpha\nbeta\ngamma\n");
        let source = RawLogs::open(file.path()).unwrap();
        assert_eq!(source.line_count(), 3);
    }

    #[test]
    fn given_file_with_content_when_get_line_then_returns_correct_line() {
        let file = write_temp_file(b"first\nsecond\nthird\n");
        let source = RawLogs::open(file.path()).unwrap();

        assert_eq!(source.get_line(0).unwrap(), "first");
        assert_eq!(source.get_line(1).unwrap(), "second");
        assert_eq!(source.get_line(2).unwrap(), "third");
    }

    #[test]
    fn given_file_when_get_line_out_of_bounds_then_returns_error() {
        let file = write_temp_file(b"only line\n");
        let source = RawLogs::open(file.path()).unwrap();

        assert!(source.get_line(1).is_err());
    }

    #[test]
    fn given_empty_file_when_opened_then_line_count_is_zero() {
        let file = write_temp_file(b"");
        let source = RawLogs::open(file.path()).unwrap();

        assert_eq!(source.line_count(), 0);
        assert!(source.get_line(0).is_err());
    }

    #[test]
    fn given_file_without_trailing_newline_when_opened_then_last_line_accessible() {
        let file = write_temp_file(b"line one\nline two");
        let source = RawLogs::open(file.path()).unwrap();

        assert_eq!(source.line_count(), 2);
        assert_eq!(source.get_line(1).unwrap(), "line two");
    }

    #[test]
    fn given_file_with_very_long_line_when_get_line_then_returns_correct_content() {
        let long_line = "x".repeat(100_000);
        let content = format!("before\n{long_line}\nafter\n");
        let file = write_temp_file(content.as_bytes());
        let source = RawLogs::open(file.path()).unwrap();

        assert_eq!(source.line_count(), 3);
        assert_eq!(source.get_line(1).unwrap(), long_line);
    }

    #[test]
    fn given_file_with_lines_when_get_lines_with_range_then_returns_slice() {
        let file = write_temp_file(b"alpha\nbeta\ngamma\ndelta\n");
        let source = RawLogs::open(file.path()).unwrap();

        let lines = source.get_lines(1, 3);
        assert_eq!(lines, vec!["beta", "gamma"]);
    }

    #[test]
    fn given_file_when_get_lines_end_exceeds_total_then_clamps_to_available() {
        let file = write_temp_file(b"one\ntwo\nthree\n");
        let source = RawLogs::open(file.path()).unwrap();

        let lines = source.get_lines(0, 100);
        assert_eq!(lines, vec!["one", "two", "three"]);
    }

    #[test]
    fn given_file_when_get_lines_start_equals_end_then_returns_empty() {
        let file = write_temp_file(b"one\ntwo\nthree\n");
        let source = RawLogs::open(file.path()).unwrap();

        let lines = source.get_lines(1, 1);
        assert!(lines.is_empty());
    }

    #[test]
    fn given_file_when_get_lines_start_out_of_bounds_then_returns_empty() {
        let file = write_temp_file(b"one\ntwo\n");
        let source = RawLogs::open(file.path()).unwrap();

        let lines = source.get_lines(10, 20);
        assert!(lines.is_empty());
    }

    #[test]
    fn given_file_with_crlf_endings_when_get_lines_then_strips_trailing_newlines() {
        let file = write_temp_file(b"foo\r\nbar\r\nbaz\r\n");
        let source = RawLogs::open(file.path()).unwrap();

        let lines = source.get_lines(0, 3);
        assert_eq!(lines, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn given_file_when_get_lines_full_range_then_matches_individual_get_line_calls() {
        let file = write_temp_file(b"line0\nline1\nline2\nline3\n");
        let source = RawLogs::open(file.path()).unwrap();

        let batch = source.get_lines(0, 4);
        for (i, line) in batch.iter().enumerate() {
            assert_eq!(line, &source.get_line(i).unwrap());
        }
    }

    #[test]
    fn given_empty_file_when_memory_usage_then_returns_nonzero_struct_overhead() {
        let file = write_temp_file(b"");
        let source = RawLogs::open(file.path()).unwrap();

        // Even with no lines the struct itself occupies memory.
        assert!(source.memory_usage() > 0);
    }

    #[test]
    fn given_file_with_lines_when_memory_usage_then_returns_nonzero() {
        let file = write_temp_file(b"alpha\nbeta\ngamma\n");
        let source = RawLogs::open(file.path()).unwrap();

        assert!(source.memory_usage() > 0);
    }

    #[test]
    fn given_larger_file_when_memory_usage_then_exceeds_smaller_file() {
        let small_file = write_temp_file(b"line\n");
        let small_source = RawLogs::open(small_file.path()).unwrap();

        // Build a file large enough to produce more sparse index entries.
        let many_lines: Vec<u8> = (0..10_000)
            .flat_map(|i| format!("log line {i}\n").into_bytes())
            .collect();
        let large_file = write_temp_file(&many_lines);
        let large_source = RawLogs::open(large_file.path()).unwrap();

        assert!(large_source.memory_usage() > small_source.memory_usage());
    }

    #[test]
    fn given_large_file_when_opened_then_build_and_access_complete_within_time_limits() {
        use std::time::Instant;

        let mut content = Vec::new();
        for i in 0..100_000 {
            content.extend_from_slice(format!("log line {i}\n").as_bytes());
        }
        let file = write_temp_file(&content);

        let build_start = Instant::now();
        let source = RawLogs::open(file.path()).unwrap();
        let build_duration = build_start.elapsed();

        assert_eq!(source.line_count(), 100_000);
        assert!(
            build_duration.as_millis() < 500,
            "Index build took {}ms, expected < 500ms",
            build_duration.as_millis()
        );

        let access_start = Instant::now();
        for i in (0..100_000).step_by(1000) {
            let line = source.get_line(i).unwrap();
            assert!(line.starts_with("log line"));
        }
        let access_duration = access_start.elapsed();

        assert!(
            access_duration.as_millis() < 50,
            "100 random accesses took {}ms, expected < 50ms",
            access_duration.as_millis()
        );
    }
}
