use std::ops::Range;

use crate::domain::phrase_matching::{LineFilter, MatchingLines, MatchingLinesBatch};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GrepError {
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
    #[error("Every line contains: {0}")]
    PatternOnEveryLine(String),
}

/// A grep's line filters and the lines found so far that pass all of them.
pub struct GrepState {
    line_filters: Vec<LineFilter>,
    matching_lines: MatchingLines,
}

impl GrepState {
    pub(super) fn new(line_filters: Vec<LineFilter>) -> Self {
        Self {
            line_filters,
            matching_lines: MatchingLines::default(),
        }
    }

    pub fn line_filters(&self) -> &[LineFilter] {
        &self.line_filters
    }

    pub fn percent_grepped(&self) -> u64 {
        self.matching_lines.percent_scanned()
    }

    pub fn match_count(&self) -> usize {
        self.matching_lines.count()
    }

    pub fn no_lines_error(&self) -> GrepError {
        match self.line_filters.last() {
            Some(LineFilter::NotContaining(phrase)) => {
                GrepError::PatternOnEveryLine(phrase.clone())
            }
            Some(LineFilter::Containing(phrase)) => GrepError::PatternNotFound(phrase.clone()),
            None => GrepError::PatternNotFound(String::new()),
        }
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

    pub(super) fn matching_lines_in(&self, positions: Range<usize>) -> Vec<usize> {
        self.matching_lines.in_positions(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn containing(phrase: &str) -> LineFilter {
        LineFilter::Containing(phrase.to_owned())
    }

    fn not_containing(phrase: &str) -> LineFilter {
        LineFilter::NotContaining(phrase.to_owned())
    }

    fn batch(matching_lines: &[usize], percent_scanned: u64) -> MatchingLinesBatch {
        MatchingLinesBatch {
            matching_lines: matching_lines.to_vec(),
            percent_scanned,
        }
    }

    #[test]
    fn given_new_grep_when_asking_for_its_matches_then_there_are_none_and_nothing_is_grepped() {
        let grep_state = GrepState::new(vec![containing("ERROR")]);

        assert_eq!(grep_state.line_filters(), [containing("ERROR")]);
        assert_eq!(grep_state.match_count(), 0);
        assert_eq!(grep_state.percent_grepped(), 0);
    }

    #[test]
    fn given_two_batches_added_when_asking_for_the_matching_lines_then_they_are_one_list_in_order()
    {
        let mut grep_state = GrepState::new(vec![containing("ERROR")]);
        grep_state.add_batch(batch(&[3, 7], 40));

        grep_state.add_batch(batch(&[12], 62));

        assert_eq!(grep_state.match_count(), 3);
        assert_eq!(grep_state.matching_line(2), Some(12));
        assert_eq!(grep_state.matching_line(3), None);
        assert_eq!(grep_state.percent_grepped(), 62);
    }

    #[test]
    fn given_grep_without_lines_when_asking_for_its_error_then_it_depends_on_the_last_filter() {
        let grep_ending_with_not_containing =
            GrepState::new(vec![containing("ERROR"), not_containing("user=42")]);
        let grep_ending_with_containing =
            GrepState::new(vec![not_containing("user=42"), containing("ERROR")]);

        assert_eq!(
            grep_ending_with_not_containing.no_lines_error(),
            GrepError::PatternOnEveryLine("user=42".to_owned())
        );
        assert_eq!(
            grep_ending_with_containing.no_lines_error(),
            GrepError::PatternNotFound("ERROR".to_owned())
        );
    }

    #[test]
    fn given_grep_error_when_displayed_then_it_names_the_phrase() {
        assert_eq!(
            GrepError::PatternNotFound("zzz".to_owned()).to_string(),
            "Pattern not found: zzz"
        );
        assert_eq!(
            GrepError::PatternOnEveryLine("INFO".to_owned()).to_string(),
            "Every line contains: INFO"
        );
    }
}
