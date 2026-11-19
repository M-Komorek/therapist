mod line_offsets;
mod matching_lines;

use std::ops::Range;

use memchr::memmem;

pub use matching_lines::MatchingLines;

use crate::domain::log_file::count_newlines;

#[derive(Debug, Clone, PartialEq)]
pub enum LineFilter {
    Containing(String),
    NotContaining(String),
}

#[derive(Debug, PartialEq)]
pub struct MatchingLinesBatch {
    pub matching_lines: Vec<usize>,
    pub percent_scanned: u64,
}

impl LineFilter {
    pub fn phrase(&self) -> &str {
        match self {
            Self::Containing(phrase) | Self::NotContaining(phrase) => phrase,
        }
    }

    pub fn keeps_line(&self, line_contains_phrase: bool) -> bool {
        match self {
            Self::Containing(_) => line_contains_phrase,
            Self::NotContaining(_) => !line_contains_phrase,
        }
    }
}

pub fn find_matching_lines(
    chunk: &[u8],
    chunk_first_line: usize,
    line_filters: &[LineFilter],
) -> Vec<usize> {
    let filters_in_scan_order = containing_filters_first(line_filters);
    let Some((first_filter, other_filters)) = filters_in_scan_order.split_first() else {
        return Vec::new();
    };

    let mut matching_lines = lines_passing(first_filter, chunk, chunk_first_line);
    for line_filter in other_filters {
        let lines_with_phrase =
            find_lines_containing(chunk, chunk_first_line, line_filter.phrase());
        matching_lines
            .retain(|line| line_filter.keeps_line(lines_with_phrase.binary_search(line).is_ok()));
    }
    matching_lines
}

fn lines_passing(line_filter: &LineFilter, chunk: &[u8], chunk_first_line: usize) -> Vec<usize> {
    match line_filter {
        LineFilter::Containing(phrase) => find_lines_containing(chunk, chunk_first_line, phrase),
        LineFilter::NotContaining(phrase) => {
            let lines_with_phrase = find_lines_containing(chunk, chunk_first_line, phrase);
            lines_in_chunk(chunk, chunk_first_line)
                .filter(|line| lines_with_phrase.binary_search(line).is_err())
                .collect()
        }
    }
}

fn containing_filters_first(line_filters: &[LineFilter]) -> Vec<&LineFilter> {
    let mut sorted_filters: Vec<&LineFilter> = line_filters.iter().collect();
    sorted_filters.sort_by_key(|line_filter| matches!(line_filter, LineFilter::NotContaining(_)));
    sorted_filters
}

fn lines_in_chunk(chunk: &[u8], chunk_first_line: usize) -> Range<usize> {
    let has_unterminated_last_line = !chunk.is_empty() && !chunk.ends_with(b"\n");
    let line_count = count_newlines(chunk) + usize::from(has_unterminated_last_line);
    chunk_first_line..chunk_first_line + line_count
}

fn find_lines_containing(chunk: &[u8], chunk_first_line: usize, phrase: &str) -> Vec<usize> {
    if phrase.is_empty() {
        return Vec::new();
    }

    let mut lines_with_phrase = Vec::new();
    let mut current_line = chunk_first_line;
    let mut newlines_counted_up_to = 0;

    for match_offset in memmem::find_iter(chunk, phrase.as_bytes()) {
        current_line += count_newlines(&chunk[newlines_counted_up_to..match_offset]);
        newlines_counted_up_to = match_offset;
        if lines_with_phrase.last() != Some(&current_line) {
            lines_with_phrase.push(current_line);
        }
    }
    lines_with_phrase
}

#[cfg(test)]
mod tests {
    use super::*;

    fn containing(phrases: &[&str]) -> Vec<LineFilter> {
        phrases
            .iter()
            .map(|phrase| LineFilter::Containing((*phrase).to_owned()))
            .collect()
    }

    fn not_containing(phrase: &str) -> LineFilter {
        LineFilter::NotContaining(phrase.to_owned())
    }

