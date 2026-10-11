use super::{SearchDirection, SearchQuery};

#[derive(Debug, PartialEq)]
pub struct MatchJump {
    pub line: usize,
    pub direction: SearchDirection,
    pub wrapped: bool,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SearchError {
    #[error("No previous search")]
    NoPreviousSearch,
    #[error("Still indexing, try again")]
    StillIndexing,
}

/// The last search and the lines that match it: their positions, counted from 0 like the cursor
/// line, in file order. A new query or clearing empties them, so they always belong to the query.
/// Jumps between them use a binary search.
#[derive(Default)]
pub struct SearchState {
    query: Option<SearchQuery>,
    matching_lines: Vec<usize>,
}

impl SearchState {
    pub fn query(&self) -> Option<&SearchQuery> {
        self.query.as_ref()
    }

    pub fn replace_query(&mut self, query: SearchQuery) {
        self.query = Some(query);
        self.matching_lines.clear();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn set_direction(&mut self, direction: SearchDirection) {
        if let Some(query) = &mut self.query {
            query.direction = direction;
        }
    }

    pub fn add_matching_lines(&mut self, matching_lines: Vec<usize>) {
        self.matching_lines.extend(matching_lines);
    }

    pub fn next_match_from(&self, line: usize) -> Result<Option<MatchJump>, SearchError> {
        let direction = self.direction()?;
        Ok(self.jump_from(line, direction))
    }

    pub fn previous_match_from(&self, line: usize) -> Result<Option<MatchJump>, SearchError> {
        let direction = self.direction()?;
        Ok(self.jump_from(line, direction.reversed()))
    }

    pub fn match_count(&self) -> usize {
        self.matching_lines.len()
    }

    pub fn match_count_at_or_above(&self, line: usize) -> usize {
        self.matching_lines
            .partition_point(|&matching_line| matching_line <= line)
    }

    fn jump_from(&self, line: usize, direction: SearchDirection) -> Option<MatchJump> {
        let (next_match, wrap_target) = match direction {
            SearchDirection::Forward => (
                self.matches_below(line).first(),
                self.matching_lines.first(),
            ),
            SearchDirection::Backward => {
                (self.matches_above(line).last(), self.matching_lines.last())
            }
        };
        let wrapped = next_match.is_none();

        next_match.or(wrap_target).map(|&matching_line| MatchJump {
            line: matching_line,
            direction,
            wrapped,
        })
    }

    fn matches_below(&self, line: usize) -> &[usize] {
        &self.matching_lines[self.match_count_at_or_above(line)..]
    }

    fn matches_above(&self, line: usize) -> &[usize] {
        let match_count_above = self
            .matching_lines
            .partition_point(|&matching_line| matching_line < line);
        &self.matching_lines[..match_count_above]
    }

    fn direction(&self) -> Result<SearchDirection, SearchError> {
        self.query
            .as_ref()
            .map(|query| query.direction)
            .ok_or(SearchError::NoPreviousSearch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::search::SearchDirection::{Backward, Forward};

    fn make_query(phrase: &str, direction: SearchDirection) -> SearchQuery {
        SearchQuery {
            phrase: phrase.to_owned(),
            direction,
        }
    }

    fn make_search_state(direction: SearchDirection, matching_lines: &[usize]) -> SearchState {
        let mut search_state = SearchState::default();
        search_state.replace_query(make_query("ERROR", direction));
        search_state.add_matching_lines(matching_lines.to_vec());
        search_state
    }

    fn make_matches(matching_lines: &[usize]) -> SearchState {
        make_search_state(Forward, matching_lines)
    }

    fn jump_line(match_jump: Result<Option<MatchJump>, SearchError>) -> Option<usize> {
        match_jump.unwrap().map(|match_jump| match_jump.line)
    }

    fn jump_to(line: usize, direction: SearchDirection) -> Option<MatchJump> {
        Some(MatchJump {
            line,
            direction,
            wrapped: false,
        })
    }

    fn wrap_to(line: usize, direction: SearchDirection) -> Option<MatchJump> {
        Some(MatchJump {
            line,
            direction,
            wrapped: true,
        })
    }

    #[test]
    fn given_matches_of_a_query_when_the_query_is_replaced_then_the_old_matches_are_gone() {
        let mut search_state = make_search_state(Forward, &[3, 7]);

        search_state.replace_query(make_query("WARN", Forward));

        assert_eq!(search_state.match_count(), 0);
        assert_eq!(search_state.query(), Some(&make_query("WARN", Forward)));
    }

    #[test]
    fn given_query_with_matches_when_cleared_then_both_are_gone() {
        let mut search_state = make_search_state(Forward, &[3, 7]);

        search_state.clear();

        assert_eq!(search_state.query(), None);
        assert_eq!(search_state.match_count(), 0);
    }

    #[test]
    fn given_forward_query_when_its_direction_is_set_backward_then_the_phrase_and_matches_stay() {
        let mut search_state = make_search_state(Forward, &[3, 7]);

        search_state.set_direction(Backward);

        assert_eq!(search_state.query(), Some(&make_query("ERROR", Backward)));
        assert_eq!(search_state.match_count(), 2);
    }

    #[test]
    fn given_no_query_when_a_direction_is_set_then_there_is_still_no_query() {
        let mut search_state = SearchState::default();

        search_state.set_direction(Backward);

        assert_eq!(search_state.query(), None);
    }

    #[test]
    fn given_matching_lines_added_in_two_batches_when_jumping_then_they_are_one_list() {
        let mut search_state = make_matches(&[3, 7]);

        search_state.add_matching_lines(vec![12, 20]);

        assert_eq!(search_state.match_count(), 4);
        assert_eq!(search_state.jump_from(7, Forward), jump_to(12, Forward));
        assert_eq!(search_state.jump_from(20, Backward), jump_to(12, Backward));
    }

    #[test]
    fn given_forward_query_when_asking_for_next_and_previous_matches_then_they_go_down_and_up() {
        let search_state = make_search_state(Forward, &[3, 7, 12]);

        assert_eq!(jump_line(search_state.next_match_from(7)), Some(12));
        assert_eq!(jump_line(search_state.previous_match_from(7)), Some(3));
    }

    #[test]
    fn given_backward_query_when_asking_for_next_and_previous_matches_then_they_go_up_and_down() {
        let search_state = make_search_state(Backward, &[3, 7, 12]);

        assert_eq!(jump_line(search_state.next_match_from(7)), Some(3));
        assert_eq!(jump_line(search_state.previous_match_from(7)), Some(12));
    }

    #[test]
    fn given_query_without_matches_when_asking_for_the_next_match_then_there_is_none() {
        let search_state = make_search_state(Forward, &[]);

        assert_eq!(search_state.next_match_from(0), Ok(None));
    }

    #[test]
    fn given_no_query_when_asking_for_a_match_then_there_is_no_previous_search() {
        let search_state = SearchState::default();

        assert_eq!(
            search_state.next_match_from(0),
            Err(SearchError::NoPreviousSearch)
        );
        assert_eq!(
            search_state.previous_match_from(0),
            Err(SearchError::NoPreviousSearch)
        );
    }

    #[test]
    fn given_cursor_before_the_first_match_when_jumping_forward_then_it_goes_to_the_first_match() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(0, Forward), jump_to(3, Forward));
    }

    #[test]
    fn given_cursor_after_the_last_match_when_jumping_backward_then_it_goes_to_the_last_match() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(20, Backward), jump_to(12, Backward));
    }

