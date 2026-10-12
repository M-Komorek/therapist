use std::ops::Range;

use crate::domain::phrase_matching::MatchingLinesBatch;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GrepError {
    #[error("Still indexing, try again")]
    StillIndexing,
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
}

/// A grep's phrase and the lines that contain it, found so far: positions counted from 0, in file
/// order, because the grep reads the file from the start. Finding a line among them uses a binary
/// search.
pub struct GrepState {
    phrase: String,
    matching_lines: Vec<usize>,
    percent_grepped: u64,
}

impl GrepState {
    pub(super) fn new(phrase: String) -> Self {
        Self {
            phrase,
            matching_lines: Vec::new(),
            percent_grepped: 0,
        }
    }

    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    pub fn percent_grepped(&self) -> u64 {
        self.percent_grepped
    }

    pub fn match_count(&self) -> usize {
        self.matching_lines.len()
    }

    pub fn add_batch(&mut self, matching_lines_batch: MatchingLinesBatch) {
        self.matching_lines
            .extend(matching_lines_batch.matching_lines);
        self.percent_grepped = matching_lines_batch.percent_scanned;
    }

    pub(super) fn matching_line(&self, position: usize) -> Option<usize> {
        self.matching_lines.get(position).copied()
    }

    pub(super) fn match_count_above(&self, line: usize) -> usize {
        self.matching_lines
            .partition_point(|&matching_line| matching_line < line)
    }

    pub(super) fn contains(&self, line: usize) -> bool {
        self.matching_lines.binary_search(&line).is_ok()
    }

    pub(super) fn matching_lines_in(&self, positions: Range<usize>) -> &[usize] {
        let positions_end = positions.end.min(self.matching_lines.len());
        let positions_start = positions.start.min(positions_end);
        &self.matching_lines[positions_start..positions_end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(matching_lines: &[usize], percent_scanned: u64) -> MatchingLinesBatch {
        MatchingLinesBatch {
            matching_lines: matching_lines.to_vec(),
            percent_scanned,
        }
    }

    fn make_grep_state(matching_lines: &[usize]) -> GrepState {
        let mut grep_state = GrepState::new("ERROR".to_owned());
        grep_state.add_batch(batch(matching_lines, 100));
        grep_state
    }

    #[test]
    fn given_new_grep_when_asking_for_its_matches_then_there_are_none_and_nothing_is_grepped() {
        let grep_state = GrepState::new("ERROR".to_owned());

        assert_eq!(grep_state.phrase(), "ERROR");
        assert_eq!(grep_state.match_count(), 0);
        assert_eq!(grep_state.percent_grepped(), 0);
    }

    #[test]
    fn given_two_batches_added_when_asking_for_the_matching_lines_then_they_are_one_list_in_order()
    {
        let mut grep_state = GrepState::new("ERROR".to_owned());
        grep_state.add_batch(batch(&[3, 7], 40));

        grep_state.add_batch(batch(&[12], 62));

        assert_eq!(grep_state.match_count(), 3);
        assert_eq!(grep_state.matching_line(2), Some(12));
        assert_eq!(grep_state.matching_line(3), None);
        assert_eq!(grep_state.percent_grepped(), 62);
    }

    #[test]
    fn given_matching_lines_when_counting_the_matches_above_a_line_then_a_match_on_the_line_is_not_counted()
     {
        let grep_state = make_grep_state(&[3, 7, 12]);

        assert_eq!(grep_state.match_count_above(0), 0);
        assert_eq!(grep_state.match_count_above(3), 0);
        assert_eq!(grep_state.match_count_above(8), 2);
        assert_eq!(grep_state.match_count_above(99), 3);
    }

    #[test]
    fn given_matching_lines_when_asking_whether_lines_match_then_only_those_lines_do() {
        let grep_state = make_grep_state(&[3, 7]);

        assert!(grep_state.contains(3) && grep_state.contains(7));
        assert!(!grep_state.contains(4));
    }

    #[test]
    fn given_positions_past_the_last_match_when_taking_matching_lines_then_they_stop_at_the_last() {
        let grep_state = make_grep_state(&[3, 7, 12]);

        assert_eq!(grep_state.matching_lines_in(1..10), [7, 12]);
        assert!(grep_state.matching_lines_in(5..10).is_empty());
    }

    #[test]
    fn given_grep_errors_when_displayed_then_they_say_what_happened() {
        assert_eq!(
            GrepError::StillIndexing.to_string(),
            "Still indexing, try again"
        );
        assert_eq!(
            GrepError::PatternNotFound("zzz".to_owned()).to_string(),
            "Pattern not found: zzz"
        );
    }
}