    #[test]
    fn given_matches_at_the_very_start_and_end_when_searching_then_both_lines_are_found() {
        assert_eq!(
            find_lines_containing(b"ERROR first\nsecond\nlast ERROR", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_several_matches_on_one_line_when_searching_then_the_line_is_found_once() {
        assert_eq!(
            find_lines_containing(b"ERROR ERROR\nok\nERRORERROR\n", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_crlf_line_endings_when_searching_then_lines_are_counted_by_newlines() {
        assert_eq!(
            find_lines_containing(b"ok\r\nERROR\r\nok\r\nERROR\r\n", 0, "ERROR"),
            [1, 3]
        );
    }

    #[test]
    fn given_empty_phrase_when_searching_then_no_line_matches() {
        assert!(find_lines_containing(b"a\nb\n", 0, "").is_empty());
    }

    #[test]
    fn given_phrase_not_in_the_chunk_when_searching_then_no_line_matches() {
        assert!(find_lines_containing(b"a\nb\n", 0, "ERROR").is_empty());
    }

    #[test]
    fn given_chunk_starting_at_line_100_when_searching_then_lines_are_counted_from_there() {
        assert_eq!(
            find_lines_containing(b"ok\nERROR\nok\n", 100, "ERROR"),
            [101]
        );
    }

    #[test]
    fn given_two_phrases_when_finding_matching_lines_then_only_lines_with_both_match() {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=42\nuser=42 ERROR\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &containing(&["ERROR", "user=42"])),
            [0, 3]
        );
    }

    #[test]
    fn given_two_phrases_in_either_order_when_finding_matching_lines_then_the_lines_are_the_same() {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=42\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &containing(&["user=42", "ERROR"])),
            find_matching_lines(chunk, 0, &containing(&["ERROR", "user=42"]))
        );
    }

    #[test]
    fn given_phrases_on_different_lines_only_when_finding_matching_lines_then_no_line_matches() {
        assert!(
            find_matching_lines(b"ERROR\nuser=42\n", 0, &containing(&["ERROR", "user=42"]))
                .is_empty()
        );
    }

    #[test]
    fn given_no_phrases_when_finding_matching_lines_then_no_line_matches() {
        assert!(find_matching_lines(b"a\nb\n", 0, &[]).is_empty());
    }

    #[test]
    fn given_chunk_starting_at_line_100_when_finding_lines_with_two_phrases_then_lines_are_counted_from_there()
     {
        assert_eq!(
            find_matching_lines(
                b"ERROR\nERROR user=42\n",
                100,
                &containing(&["ERROR", "user=42"])
            ),
            [101]
        );
    }

    #[test]
    fn given_only_a_not_containing_filter_when_finding_matching_lines_then_every_line_without_the_phrase_matches()
     {
        let chunk = b"INFO a\nWARN b\nINFO c\nERROR d\n";

        assert_eq!(
            find_matching_lines(chunk, 10, &[not_containing("INFO")]),
            [11, 13]
        );
    }

    #[test]
    fn given_containing_and_not_containing_filters_in_either_order_when_finding_matching_lines_then_lines_meet_both()
     {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=7\nERROR user=9\n";
        let error_filter = LineFilter::Containing("ERROR".to_owned());
        let not_user_42_filter = not_containing("user=42");

        assert_eq!(
            find_matching_lines(
                chunk,
                0,
                &[error_filter.clone(), not_user_42_filter.clone()]
            ),
            [1, 3]
        );
        assert_eq!(
            find_matching_lines(chunk, 0, &[not_user_42_filter, error_filter]),
            [1, 3]
        );
    }

    #[test]
    fn given_two_not_containing_filters_when_finding_matching_lines_then_only_lines_without_either_match()
     {
        let chunk = b"INFO a\nWARN b\nDEBUG c\nERROR d\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &[not_containing("INFO"), not_containing("DEBUG")]),
            [1, 3]
        );
    }

    #[test]
    fn given_crlf_and_an_unterminated_last_line_when_keeping_lines_without_a_phrase_then_each_line_is_counted_once()
     {
        assert_eq!(
            find_matching_lines(b"a\r\nzzz\r\nb", 0, &[not_containing("zzz")]),
            [0, 2]
        );
    }

    #[test]
    fn given_empty_chunk_when_keeping_lines_without_a_phrase_then_no_line_matches() {
        assert!(find_matching_lines(b"", 0, &[not_containing("zzz")]).is_empty());
    }

    #[test]
    fn given_phrase_on_every_line_when_keeping_lines_without_it_then_no_line_matches() {
        assert!(find_matching_lines(b"INFO a\nINFO b\n", 0, &[not_containing("INFO")]).is_empty());
    }
}
