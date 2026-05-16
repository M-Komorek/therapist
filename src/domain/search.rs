use memchr::memmem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub phrase: String,
    pub matching_lines: Vec<usize>,
}

impl SearchResult {
    pub fn empty(phrase: impl Into<String>) -> Self {
        Self {
            phrase: phrase.into(),
            matching_lines: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.matching_lines.is_empty()
    }

    pub fn match_count(&self) -> usize {
        self.matching_lines.len()
    }
}

pub fn search(data: &[u8], phrase: &str) -> Option<SearchResult> {
    if phrase.is_empty() {
        return None;
    }

    let finder = memmem::Finder::new(phrase.as_bytes());
    let matching_lines = collect_matching_lines(data, &finder);

    Some(SearchResult {
        phrase: phrase.to_owned(),
        matching_lines,
    })
}

/// Walk `data` once, yielding the line number of every match.
/// Deduplicates multiple matches on the same line.
fn collect_matching_lines(data: &[u8], finder: &memmem::Finder) -> Vec<usize> {
    let mut matching_lines: Vec<usize> = Vec::new();
    let mut current_line: usize = 0;
    let mut scan_pos: usize = 0;

    for match_offset in finder.find_iter(data) {
        advance_to_offset(data, &mut scan_pos, match_offset, &mut current_line);
        push_if_new(&mut matching_lines, current_line);
    }

    matching_lines
}

/// Advance `scan_pos` to `target`, counting newlines and updating `current_line`.
/// Uses SIMD-accelerated `memchr_iter` instead of a scalar byte-by-byte walk.
/// After return, `scan_pos` equals `target`.
fn advance_to_offset(data: &[u8], scan_pos: &mut usize, target: usize, current_line: &mut usize) {
    *current_line += memchr::memchr_iter(b'\n', &data[*scan_pos..target]).count();
    *scan_pos = target;
}

/// Push `line` only if it differs from the last entry (dedup for same-line multi-match).
fn push_if_new(matching_lines: &mut Vec<usize>, line: usize) {
    if matching_lines.last() != Some(&line) {
        matching_lines.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn matching_lines(data: &[u8], phrase: &str) -> Vec<usize> {
        search(data, phrase).unwrap().matching_lines
    }

    // ── correctness ──────────────────────────────────────────────────────────

    #[test]
    fn given_empty_phrase_when_search_then_returns_none() {
        assert_eq!(search(b"hello\nworld\n", ""), None);
    }

    #[test]
    fn given_empty_data_when_search_then_returns_empty_result() {
        let result = search(b"", "foo").unwrap();
        assert!(result.is_empty());
        assert_eq!(result.match_count(), 0);
    }

    #[test]
    fn given_phrase_not_present_when_search_then_returns_empty_result() {
        assert!(matching_lines(b"hello\nworld\n", "xyz").is_empty());
    }

    #[test]
    fn given_phrase_on_first_line_when_search_then_returns_line_zero() {
        assert_eq!(matching_lines(b"foo bar\nbaz\n", "foo"), vec![0]);
    }

    #[test]
    fn given_phrase_on_last_line_when_search_then_returns_correct_index() {
        assert_eq!(matching_lines(b"aaa\nbbb\nccc\n", "ccc"), vec![2]);
    }

    #[test]
    fn given_phrase_on_multiple_lines_when_search_then_returns_all_indices() {
        assert_eq!(matching_lines(b"hit\nmiss\nhit\n", "hit"), vec![0, 2]);
    }

    #[test]
    fn given_phrase_appears_twice_on_same_line_when_search_then_line_appears_once() {
        assert_eq!(matching_lines(b"foo foo\nbar\n", "foo"), vec![0]);
    }

    #[test]
    fn given_phrase_in_middle_of_line_when_search_then_correct_line() {
        assert_eq!(matching_lines(b"abc\nxyzfoo\n", "foo"), vec![1]);
    }

    #[test]
    fn given_phrase_is_entire_line_when_search_then_correct_line() {
        assert_eq!(matching_lines(b"exact\nother\n", "exact"), vec![0]);
    }

    #[test]
    fn given_phrase_at_end_no_trailing_newline_when_search_then_correct_line() {
        assert_eq!(matching_lines(b"line1\nlast", "last"), vec![1]);
    }

    #[test]
    fn given_phrase_matching_all_lines_when_search_then_all_indices_returned() {
        let data = b"x\nx\nx\nx\nx\n";
        assert_eq!(matching_lines(data, "x"), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn given_case_sensitive_search_when_phrase_differs_in_case_then_no_match() {
        assert!(matching_lines(b"Hello\n", "hello").is_empty());
    }

    #[test]
    fn given_multibyte_utf8_phrase_when_search_then_correct_line() {
        let data = "café\nbar\n".as_bytes();
        assert_eq!(matching_lines(data, "café"), vec![0]);
    }

    #[test]
    fn given_result_phrase_field_when_search_then_matches_input_phrase() {
        let result = search(b"hello\n", "hello").unwrap();
        assert_eq!(result.phrase, "hello");
    }

    #[test]
    fn given_crlf_line_endings_when_search_then_correct_line_numbers() {
        // \r is not a line separator in our model — only \n is.
        // "bar" starts after the \r\n of line 0, so it is on line 1.
        assert_eq!(matching_lines(b"foo\r\nbar\r\n", "bar"), vec![1]);
    }

    // ── edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn given_phrase_longer_than_data_when_search_then_no_match() {
        assert!(matching_lines(b"hi", "hello world").is_empty());
    }

    #[test]
    fn given_single_byte_phrase_when_search_then_finds_all_occurrences() {
        assert_eq!(matching_lines(b"a\nb\na\n", "a"), vec![0, 2]);
    }

    #[test]
    fn given_phrase_equal_to_entire_data_when_search_then_returns_line_zero() {
        assert_eq!(matching_lines(b"hello", "hello"), vec![0]);
    }

    #[test]
    fn given_phrase_at_byte_offset_zero_when_search_then_line_zero() {
        assert_eq!(matching_lines(b"match rest\nother\n", "match"), vec![0]);
    }

    #[test]
    fn given_newline_as_phrase_when_search_then_matches_line_boundaries() {
        // "\n" is a valid non-empty phrase. Each \n byte is at the end of a line.
        // data: "a\nb\nc\n" — \n at offsets 1, 3, 5.
        // offset 1 → line 0 (scan sees 0 newlines before offset 1, then hits it)
        // offset 3 → line 1, offset 5 → line 2
        let result = matching_lines(b"a\nb\nc\n", "\n");
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn given_search_result_empty_constructor_when_called_then_phrase_set_and_lines_empty() {
        let r = SearchResult::empty("test");
        assert_eq!(r.phrase, "test");
        assert!(r.is_empty());
        assert_eq!(r.match_count(), 0);
    }

    // ── performance ──────────────────────────────────────────────────────────

    #[test]
    fn given_100k_lines_when_search_rare_phrase_then_completes_within_50ms() {
        use std::time::Instant;

        let mut data: Vec<u8> = Vec::with_capacity(8 * 1024 * 1024);
        for i in 0..100_000usize {
            // Inject the rare phrase on 10 evenly-spaced lines.
            if i % 10_000 == 0 {
                data.extend_from_slice(
                    b"RARE_MARKER log entry with some padding to reach 80 bytes xx\n",
                );
            } else {
                data.extend_from_slice(
                    b"ordinary log entry with some padding to reach eighty bytes xx\n",
                );
            }
        }

        let start = Instant::now();
        let result = search(&data, "RARE_MARKER").unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.match_count(), 10);
        assert!(
            elapsed.as_millis() < 50,
            "Search took {}ms, expected < 50ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn given_100k_lines_when_search_common_phrase_then_completes_within_100ms() {
        use std::time::Instant;

        let mut data: Vec<u8> = Vec::with_capacity(8 * 1024 * 1024);
        for _ in 0..100_000usize {
            data.extend_from_slice(
                b"common log entry with MATCH phrase and some padding bytes xx\n",
            );
        }

        let start = Instant::now();
        let result = search(&data, "MATCH").unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.match_count(), 100_000);
        assert!(
            elapsed.as_millis() < 200,
            "Search took {}ms, expected < 200ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn given_1m_lines_when_search_then_completes_within_500ms() {
        use std::time::Instant;

        let mut data: Vec<u8> = Vec::with_capacity(50 * 1024 * 1024);
        for i in 0..1_000_000usize {
            if i % 10_000 == 0 {
                data.extend_from_slice(b"NEEDLE short\n");
            } else {
                data.extend_from_slice(b"ordinary short line\n");
            }
        }

        let start = Instant::now();
        let result = search(&data, "NEEDLE").unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.match_count(), 100);
        assert!(
            elapsed.as_millis() < 500,
            "Search took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }
}
