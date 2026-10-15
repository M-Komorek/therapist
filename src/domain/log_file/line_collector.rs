use std::ops::ControlFlow;

pub(super) const KEPT_BYTES_PER_LINE: usize = 64 * 1024;

pub(super) struct LineCollector {
    lines_to_skip: usize,
    lines_to_keep: usize,
    lines: Vec<String>,
    partial_line: Vec<u8>,
}

impl LineCollector {
    pub(super) fn new(lines_to_skip: usize, lines_to_keep: usize) -> Self {
        Self {
            lines_to_skip,
            lines_to_keep,
            lines: Vec::with_capacity(lines_to_keep),
            partial_line: Vec::new(),
        }
    }

    pub(super) fn add_chunk(&mut self, chunk: &[u8]) -> ControlFlow<()> {
        if self.is_full() {
            return ControlFlow::Break(());
        }

        let mut line_start = 0;
        for newline_position in memchr::memchr_iter(b'\n', chunk) {
            self.push_line_part(&chunk[line_start..newline_position]);
            self.finish_line();
            if self.is_full() {
                return ControlFlow::Break(());
            }
            line_start = newline_position + 1;
        }
        self.push_line_part(&chunk[line_start..]);
        if self.is_last_line_to_keep_at_the_cap() {
            self.finish_line();
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    pub(super) fn finish(mut self) -> Vec<String> {
        if !self.partial_line.is_empty() && !self.is_full() {
            self.finish_line();
        }
        self.lines
    }

    fn is_full(&self) -> bool {
        self.lines.len() >= self.lines_to_keep
    }

    fn is_last_line_to_keep_at_the_cap(&self) -> bool {
        self.lines.len() + 1 == self.lines_to_keep && self.partial_line.len() == KEPT_BYTES_PER_LINE
    }

    fn push_line_part(&mut self, line_part: &[u8]) {
        if self.lines_to_skip > 0 {
            return;
        }

        let room_in_partial_line = KEPT_BYTES_PER_LINE - self.partial_line.len();
        let bytes_to_keep = line_part.len().min(room_in_partial_line);
        self.partial_line
            .extend_from_slice(&line_part[..bytes_to_keep]);
    }

    fn finish_line(&mut self) {
        if self.lines_to_skip > 0 {
            self.lines_to_skip -= 1;
        } else {
            self.lines.push(decode_line(&self.partial_line));
        }
        self.partial_line.clear();
    }
}

fn decode_line(line: &[u8]) -> String {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    String::from_utf8_lossy(line).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(lines_to_skip: usize, lines_to_keep: usize, chunks: &[&[u8]]) -> Vec<String> {
        let mut line_collector = LineCollector::new(lines_to_skip, lines_to_keep);
        for chunk in chunks {
            if line_collector.add_chunk(chunk).is_break() {
                break;
            }
        }
        line_collector.finish()
    }

    fn kept_text() -> String {
        "x".repeat(KEPT_BYTES_PER_LINE)
    }

    #[test]
    fn given_no_chunks_when_finishing_then_there_are_no_lines() {
        assert!(collect(0, 3, &[]).is_empty());
    }

    #[test]
    fn given_lines_to_skip_when_collecting_then_they_are_left_out() {
        assert_eq!(collect(1, 2, &[b"a\nb\nc\n"]), ["b", "c"]);
    }

    #[test]
    fn given_keep_limit_reached_when_adding_a_chunk_then_it_asks_to_stop() {
        let mut line_collector = LineCollector::new(0, 1);

        let flow = line_collector.add_chunk(b"a\nb\n");

        assert!(flow.is_break());
        assert_eq!(line_collector.finish(), ["a"]);
    }

    #[test]
    fn given_no_lines_to_keep_when_adding_a_chunk_then_it_asks_to_stop_and_keeps_nothing() {
        let mut line_collector = LineCollector::new(0, 0);

        let flow = line_collector.add_chunk(b"a\nb\n");

        assert!(flow.is_break());
        assert!(line_collector.finish().is_empty());
    }

    #[test]
    fn given_line_split_across_chunks_when_collecting_then_it_comes_back_whole() {
        assert_eq!(collect(0, 2, &[b"ab", b"c\nd", b"e\n"]), ["abc", "de"]);
    }

    #[test]
    fn given_skipped_line_split_across_chunks_when_collecting_then_the_next_line_is_whole() {
        assert_eq!(collect(1, 1, &[b"xx", b"x\nab", b"c\n"]), ["abc"]);
    }

    #[test]
    fn given_unterminated_last_line_when_finishing_then_it_is_kept() {
        assert_eq!(collect(0, 5, &[b"a\nb"]), ["a", "b"]);
    }

    #[test]
    fn given_cr_before_the_newline_in_the_next_chunk_when_collecting_then_the_cr_is_stripped() {
        assert_eq!(collect(0, 2, &[b"a\r", b"\nb\r\n"]), ["a", "b"]);
    }

    #[test]
    fn given_line_longer_than_the_kept_bytes_when_collecting_then_its_start_comes_back_and_the_next_line_is_whole()
     {
        let long_line = [b'x'; 70_000];

        let lines = collect(0, 2, &[&long_line, b"\nafter\n"]);

        assert_eq!(lines, [kept_text(), "after".to_owned()]);
    }

    #[test]
    fn given_last_line_to_keep_longer_than_the_kept_bytes_when_adding_its_chunk_then_it_asks_to_stop_before_its_end()
     {
        let mut line_collector = LineCollector::new(0, 1);

        let flow = line_collector.add_chunk(&[b'x'; 70_000]);

        assert!(flow.is_break());
        assert_eq!(line_collector.finish(), [kept_text()]);
    }

    #[test]
    fn given_unterminated_last_line_longer_than_the_kept_bytes_when_finishing_then_its_start_comes_back()
     {
        let long_line = [b'x'; 70_000];

        let lines = collect(0, 5, &[b"a\n", &long_line]);

        assert_eq!(lines, ["a".to_owned(), kept_text()]);
    }

    #[test]
    fn given_skipped_line_longer_than_the_kept_bytes_when_collecting_then_the_next_line_is_whole() {
        let long_line = [b'x'; 70_000];

        assert_eq!(collect(1, 1, &[&long_line, b"\nafter\n"]), ["after"]);
    }
}
