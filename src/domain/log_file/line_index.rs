const LINES_PER_CHECKPOINT: usize = 1000;
const BYTES_PER_CHECKPOINT: u64 = 64 * 1024;

/// Checkpoints (line starts) built from consecutive chunks of a file. A new checkpoint is the
/// first line start `lines_per_checkpoint` lines or `bytes_per_checkpoint` bytes past the previous
/// one, whichever comes first.
#[derive(Debug, PartialEq, Clone)]
pub struct LineIndex {
    lines_per_checkpoint: usize,
    bytes_per_checkpoint: u64,
    checkpoints: Vec<Checkpoint>,
    newline_count: usize,
    bytes_indexed: u64,
    ends_with_newline: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Checkpoint {
    pub line: usize,
    pub byte_offset: u64,
}

impl Default for LineIndex {
    fn default() -> Self {
        Self::new(LINES_PER_CHECKPOINT, BYTES_PER_CHECKPOINT)
    }
}

impl LineIndex {
    pub fn new(lines_per_checkpoint: usize, bytes_per_checkpoint: u64) -> Self {
        Self {
            lines_per_checkpoint,
            bytes_per_checkpoint,
            checkpoints: vec![Checkpoint {
                line: 0,
                byte_offset: 0,
            }],
            newline_count: 0,
            bytes_indexed: 0,
            ends_with_newline: false,
        }
    }

    pub fn add_chunk(&mut self, chunk: &[u8]) {
        for newline_position in memchr::memchr_iter(b'\n', chunk) {
            self.newline_count += 1;
            let line_start = Checkpoint {
                line: self.newline_count,
                byte_offset: self.bytes_indexed + newline_position as u64 + 1,
            };
            if self.is_checkpoint_due(&line_start) {
                self.checkpoints.push(line_start);
            }
        }
        if let Some(&last_byte) = chunk.last() {
            self.ends_with_newline = last_byte == b'\n';
        }
        self.bytes_indexed += chunk.len() as u64;
    }

    pub fn percent_indexed(&self, file_size: u64) -> u64 {
        if file_size == 0 {
            return 100;
        }
        (self.bytes_indexed * 100 / file_size).min(100)
    }

    pub fn line_count(&self) -> usize {
        let has_unterminated_last_line = self.bytes_indexed > 0 && !self.ends_with_newline;
        self.newline_count + usize::from(has_unterminated_last_line)
    }

    pub fn checkpoint_for(&self, line: usize) -> Option<&Checkpoint> {
        if line >= self.line_count() {
            return None;
        }
        let checkpoints_up_to_line = self
            .checkpoints
            .partition_point(|checkpoint| checkpoint.line <= line);
        self.checkpoints.get(checkpoints_up_to_line.checked_sub(1)?)
    }

