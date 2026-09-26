/// The byte offset of every `checkpoint_interval`-th line start, built from consecutive chunks of
/// a file.
#[derive(Debug, PartialEq)]
pub struct LineIndex {
    checkpoint_interval: usize,
    checkpoints: Vec<u64>,
    newline_count: usize,
    bytes_indexed: u64,
    ends_with_newline: bool,
}

#[derive(Debug, PartialEq)]
pub struct Checkpoint {
    pub byte_offset: u64,
    pub lines_to_skip: usize,
}

impl LineIndex {
    pub fn new(checkpoint_interval: usize) -> Self {
        Self {
            checkpoint_interval: checkpoint_interval.max(1),
            checkpoints: vec![0],
            newline_count: 0,
            bytes_indexed: 0,
            ends_with_newline: false,
        }
    }

    pub fn add_chunk(&mut self, chunk: &[u8]) {
        for newline_position in memchr::memchr_iter(b'\n', chunk) {
            self.newline_count += 1;
            if self.newline_count == self.checkpoints.len() * self.checkpoint_interval {
                self.checkpoints
                    .push(self.bytes_indexed + newline_position as u64 + 1);
            }
        }
        if let Some(&last_byte) = chunk.last() {
            self.ends_with_newline = last_byte == b'\n';
        }
        self.bytes_indexed += chunk.len() as u64;
    }

    pub fn line_count(&self) -> usize {
        let has_unterminated_last_line = self.bytes_indexed > 0 && !self.ends_with_newline;
        self.newline_count + usize::from(has_unterminated_last_line)
    }

    pub fn checkpoint_for(&self, line: usize) -> Option<Checkpoint> {
        if line >= self.line_count() {
            return None;
        }
        let byte_offset = *self.checkpoints.get(line / self.checkpoint_interval)?;
        Some(Checkpoint {
            byte_offset,
            lines_to_skip: line % self.checkpoint_interval,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(text: &str, checkpoint_interval: usize) -> LineIndex {
        let mut line_index = LineIndex::new(checkpoint_interval);
        line_index.add_chunk(text.as_bytes());
        line_index
    }

    fn lookup(line_index: &LineIndex, line: usize) -> Option<(u64, usize)> {
        line_index
            .checkpoint_for(line)
            .map(|checkpoint| (checkpoint.byte_offset, checkpoint.lines_to_skip))
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
    fn given_interval_of_one_when_looking_up_then_every_line_is_a_checkpoint() {
        let line_index = index_of("a\nbb\nccc", 1);

        assert_eq!(lookup(&line_index, 0), Some((0, 0)));
        assert_eq!(lookup(&line_index, 1), Some((2, 0)));
        assert_eq!(lookup(&line_index, 2), Some((5, 0)));
    }

    #[test]
    fn given_interval_of_two_when_looking_up_then_nearest_earlier_checkpoint_is_used() {
        let line_index = index_of("a\nbb\nccc\ndddd\ne", 2);

        assert_eq!(lookup(&line_index, 1), Some((0, 1)));
        assert_eq!(lookup(&line_index, 3), Some((5, 1)));
        assert_eq!(lookup(&line_index, 4), Some((14, 0)));
    }

    #[test]
    fn given_interval_larger_than_file_when_looking_up_then_lines_are_skipped_from_start() {
        let line_index = index_of("a\nbb\nccc", 10);

        assert_eq!(lookup(&line_index, 2), Some((0, 2)));
    }

    #[test]
    fn given_zero_interval_when_looking_up_then_it_works_like_interval_of_one() {
        let line_index = index_of("a\nbb", 0);

        assert_eq!(lookup(&line_index, 1), Some((2, 0)));
    }

    #[test]
    fn given_line_past_the_end_when_looking_up_then_there_is_no_checkpoint() {
        let line_index = index_of("a\nb\n", 1);

        assert_eq!(lookup(&line_index, 2), None);
    }

    #[test]
    fn given_text_split_into_two_chunks_anywhere_when_indexed_then_it_matches_one_chunk() {
        let text = "a\r\nbb\n\nccc\r\nd";
        let indexed_in_one_go = index_of(text, 2);

        for split_at in 0..=text.len() {
            let (first_chunk, second_chunk) = text.as_bytes().split_at(split_at);
            let mut indexed_in_chunks = LineIndex::new(2);
            indexed_in_chunks.add_chunk(first_chunk);
            indexed_in_chunks.add_chunk(second_chunk);

            assert_eq!(indexed_in_chunks, indexed_in_one_go, "split at {split_at}");
        }
    }
}