    #[test]
    fn given_cursor_on_a_match_when_jumping_then_it_goes_to_the_next_match_in_that_direction() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(7, Forward), jump_to(12, Forward));
        assert_eq!(search_state.jump_from(7, Backward), jump_to(3, Backward));
    }

    #[test]
    fn given_cursor_between_matches_when_jumping_then_it_goes_to_the_nearest_match_in_that_direction()
     {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(8, Forward), jump_to(12, Forward));
        assert_eq!(search_state.jump_from(8, Backward), jump_to(7, Backward));
    }

    #[test]
    fn given_cursor_on_or_after_the_last_match_when_jumping_forward_then_it_wraps_to_the_first() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(12, Forward), wrap_to(3, Forward));
        assert_eq!(search_state.jump_from(20, Forward), wrap_to(3, Forward));
    }

    #[test]
    fn given_cursor_on_or_before_the_first_match_when_jumping_backward_then_it_wraps_to_the_last() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.jump_from(3, Backward), wrap_to(12, Backward));
        assert_eq!(search_state.jump_from(0, Backward), wrap_to(12, Backward));
    }

    #[test]
    fn given_only_match_under_the_cursor_when_jumping_then_it_wraps_to_itself() {
        let search_state = make_matches(&[5]);

        assert_eq!(search_state.jump_from(5, Forward), wrap_to(5, Forward));
        assert_eq!(search_state.jump_from(5, Backward), wrap_to(5, Backward));
    }

    #[test]
    fn given_no_matches_when_jumping_either_way_then_there_is_no_jump() {
        let search_state = make_matches(&[]);

        assert_eq!(search_state.jump_from(0, Forward), None);
        assert_eq!(search_state.jump_from(0, Backward), None);
    }

    #[test]
    fn given_cursor_anywhere_when_counting_matches_at_or_above_then_a_match_on_the_cursor_counts() {
        let search_state = make_matches(&[3, 7, 12]);

        assert_eq!(search_state.match_count_at_or_above(0), 0);
        assert_eq!(search_state.match_count_at_or_above(3), 1);
        assert_eq!(search_state.match_count_at_or_above(8), 2);
        assert_eq!(search_state.match_count_at_or_above(12), 3);
        assert_eq!(search_state.match_count_at_or_above(99), 3);
    }

    #[test]
    fn given_search_errors_when_displayed_then_they_say_what_happened() {
        assert_eq!(
            SearchError::NoPreviousSearch.to_string(),
            "No previous search"
        );
        assert_eq!(
            SearchError::StillIndexing.to_string(),
            "Still indexing, try again"
        );
    }
}
