use std::ops::Range;

use super::{GrepState, LogViewState, TabId};
use crate::domain::{
    log_file::{LogFile, LogFileError},
    search::SearchState,
};

const ALL_LINES_TAB_NAME: &str = "all";
const PHRASE_SEPARATOR: &str = " > ";

#[derive(Debug, PartialEq)]
pub struct TabRow {
    pub line: usize,
    pub text: String,
}

enum TabLines {
    All,
    Grepped(GrepState),
}

/// One tab: the lines it shows, its own log view state and its own search. Rows are counted within
/// the tab: in the `[all]` tab, row k shows file line k; in a grep tab, it shows the k-th matching
/// line in file order.
pub struct TabState {
    tab_id: TabId,
    tab_lines: TabLines,
    log_view_state: LogViewState,
    search_state: SearchState,
}

impl TabState {
    pub(super) fn for_all_lines(tab_id: TabId) -> Self {
        Self {
            tab_id,
            tab_lines: TabLines::All,
            log_view_state: LogViewState::default(),
            search_state: SearchState::default(),
        }
    }

    pub(super) fn for_grep(
        tab_id: TabId,
        phrases: Vec<String>,
        log_view_state: LogViewState,
    ) -> Self {
        Self {
            tab_id,
            tab_lines: TabLines::Grepped(GrepState::new(phrases)),
            log_view_state,
            search_state: SearchState::default(),
        }
    }

    pub fn tab_id(&self) -> TabId {
        self.tab_id
    }

    pub fn name(&self) -> String {
        match &self.tab_lines {
            TabLines::All => ALL_LINES_TAB_NAME.to_owned(),
            TabLines::Grepped(grep_state) => grep_state.phrases().join(PHRASE_SEPARATOR),
        }
    }

    pub fn grep_phrases_with(&self, phrase: String) -> Vec<String> {
        let mut phrases = match &self.tab_lines {
            TabLines::All => Vec::new(),
            TabLines::Grepped(grep_state) => grep_state.phrases().to_vec(),
        };
        phrases.push(phrase);
        phrases
    }

    pub fn shows_all_lines(&self) -> bool {
        matches!(self.tab_lines, TabLines::All)
    }

    pub fn grep_state(&self) -> Option<&GrepState> {
        match &self.tab_lines {
            TabLines::All => None,
            TabLines::Grepped(grep_state) => Some(grep_state),
        }
    }

    pub(super) fn grep_state_mut(&mut self) -> Option<&mut GrepState> {
        match &mut self.tab_lines {
            TabLines::All => None,
            TabLines::Grepped(grep_state) => Some(grep_state),
        }
    }

    pub fn log_view_state(&self) -> &LogViewState {
        &self.log_view_state
    }

    pub fn log_view_state_mut(&mut self) -> &mut LogViewState {
        &mut self.log_view_state
    }

    pub fn search_state(&self) -> &SearchState {
        &self.search_state
    }

    pub fn search_state_mut(&mut self) -> &mut SearchState {
        &mut self.search_state
    }

    pub fn row_count(&self, file_line_count: usize) -> usize {
        match &self.tab_lines {
            TabLines::All => file_line_count,
            TabLines::Grepped(grep_state) => grep_state.match_count(),
        }
    }

    pub fn contains_line(&self, line: usize) -> bool {
        match &self.tab_lines {
            TabLines::All => true,
            TabLines::Grepped(grep_state) => grep_state.contains(line),
        }
    }

    pub fn cursor_line(&self, file_line_count: usize) -> Option<usize> {
        self.line_at(self.log_view_state.cursor_row(), file_line_count)
    }

    pub fn jump_cursor_to_line(&mut self, line: usize, file_line_count: usize) {
        let row = self.row_at_or_after(line);
        let row_count = self.row_count(file_line_count);
        self.log_view_state.jump_cursor_to_row(row, row_count);
    }

    pub fn move_cursor_to_line_number(&mut self, line_number: usize, file_line_count: usize) {
        self.jump_cursor_to_line(line_number.saturating_sub(1), file_line_count);
    }

