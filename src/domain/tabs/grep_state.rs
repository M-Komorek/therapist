use std::ops::Range;

use crate::domain::phrase_matching::{MatchingLines, MatchingLinesBatch};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GrepError {
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
}

/// A grep's phrases and the lines found so far that contain all of them.
pub struct GrepState {
    phrases: Vec<String>,
    matching_lines: MatchingLines,
}

impl GrepState {
    pub(super) fn new(phrases: Vec<String>) -> Self {
        Self {
            phrases,
            matching_lines: MatchingLines::default(),
        }
    }

    pub fn phrases(&self) -> &[String] {
        &self.phrases
    }

    pub fn percent_grepped(&self) -> u64 {
        self.matching_lines.percent_scanned()
    }

    pub fn match_count(&self) -> usize {
        self.matching_lines.count()
    }

    pub fn add_batch(&mut self, matching_lines_batch: MatchingLinesBatch) {
        self.matching_lines.add_batch(matching_lines_batch);
    }

    pub(super) fn matching_line(&self, position: usize) -> Option<usize> {
        self.matching_lines.get(position)
    }

    pub(super) fn match_count_above(&self, line: usize) -> usize {
        self.matching_lines.count_above(line)
    }

    pub(super) fn contains(&self, line: usize) -> bool {
        self.matching_lines.contains(line)
    }

    pub(super) fn matching_lines_in(&self, positions: Range<usize>) -> &[usize] {
        self.matching_lines.in_positions(positions)
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

    #[test]
    fn given_new_grep_when_asking_for_its_matches_then_there_are_none_and_nothing_is_grepped() {
        let grep_state = GrepState::new(vec!["ERROR".to_owned()]);

        assert_eq!(grep_state.phrases(), ["ERROR"]);
        assert_eq!(grep_state.match_count(), 0);
        assert_eq!(grep_state.percent_grepped(), 0);
    }

    #[test]
    fn given_two_batches_added_when_asking_for_the_matching_lines_then_they_are_one_list_in_order()
    {
        let mut grep_state = GrepState::new(vec!["ERROR".to_owned()]);
        grep_state.add_batch(batch(&[3, 7], 40));

        grep_state.add_batch(batch(&[12], 62));

        assert_eq!(grep_state.match_count(), 3);
        assert_eq!(grep_state.matching_line(2), Some(12));
        assert_eq!(grep_state.matching_line(3), None);
        assert_eq!(grep_state.percent_grepped(), 62);
    }

    #[test]
    fn given_grep_error_when_displayed_then_it_names_the_phrase() {
        assert_eq!(
            GrepError::PatternNotFound("zzz".to_owned()).to_string(),
            "Pattern not found: zzz"
        );
    }
}