    fn is_checkpoint_due(&self, line_start: &Checkpoint) -> bool {
        self.checkpoints.last().is_none_or(|last_checkpoint| {
            line_start.line - last_checkpoint.line >= self.lines_per_checkpoint
                || line_start.byte_offset - last_checkpoint.byte_offset >= self.bytes_per_checkpoint
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(text: &str, lines_per_checkpoint: usize) -> LineIndex {
        index_with_limits(text, lines_per_checkpoint, u64::MAX)
    }

    fn index_with_limits(
        text: &str,
        lines_per_checkpoint: usize,
        bytes_per_checkpoint: u64,
    ) -> LineIndex {
        let mut line_index = LineIndex::new(lines_per_checkpoint, bytes_per_checkpoint);
        line_index.add_chunk(text.as_bytes());
        line_index
    }

    fn lookup(line_index: &LineIndex, line: usize) -> Option<(u64, usize)> {
        line_index
            .checkpoint_for(line)
            .map(|checkpoint| (checkpoint.byte_offset, line - checkpoint.line))
    }

    #[test]
    fn given_empty_file_when_computing_percent_indexed_then_it_is_complete() {
        assert_eq!(LineIndex::default().percent_indexed(0), 100);
    }

    #[test]
    fn given_file_that_grew_while_indexing_when_computing_percent_then_it_stops_at_100() {
        let mut line_index = LineIndex::default();
        line_index.add_chunk(&[b'x'; 150]);

        assert_eq!(line_index.percent_indexed(100), 100);
    }

    #[test]
    fn given_empty_file_when_indexed_then_it_has_no_lines() {
        let line_index = index_of("", 1);

        assert_eq!(line_index.line_count(), 0);
        assert_eq!(lookup(&line_index, 0), None);
    }

    #[test]
    fn given_trailing_newline_when_indexed_then_no_empty_line_follows_it() {
        assert_eq!(index_of("a\nb\n", 1).line_count(), 2);
    }

    #[test]
    fn given_no_trailing_newline_when_indexed_then_last_line_is_counted() {
        assert_eq!(index_of("a\nb", 1).line_count(), 2);
    }

    #[test]
    fn given_only_newlines_when_indexed_then_each_is_an_empty_line() {
        let line_index = index_of("\n\n\n", 1);

        assert_eq!(line_index.line_count(), 3);
        assert_eq!(lookup(&line_index, 2), Some((2, 0)));
    }

    #[test]
    fn given_crlf_endings_when_indexed_then_lines_start_after_the_newline() {
        let line_index = index_of("a\r\nbb\r\nc", 1);

        assert_eq!(line_index.line_count(), 3);
        assert_eq!(lookup(&line_index, 1), Some((3, 0)));
        assert_eq!(lookup(&line_index, 2), Some((7, 0)));
    }

    #[test]
    fn given_line_limit_of_one_when_looking_up_then_every_line_is_a_checkpoint() {
        let line_index = index_of("a\nbb\nccc", 1);

        assert_eq!(lookup(&line_index, 0), Some((0, 0)));
        assert_eq!(lookup(&line_index, 1), Some((2, 0)));
        assert_eq!(lookup(&line_index, 2), Some((5, 0)));
    }

    #[test]
    fn given_line_limit_of_two_when_looking_up_then_nearest_earlier_checkpoint_is_used() {
        let line_index = index_of("a\nbb\nccc\ndddd\ne", 2);

        assert_eq!(lookup(&line_index, 1), Some((0, 1)));
        assert_eq!(lookup(&line_index, 3), Some((5, 1)));
        assert_eq!(lookup(&line_index, 4), Some((14, 0)));
    }

    #[test]
    fn given_line_limit_larger_than_file_when_looking_up_then_lines_are_skipped_from_start() {
        let line_index = index_of("a\nbb\nccc", 10);

        assert_eq!(lookup(&line_index, 2), Some((0, 2)));
    }

    #[test]
    fn given_line_limit_of_zero_when_looking_up_then_it_works_like_a_limit_of_one() {
        let line_index = index_of("a\nbb", 0);

        assert_eq!(lookup(&line_index, 1), Some((2, 0)));
    }

    #[test]
    fn given_byte_limit_when_looking_up_then_checkpoint_is_first_line_start_that_many_bytes_on() {
        let line_index = index_with_limits("aaaa\nbb\ncc\nd", usize::MAX, 5);

        assert_eq!(lookup(&line_index, 1), Some((5, 0)));
        assert_eq!(lookup(&line_index, 2), Some((5, 1)));
        assert_eq!(lookup(&line_index, 3), Some((11, 0)));
    }

    #[test]
    fn given_lines_longer_than_byte_limit_when_looking_up_then_every_line_is_a_checkpoint() {
        let line_index = index_with_limits("aaaaaa\nbbbbbb\nc", usize::MAX, 4);

        assert_eq!(lookup(&line_index, 1), Some((7, 0)));
        assert_eq!(lookup(&line_index, 2), Some((14, 0)));
    }

    #[test]
    fn given_both_limits_when_looking_up_then_whichever_is_reached_first_places_the_checkpoint() {
        let line_index = index_with_limits("a\nb\nc\ndddddddd\ne\nf\ng", 3, 6);

        assert_eq!(lookup(&line_index, 2), Some((0, 2)));
        assert_eq!(lookup(&line_index, 3), Some((6, 0)));
        assert_eq!(lookup(&line_index, 4), Some((15, 0)));
        assert_eq!(lookup(&line_index, 6), Some((15, 2)));
    }

    #[test]
    fn given_line_past_the_end_when_looking_up_then_there_is_no_checkpoint() {
        let line_index = index_of("a\nb\n", 1);

        assert_eq!(lookup(&line_index, 2), None);
    }

    #[test]
    fn given_text_split_into_two_chunks_anywhere_when_indexed_then_it_matches_one_chunk() {
        let text = "a\r\nbb\n\nccc\r\nd";
        for (lines_per_checkpoint, bytes_per_checkpoint) in [(2, u64::MAX), (usize::MAX, 4)] {
            let indexed_in_one_go =
                index_with_limits(text, lines_per_checkpoint, bytes_per_checkpoint);

            for split_at in 0..=text.len() {
                let (first_chunk, second_chunk) = text.as_bytes().split_at(split_at);
                let mut indexed_in_chunks =
                    LineIndex::new(lines_per_checkpoint, bytes_per_checkpoint);
                indexed_in_chunks.add_chunk(first_chunk);
                indexed_in_chunks.add_chunk(second_chunk);

                assert_eq!(
                    indexed_in_chunks, indexed_in_one_go,
                    "split at {split_at} with limits {lines_per_checkpoint} lines, {bytes_per_checkpoint} bytes"
                );
            }
        }
    }
}