    pub fn read_rows(
        &self,
        rows: Range<usize>,
        log_file: &LogFile,
    ) -> Result<Vec<TabRow>, LogFileError> {
        match &self.tab_lines {
            TabLines::All => read_line_range(rows, log_file),
            TabLines::Grepped(grep_state) => {
                read_each_line(grep_state.matching_lines_in(rows), log_file)
            }
        }
    }

    fn line_at(&self, row: usize, file_line_count: usize) -> Option<usize> {
        match &self.tab_lines {
            TabLines::All => (row < file_line_count).then_some(row),
            TabLines::Grepped(grep_state) => grep_state.matching_line(row),
        }
    }

    fn row_at_or_after(&self, line: usize) -> usize {
        match &self.tab_lines {
            TabLines::All => line,
            TabLines::Grepped(grep_state) => grep_state.match_count_above(line),
        }
    }
}

fn read_line_range(rows: Range<usize>, log_file: &LogFile) -> Result<Vec<TabRow>, LogFileError> {
    let line_texts = log_file.read_lines(rows.clone())?;
    let tab_rows = rows
        .zip(line_texts)
        .map(|(line, text)| TabRow { line, text })
        .collect();
    Ok(tab_rows)
}

fn read_each_line(lines: &[usize], log_file: &LogFile) -> Result<Vec<TabRow>, LogFileError> {
    let mut tab_rows = Vec::with_capacity(lines.len());

    for &line in lines {
        let Some(text) = log_file.read_lines(line..line + 1)?.pop() else {
            break;
        };
        tab_rows.push(TabRow { line, text });
    }
    Ok(tab_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phrase_matching::MatchingLinesBatch;

    const GREPPED_TEXT: &str = "ok\nERROR one\nok\nok\nERROR two\nok\nERROR three\n";

    fn make_all_lines_tab(height: usize) -> TabState {
        let mut tab_state = TabState::for_all_lines(TabId(0));
        tab_state.log_view_state.set_height(height);
        tab_state
    }

    fn make_grep_tab(matching_lines: &[usize], height: usize) -> TabState {
        let mut log_view_state = LogViewState::default();
        log_view_state.set_height(height);
        let mut tab_state = TabState::for_grep(TabId(1), vec!["ERROR".to_owned()], log_view_state);
        if let Some(grep_state) = tab_state.grep_state_mut() {
            grep_state.add_batch(MatchingLinesBatch {
                matching_lines: matching_lines.to_vec(),
                percent_scanned: 100,
            });
        }
        tab_state
    }

    fn tab_row(line: usize, text: &str) -> TabRow {
        TabRow {
            line,
            text: text.to_owned(),
        }
    }

    #[test]
    fn given_empty_file_when_asking_for_the_cursor_line_then_there_is_none() {
        let tab_state = make_all_lines_tab(10);

        assert_eq!(tab_state.cursor_line(0), None);
    }

    #[test]
    fn given_all_lines_tab_when_the_cursor_moves_then_its_row_is_the_file_line() {
        let mut tab_state = make_all_lines_tab(10);

        tab_state.log_view_state_mut().move_cursor_down(3, 100);

        assert_eq!(tab_state.cursor_line(100), Some(3));
    }

    #[test]
    fn given_line_number_when_moving_to_it_then_the_cursor_is_on_the_line_before_it_counted_from_0()
    {
        let mut tab_state = make_all_lines_tab(10);

        tab_state.move_cursor_to_line_number(50, 100);

        assert_eq!(tab_state.cursor_line(100), Some(49));
        assert_eq!(tab_state.log_view_state().top_row(), 44);
    }

    #[test]
    fn given_line_number_past_the_end_when_moving_to_it_then_cursor_stops_at_the_last_line() {
        let mut tab_state = make_all_lines_tab(10);

        tab_state.move_cursor_to_line_number(500, 100);

        assert_eq!(tab_state.cursor_line(100), Some(99));
    }

    #[test]
    fn given_line_number_zero_when_moving_to_it_then_cursor_goes_to_the_first_line() {
        let mut tab_state = make_all_lines_tab(10);
        tab_state.log_view_state_mut().move_cursor_down(5, 100);

        tab_state.move_cursor_to_line_number(0, 100);

        assert_eq!(tab_state.cursor_line(100), Some(0));
    }

    #[test]
    fn given_all_lines_tab_when_reading_rows_then_each_comes_with_its_file_line() {
        let log_file = LogFile::from_text("a\nb\nc\n");
        let tab_state = make_all_lines_tab(10);

        let tab_rows = tab_state.read_rows(1..5, &log_file).unwrap();

        assert_eq!(tab_rows, [tab_row(1, "b"), tab_row(2, "c")]);
    }

    #[test]
    fn given_grep_tab_when_asking_for_its_rows_then_they_are_its_matching_lines() {
        let mut tab_state = make_grep_tab(&[16, 33, 50, 67], 10);

        tab_state.log_view_state_mut().move_cursor_down(1, 4);

        assert_eq!(tab_state.row_count(1000), 4);
        assert_eq!(tab_state.cursor_line(1000), Some(33));
        assert_eq!(tab_state.name(), "ERROR");
        assert!(!tab_state.shows_all_lines());
    }

    #[test]
    fn given_grep_tab_when_jumping_to_a_line_then_the_cursor_goes_to_that_line_or_the_next_shown_one()
     {
        let mut tab_state = make_grep_tab(&[16, 33, 50, 67], 10);
        let mut cursor_line_after_jump_to = |line| {
            tab_state.jump_cursor_to_line(line, 1000);
            tab_state.cursor_line(1000)
        };

        assert_eq!(cursor_line_after_jump_to(0), Some(16));
        assert_eq!(cursor_line_after_jump_to(33), Some(33));
        assert_eq!(cursor_line_after_jump_to(39), Some(50));
        assert_eq!(cursor_line_after_jump_to(99), Some(67));
    }

    #[test]
    fn given_grep_tab_without_lines_when_asking_for_the_cursor_line_then_there_is_none() {
        let tab_state = make_grep_tab(&[], 10);

        assert_eq!(tab_state.row_count(1000), 0);
        assert_eq!(tab_state.cursor_line(1000), None);
    }

    #[test]
    fn given_grep_tab_when_asking_whether_it_contains_lines_then_only_its_matching_lines_are_in_it()
    {
        let tab_state = make_grep_tab(&[16, 33], 10);

        assert!(tab_state.contains_line(16) && tab_state.contains_line(33));
        assert!(!tab_state.contains_line(17));
        assert!(make_all_lines_tab(10).contains_line(17));
    }

    #[test]
    fn given_grep_tab_when_reading_rows_then_each_comes_with_its_original_line() {
        let log_file = LogFile::from_text(GREPPED_TEXT);
        let tab_state = make_grep_tab(&[1, 4, 6], 10);

        let tab_rows = tab_state.read_rows(1..10, &log_file).unwrap();

        assert_eq!(
            tab_rows,
            [tab_row(4, "ERROR two"), tab_row(6, "ERROR three")]
        );
    }

    #[test]
    fn given_all_lines_tab_and_grep_tab_when_asking_for_their_grep_then_only_the_grep_tab_has_one()
    {
        assert!(make_all_lines_tab(10).grep_state().is_none());
        assert_eq!(
            make_grep_tab(&[1, 4], 10)
                .grep_state()
                .map(GrepState::phrases),
            Some(["ERROR".to_owned()].as_slice())
        );
    }

    #[test]
    fn given_cascaded_grep_tab_when_asking_for_its_name_then_it_joins_the_phrases_in_order() {
        let phrases = vec!["ERROR".to_owned(), "user=42".to_owned()];
        let tab_state = TabState::for_grep(TabId(2), phrases, LogViewState::default());

        assert_eq!(tab_state.name(), "ERROR > user=42");
        assert_eq!(
            tab_state.grep_phrases_with("took=5".to_owned()),
            ["ERROR", "user=42", "took=5"]
        );
    }

    #[test]
    fn given_all_lines_tab_when_adding_a_phrase_to_its_grep_phrases_then_it_is_the_only_one() {
        let tab_state = make_all_lines_tab(10);

        assert_eq!(tab_state.name(), "all");
        assert_eq!(tab_state.grep_phrases_with("WARN".to_owned()), ["WARN"]);
    }
}
