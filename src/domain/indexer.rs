#[derive(Debug)]
pub struct SparseLineIndexer {
    sparse_offsets: Vec<usize>,
    sparse_factor: usize,
    total_lines: usize,
}

impl SparseLineIndexer {
    pub const DEFAULT_SPARSE_FACTOR: usize = 1000;

    pub fn build(data: &[u8]) -> Self {
        Self::build_with_factor(data, Self::DEFAULT_SPARSE_FACTOR)
    }

    pub fn build_with_factor(data: &[u8], sparse_factor: usize) -> Self {
        let sparse_factor = sparse_factor.max(1);
        let mut sparse_offsets = vec![0];
        let mut line_count = 0;
        let mut lines_since_last_index = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte == b'\n' {
                line_count += 1;
                lines_since_last_index += 1;

                if lines_since_last_index >= sparse_factor {
                    sparse_offsets.push(i + 1);
                    lines_since_last_index = 0;
                }
            }
        }

        if !data.is_empty() && data[data.len() - 1] != b'\n' {
            line_count += 1;
        }

        Self {
            sparse_offsets,
            sparse_factor,
            total_lines: line_count,
        }
    }

    #[allow(dead_code)]
    pub fn sparse_factor(&self) -> usize {
        self.sparse_factor
    }

    #[allow(dead_code)]
    pub fn indexed_count(&self) -> usize {
        self.sparse_offsets.len()
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    // TO_CONSIDER
    // Is it a good idea to fail here if the from_utf8 fails?
    #[allow(dead_code)]
    pub fn get_line<'a>(&self, data: &'a [u8], line_num: usize) -> Option<&'a str> {
        if line_num >= self.total_lines {
            return None;
        }

        let line_start = self.find_line_start(data, line_num)?;
        let line_end = self.find_line_end(data, line_start);

        std::str::from_utf8(&data[line_start..line_end]).ok()
    }

    pub fn get_lines<'a>(&self, data: &'a [u8], start: usize, end: usize) -> Vec<&'a str> {
        let Some(mut offset) = self.find_line_start(data, start) else {
            return vec![];
        };

        let mut out = Vec::with_capacity(end - start);
        for _ in start..end.min(self.total_lines) {
            let line_end = self.find_line_end(data, offset);
            if let Ok(s) = std::str::from_utf8(&data[offset..line_end]) {
                out.push(s);
            }
            offset = line_end; // or line_end without trailing \n skip
        }
        out
    }

    #[allow(dead_code)]
    pub fn line_offset(&self, data: &[u8], line_num: usize) -> Option<usize> {
        if line_num >= self.total_lines {
            return None;
        }

        self.find_line_start(data, line_num)
    }

    pub fn memory_usage(&self) -> usize {
        self.sparse_offsets.len() * std::mem::size_of::<usize>() + std::mem::size_of::<Self>()
    }

    fn find_line_start(&self, data: &[u8], line_num: usize) -> Option<usize> {
        let sparse_index = line_num / self.sparse_factor;
        let start_offset = *self.sparse_offsets.get(sparse_index)?;
        let lines_to_skip = line_num % self.sparse_factor;

        if lines_to_skip == 0 {
            return Some(start_offset);
        }

        Some(self.scan_forward(data, start_offset, lines_to_skip))
    }

    fn scan_forward(&self, data: &[u8], start_offset: usize, lines_to_skip: usize) -> usize {
        let mut current_offset = start_offset;
        let mut lines_skipped = 0;

        while lines_skipped < lines_to_skip && current_offset < data.len() {
            while current_offset < data.len() && data[current_offset] != b'\n' {
                current_offset += 1;
            }
            current_offset += 1;
            lines_skipped += 1;
        }

        current_offset
    }

    fn find_line_end(&self, data: &[u8], line_start: usize) -> usize {
        let mut line_end = line_start;

        while line_end < data.len() && data[line_end] != b'\n' {
            line_end += 1;
        }

        if line_end < data.len() && data[line_end] == b'\n' {
            line_end += 1;
        }

        line_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_data_when_built_then_total_lines_is_zero() {
        let data = b"";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 0);
        assert_eq!(index.get_line(data, 0), None);
    }

    #[test]
    fn given_single_line_with_newline_when_built_then_line_count_is_one() {
        let data = b"Hello\n";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 1);
        assert_eq!(index.get_line(data, 0), Some("Hello\n"));
        assert_eq!(index.get_line(data, 1), None);
    }

    #[test]
    fn given_single_line_without_newline_when_built_then_line_count_is_one() {
        let data = b"Hello";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 1);
        assert_eq!(index.get_line(data, 0), Some("Hello"));
        assert_eq!(index.get_line(data, 1), None);
    }

    #[test]
    fn given_multiple_lines_with_trailing_newline_when_built_then_all_lines_accessible() {
        let data = b"Line 1\nLine 2\nLine 3\n";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 3);
        assert_eq!(index.get_line(data, 0), Some("Line 1\n"));
        assert_eq!(index.get_line(data, 1), Some("Line 2\n"));
        assert_eq!(index.get_line(data, 2), Some("Line 3\n"));
        assert_eq!(index.get_line(data, 3), None);
    }

    #[test]
    fn given_multiple_lines_without_trailing_newline_when_built_then_all_lines_accessible() {
        let data = b"Line 1\nLine 2\nLine 3";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 3);
        assert_eq!(index.get_line(data, 0), Some("Line 1\n"));
        assert_eq!(index.get_line(data, 1), Some("Line 2\n"));
        assert_eq!(index.get_line(data, 2), Some("Line 3"));
        assert_eq!(index.get_line(data, 3), None);
    }

    #[test]
    fn given_empty_lines_in_middle_when_get_line_then_returns_bare_newline() {
        let data = b"Line 1\n\nLine 3\n";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 3);
        assert_eq!(index.get_line(data, 0), Some("Line 1\n"));
        assert_eq!(index.get_line(data, 1), Some("\n"));
        assert_eq!(index.get_line(data, 2), Some("Line 3\n"));
    }

    #[test]
    fn given_only_newlines_when_built_then_each_line_is_newline_char() {
        let data = b"\n\n\n";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 3);
        assert_eq!(index.get_line(data, 0), Some("\n"));
        assert_eq!(index.get_line(data, 1), Some("\n"));
        assert_eq!(index.get_line(data, 2), Some("\n"));
    }

    #[test]
    fn given_multiline_data_when_get_lines_with_range_then_returns_slice() {
        let data = b"Line 1\nLine 2\nLine 3\nLine 4\n";
        let index = SparseLineIndexer::build(data);

        let lines = index.get_lines(data, 1, 3);
        assert_eq!(lines, vec!["Line 2\n", "Line 3\n"]);
    }

    #[test]
    fn given_multiline_data_when_get_lines_end_exceeds_total_then_clamps_to_available() {
        let data = b"Line 1\nLine 2\n";
        let index = SparseLineIndexer::build(data);

        let lines = index.get_lines(data, 0, 10);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn given_small_sparse_factor_when_built_then_all_lines_still_accessible() {
        let data =
            b"Line 0\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\n";
        let index = SparseLineIndexer::build_with_factor(data, 3);

        assert_eq!(index.total_lines(), 10);
        assert_eq!(index.sparse_factor(), 3);
        assert!(index.indexed_count() <= 5);

        assert_eq!(index.get_line(data, 0), Some("Line 0\n"));
        assert_eq!(index.get_line(data, 1), Some("Line 1\n"));
        assert_eq!(index.get_line(data, 4), Some("Line 4\n"));
        assert_eq!(index.get_line(data, 9), Some("Line 9\n"));
    }

    #[test]
    fn given_large_factor_when_built_then_memory_usage_is_fraction_of_full_index() {
        let mut data = Vec::new();
        for i in 0..10_000 {
            data.extend_from_slice(format!("Line {}\n", i).as_bytes());
        }

        let full_index = SparseLineIndexer::build_with_factor(&data, 1);
        let sparse_index = SparseLineIndexer::build_with_factor(&data, 1000);

        assert!(sparse_index.memory_usage() < full_index.memory_usage() / 10);
        assert_eq!(full_index.total_lines(), 10_000);
        assert_eq!(sparse_index.total_lines(), 10_000);
    }

    #[test]
    fn given_large_file_when_get_line_at_arbitrary_positions_then_content_is_correct() {
        let mut data = Vec::new();
        for i in 0..10_000 {
            data.extend_from_slice(format!("This is line number {}\n", i).as_bytes());
        }

        let index = SparseLineIndexer::build(&data);

        assert_eq!(index.total_lines(), 10_000);
        assert!(
            index
                .get_line(&data, 0)
                .unwrap()
                .starts_with("This is line number 0")
        );
        assert!(
            index
                .get_line(&data, 5000)
                .unwrap()
                .starts_with("This is line number 5000")
        );
        assert!(
            index
                .get_line(&data, 9999)
                .unwrap()
                .starts_with("This is line number 9999")
        );
        assert_eq!(index.get_line(&data, 10_000), None);
    }

    #[test]
    fn given_utf8_multibyte_content_when_get_line_then_returns_correct_str() {
        let data = "Hello 👋\nWorld 🌍\nRust 🦀\n".as_bytes();
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.total_lines(), 3);
        assert_eq!(index.get_line(data, 0), Some("Hello 👋\n"));
        assert_eq!(index.get_line(data, 1), Some("World 🌍\n"));
        assert_eq!(index.get_line(data, 2), Some("Rust 🦀\n"));
    }

    #[test]
    fn given_crlf_line_endings_when_get_line_then_cr_is_included_in_content() {
        let data = b"Line 1\r\nLine 2\r\nLine 3\r\n";
        let index = SparseLineIndexer::build(data);

        assert_eq!(index.get_line(data, 0), Some("Line 1\r\n"));
        assert_eq!(index.get_line(data, 1), Some("Line 2\r\n"));
        assert_eq!(index.get_line(data, 2), Some("Line 3\r\n"));
    }

    #[test]
    fn given_sparse_factor_zero_when_built_then_factor_is_clamped_to_one() {
        let data = b"A\nB\nC\nD\nE\n";
        let index = SparseLineIndexer::build_with_factor(data, 0);

        assert_eq!(index.sparse_factor(), 1);
        assert_eq!(index.get_line(data, 2), Some("C\n"));
    }

    #[test]
    fn given_sparse_factor_exceeds_line_count_when_built_then_all_lines_accessible() {
        let data = b"A\nB\nC\nD\nE\n";
        let index = SparseLineIndexer::build_with_factor(data, 10_000);

        assert_eq!(index.total_lines(), 5);
        assert_eq!(index.get_line(data, 0), Some("A\n"));
        assert_eq!(index.get_line(data, 4), Some("E\n"));
    }

    #[test]
    fn given_large_file_when_get_lines_viewport_pattern_then_completes_within_10ms() {
        use std::time::Instant;

        let mut data = Vec::new();
        for i in 0..100_000 {
            data.extend_from_slice(format!("Line {}\n", i).as_bytes());
        }

        let index = SparseLineIndexer::build(&data);
        let viewport_size = 50;

        let start = Instant::now();
        for viewport_start in (0..100_000).step_by(5000) {
            let lines = index.get_lines(&data, viewport_start, viewport_start + viewport_size);
            assert_eq!(lines.len(), viewport_size);
        }
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 10,
            "Viewport rendering took {}ms, expected < 10ms",
            duration.as_millis()
        );
    }

    #[test]
    fn given_large_file_with_factor_1000_when_random_get_line_then_completes_within_50ms() {
        use std::time::Instant;

        let mut data = Vec::new();
        for i in 0..10_000 {
            data.extend_from_slice(format!("Line {}\n", i).as_bytes());
        }

        let index = SparseLineIndexer::build_with_factor(&data, 1000);

        let start = Instant::now();
        for i in (0..10_000).step_by(100) {
            let _ = index.get_line(&data, i);
        }
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 50,
            "100 random accesses took {}ms, expected < 50ms",
            duration.as_millis()
        );
    }

    #[test]
    fn given_large_file_with_factor_1000_when_sequential_get_line_then_completes_within_200ms() {
        use std::time::Instant;

        let mut data = Vec::new();
        for i in 0..5000 {
            data.extend_from_slice(format!("Line {}\n", i).as_bytes());
        }

        let index = SparseLineIndexer::build_with_factor(&data, 1000);

        let start = Instant::now();
        for i in 0..5000 {
            let _ = index.get_line(&data, i);
        }
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 200,
            "Sequential access took {}ms, expected < 200ms",
            duration.as_millis()
        );
    }
}
