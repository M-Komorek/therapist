use std::ops::Range;

use memchr::memmem;

use crate::domain::log_file::count_newlines;

#[derive(Debug, PartialEq)]
pub struct MatchingLinesBatch {
    pub matching_lines: Vec<usize>,
    pub percent_scanned: u64,
}

/// The lines a phrase scan has found so far, and how much of the file it has read. The lines are
/// counted from 0 and come in file order, because a scan reads the file from the start, so finding
/// a line among them is a binary search.
#[derive(Default)]
pub struct MatchingLines {
    lines: Vec<usize>,
    percent_scanned: u64,
}

impl MatchingLines {
    pub fn add_batch(&mut self, matching_lines_batch: MatchingLinesBatch) {
        self.lines.extend(matching_lines_batch.matching_lines);
        self.percent_scanned = matching_lines_batch.percent_scanned;
    }

    pub fn percent_scanned(&self) -> u64 {
        self.percent_scanned
    }

    pub fn count(&self) -> usize {
        self.lines.len()
    }

    pub fn count_above(&self, line: usize) -> usize {
        self.lines
            .partition_point(|&matching_line| matching_line < line)
    }

    pub fn count_at_or_above(&self, line: usize) -> usize {
        self.lines
            .partition_point(|&matching_line| matching_line <= line)
    }

    pub fn contains(&self, line: usize) -> bool {
        self.lines.binary_search(&line).is_ok()
    }

    pub fn get(&self, position: usize) -> Option<usize> {
        self.lines.get(position).copied()
    }

    pub fn last(&self) -> Option<usize> {
        self.lines.last().copied()
    }

    pub fn in_positions(&self, positions: Range<usize>) -> &[usize] {
        let positions_end = positions.end.min(self.lines.len());
        let positions_start = positions.start.min(positions_end);
        &self.lines[positions_start..positions_end]
    }
}

pub fn find_matching_lines(
    chunk: &[u8],
    chunk_first_line: usize,
    phrases: &[String],
) -> Vec<usize> {
    let Some((first_phrase, other_phrases)) = phrases.split_first() else {
        return Vec::new();
    };

    let mut matching_lines = find_lines_containing(chunk, chunk_first_line, first_phrase);
    for phrase in other_phrases {
        let lines_with_phrase = find_lines_containing(chunk, chunk_first_line, phrase);
        matching_lines.retain(|line| lines_with_phrase.binary_search(line).is_ok());
    }
    matching_lines
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

    fn make_matching_lines(lines: &[usize]) -> MatchingLines {
        let mut matching_lines = MatchingLines::default();
        matching_lines.add_batch(MatchingLinesBatch {
            matching_lines: lines.to_vec(),
            percent_scanned: 100,
        });
        matching_lines
    }

    fn phrases(phrase_texts: &[&str]) -> Vec<String> {
        phrase_texts
            .iter()
            .map(|phrase| (*phrase).to_owned())
            .collect()
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
            find_matching_lines(chunk, 0, &phrases(&["ERROR", "user=42"])),
            [0, 3]
        );
    }

    #[test]
    fn given_two_phrases_in_either_order_when_finding_matching_lines_then_the_lines_are_the_same() {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=42\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &phrases(&["user=42", "ERROR"])),
            find_matching_lines(chunk, 0, &phrases(&["ERROR", "user=42"]))
        );
    }

    #[test]
    fn given_phrases_on_different_lines_only_when_finding_matching_lines_then_no_line_matches() {
        assert!(
            find_matching_lines(b"ERROR\nuser=42\n", 0, &phrases(&["ERROR", "user=42"])).is_empty()
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
                &phrases(&["ERROR", "user=42"])
            ),
            [101]
        );
    }

    #[test]
    fn given_no_batches_when_asking_for_matching_lines_then_there_are_none_and_nothing_is_scanned()
    {
        let matching_lines = MatchingLines::default();

        assert_eq!(matching_lines.count(), 0);
        assert_eq!(matching_lines.percent_scanned(), 0);
        assert_eq!(matching_lines.get(0), None);
        assert_eq!(matching_lines.last(), None);
    }

    #[test]
    fn given_two_batches_when_they_are_added_then_their_lines_are_one_list_and_the_percent_is_the_last_one_s()
     {
        let mut matching_lines = make_matching_lines(&[3, 7]);

        matching_lines.add_batch(MatchingLinesBatch {
            matching_lines: vec![12],
            percent_scanned: 62,
        });

        assert_eq!(matching_lines.count(), 3);
        assert_eq!(matching_lines.get(2), Some(12));
        assert_eq!(matching_lines.get(3), None);
        assert_eq!(matching_lines.last(), Some(12));
        assert_eq!(matching_lines.percent_scanned(), 62);
    }

    #[test]
    fn given_matching_lines_when_counting_those_above_a_line_then_a_match_on_the_line_counts_only_at_or_above()
     {
        let matching_lines = make_matching_lines(&[3, 7, 12]);

        assert_eq!(matching_lines.count_above(0), 0);
        assert_eq!(matching_lines.count_above(3), 0);
        assert_eq!(matching_lines.count_above(8), 2);
        assert_eq!(matching_lines.count_above(99), 3);
        assert_eq!(matching_lines.count_at_or_above(0), 0);
        assert_eq!(matching_lines.count_at_or_above(3), 1);
        assert_eq!(matching_lines.count_at_or_above(12), 3);
    }

    #[test]
    fn given_matching_lines_when_asking_whether_lines_match_then_only_those_lines_do() {
        let matching_lines = make_matching_lines(&[3, 7]);

        assert!(matching_lines.contains(3) && matching_lines.contains(7));
        assert!(!matching_lines.contains(4));
    }

    #[test]
    fn given_positions_past_the_last_match_when_taking_lines_in_them_then_they_stop_at_the_last() {
        let matching_lines = make_matching_lines(&[3, 7, 12]);

        assert_eq!(matching_lines.in_positions(1..10), [7, 12]);
        assert!(matching_lines.in_positions(5..10).is_empty());
    }
}
