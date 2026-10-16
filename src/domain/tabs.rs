mod grep_state;
mod log_view_state;
mod tab_state;

pub use grep_state::{GrepError, GrepState};
pub use log_view_state::LogViewState;
pub use tab_state::TabState;

use crate::domain::{phrase_matching::MatchingLinesBatch, search::SearchState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabId(u64);

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum TabError {
    #[error("The [all] tab can't be closed")]
    CannotCloseAllLinesTab,
}

/// The open tabs in tab-bar order, and the active one. The first tab shows every line of the file
/// and can't be closed, so there is always an active tab. A new tab opens right after the active
/// one and becomes active; closing the active tab activates its left neighbour. Tab ids never
/// repeat.
pub struct Tabs {
    tab_states: Vec<TabState>,
    active_position: usize,
    next_unused_tab_id: u64,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            tab_states: vec![TabState::for_all_lines(TabId(0))],
            active_position: 0,
            next_unused_tab_id: 1,
        }
    }
}

impl Tabs {
    pub fn active_tab(&self) -> &TabState {
        &self.tab_states[self.active_position]
    }

    pub fn active_tab_mut(&mut self) -> &mut TabState {
        &mut self.tab_states[self.active_position]
    }

    pub fn grep_state(&self, tab_id: TabId) -> Option<&GrepState> {
        let position = self.position_of(tab_id)?;
        self.tab_states[position].grep_state()
    }

    pub fn add_grep_batch(&mut self, tab_id: TabId, matching_lines_batch: MatchingLinesBatch) {
        if let Some(position) = self.position_of(tab_id)
            && let Some(grep_state) = self.tab_states[position].grep_state_mut()
        {
            grep_state.add_batch(matching_lines_batch);
        }
    }

    pub fn search_state(&self, tab_id: TabId) -> Option<&SearchState> {
        let position = self.position_of(tab_id)?;
        Some(self.tab_states[position].search_state())
    }

    pub fn search_state_mut(&mut self, tab_id: TabId) -> Option<&mut SearchState> {
        let position = self.position_of(tab_id)?;
        Some(self.tab_states[position].search_state_mut())
    }

    pub fn tab_states(&self) -> &[TabState] {
        &self.tab_states
    }

    pub fn active_position(&self) -> usize {
        self.active_position
    }

    pub fn open_grep_tab(&mut self, phrases: Vec<String>) -> TabId {
        let tab_id = self.take_unused_tab_id();
        let log_view_state = LogViewState::with_size_of(self.active_tab().log_view_state());
        self.active_position += 1;
        self.tab_states.insert(
            self.active_position,
            TabState::for_grep(tab_id, phrases, log_view_state),
        );
        tab_id
    }

    pub fn close(&mut self, tab_id: TabId) -> Result<(), TabError> {
        let Some(position) = self.position_of(tab_id) else {
            return Ok(());
        };
        if self.tab_states[position].shows_all_lines() {
            return Err(TabError::CannotCloseAllLinesTab);
        }

        self.tab_states.remove(position);
        if position <= self.active_position {
            self.active_position -= 1;
        }
        Ok(())
    }

    pub fn switch_to_next(&mut self) {
        self.active_position = (self.active_position + 1) % self.tab_states.len();
    }

    pub fn switch_to_previous(&mut self) {
        let tab_count = self.tab_states.len();
        self.active_position = (self.active_position + tab_count - 1) % tab_count;
    }

    pub fn set_log_view_size(&mut self, height: usize, width: usize, file_line_count: usize) {
        for tab_state in &mut self.tab_states {
            let row_count = tab_state.row_count(file_line_count);
            let log_view_state = tab_state.log_view_state_mut();
            log_view_state.set_height(height, row_count);
            log_view_state.set_width(width);
        }
    }

    fn take_unused_tab_id(&mut self) -> TabId {
        let tab_id = TabId(self.next_unused_tab_id);
        self.next_unused_tab_id += 1;
        tab_id
    }

    fn position_of(&self, tab_id: TabId) -> Option<usize> {
        self.tab_states
            .iter()
            .position(|tab_state| tab_state.tab_id() == tab_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tabs_with_grep_tabs(phrases: &[&str]) -> Tabs {
        let mut tabs = Tabs::default();
        for phrase in phrases {
            tabs.open_grep_tab(vec![(*phrase).to_owned()]);
        }
        tabs
    }

    fn batch(matching_lines: Vec<usize>) -> MatchingLinesBatch {
        MatchingLinesBatch {
            matching_lines,
            percent_scanned: 100,
        }
    }

    fn tab_names(tabs: &Tabs) -> Vec<String> {
        tabs.tab_states().iter().map(TabState::name).collect()
    }

    #[test]
    fn given_new_tabs_when_asking_for_the_active_tab_then_it_is_the_all_tab_and_the_only_one() {
        let tabs = Tabs::default();

        assert_eq!(tab_names(&tabs), ["all"]);
        assert!(tabs.active_tab().shows_all_lines());
        assert_eq!(tabs.active_position(), 0);
    }

    #[test]
    fn given_last_tab_active_when_a_grep_tab_opens_then_it_is_added_at_the_end_and_active() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR"]);

        tabs.open_grep_tab(vec!["WARN".to_owned()]);

        assert_eq!(tab_names(&tabs), ["all", "ERROR", "WARN"]);
        assert_eq!(tabs.active_position(), 2);
        assert_eq!(tabs.active_tab().name(), "WARN");
    }

    #[test]
    fn given_middle_tab_active_when_a_grep_tab_opens_then_it_is_added_right_after_it_and_active() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);
        tabs.switch_to_previous();

        tabs.open_grep_tab(vec!["ERROR".to_owned(), "user=42".to_owned()]);

        assert_eq!(
            tab_names(&tabs),
            ["all", "ERROR", "ERROR › user=42", "WARN"]
        );
        assert_eq!(tabs.active_position(), 2);
    }

    #[test]
    fn given_closed_tab_when_another_grep_tab_opens_then_its_id_is_new() {
        let mut tabs = Tabs::default();
        let closed_tab_id = tabs.open_grep_tab(vec!["ERROR".to_owned()]);
        tabs.close(closed_tab_id).unwrap();

        let new_tab_id = tabs.open_grep_tab(vec!["ERROR".to_owned()]);

        assert_ne!(new_tab_id, closed_tab_id);
        assert!(tabs.grep_state(closed_tab_id).is_none());
    }

    #[test]
    fn given_sized_tabs_when_a_grep_tab_opens_then_it_has_the_same_size() {
        let mut tabs = Tabs::default();
        tabs.set_log_view_size(10, 80, 0);
        let grep_tab_id = tabs.open_grep_tab(vec!["ERROR".to_owned()]);
        tabs.add_grep_batch(grep_tab_id, batch((0..100).collect()));
        let active_tab = tabs.active_tab_mut();

        active_tab.jump_cursor_to_line(49, 1000);

        assert_eq!(active_tab.log_view_state().top_row(), 44);
    }

    #[test]
    fn given_grep_tabs_when_a_batch_arrives_for_one_then_only_its_grep_gets_it() {
        let mut tabs = Tabs::default();
        let error_tab_id = tabs.open_grep_tab(vec!["ERROR".to_owned()]);
        let warn_tab_id = tabs.open_grep_tab(vec!["WARN".to_owned()]);

        tabs.add_grep_batch(error_tab_id, batch(vec![3, 7]));

        let match_count = |tab_id| tabs.grep_state(tab_id).map(GrepState::match_count);
        assert_eq!(match_count(error_tab_id), Some(2));
        assert_eq!(match_count(warn_tab_id), Some(0));
    }

    #[test]
    fn given_closed_tab_when_a_batch_arrives_for_it_then_nothing_changes() {
        let mut tabs = Tabs::default();
        let closed_tab_id = tabs.open_grep_tab(vec!["ERROR".to_owned()]);
        tabs.close(closed_tab_id).unwrap();

        tabs.add_grep_batch(closed_tab_id, batch(vec![3]));

        assert_eq!(tabs.tab_states().len(), 1);
    }

    #[test]
    fn given_active_grep_tab_when_it_is_closed_then_its_left_neighbour_is_active() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);
        tabs.switch_to_previous();
        let error_tab_id = tabs.active_tab().tab_id();

        tabs.close(error_tab_id).unwrap();

        assert_eq!(tab_names(&tabs), ["all", "WARN"]);
        assert_eq!(tabs.active_tab().name(), "all");
    }

    #[test]
    fn given_tab_left_of_the_active_one_when_it_is_closed_then_the_same_tab_stays_active() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);
        let error_tab_id = tabs.tab_states()[1].tab_id();

        tabs.close(error_tab_id).unwrap();

        assert_eq!(tabs.active_tab().name(), "WARN");
    }

    #[test]
    fn given_tab_right_of_the_active_one_when_it_is_closed_then_the_same_tab_stays_active() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);
        let warn_tab_id = tabs.active_tab().tab_id();
        tabs.switch_to_previous();

        tabs.close(warn_tab_id).unwrap();

        assert_eq!(tabs.active_tab().name(), "ERROR");
    }

    #[test]
    fn given_all_lines_tab_when_closing_it_then_it_is_refused() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR"]);
        let all_lines_tab_id = tabs.tab_states()[0].tab_id();

        assert_eq!(
            tabs.close(all_lines_tab_id),
            Err(TabError::CannotCloseAllLinesTab)
        );
        assert_eq!(tab_names(&tabs), ["all", "ERROR"]);
    }

    #[test]
    fn given_last_tab_active_when_switching_to_the_next_then_it_wraps_to_the_first() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);

        tabs.switch_to_next();
        assert_eq!(tabs.active_tab().name(), "all");

        tabs.switch_to_next();
        assert_eq!(tabs.active_tab().name(), "ERROR");
    }

    #[test]
    fn given_first_tab_active_when_switching_to_the_previous_then_it_wraps_to_the_last() {
        let mut tabs = make_tabs_with_grep_tabs(&["ERROR", "WARN"]);
        tabs.switch_to_next();

        tabs.switch_to_previous();

        assert_eq!(tabs.active_tab().name(), "WARN");
    }

    #[test]
    fn given_only_the_all_tab_when_switching_either_way_then_it_stays_active() {
        let mut tabs = Tabs::default();

        tabs.switch_to_next();
        tabs.switch_to_previous();

        assert_eq!(tabs.active_position(), 0);
    }

    #[test]
    fn given_tab_error_when_displayed_then_it_says_what_happened() {
        assert_eq!(
            TabError::CannotCloseAllLinesTab.to_string(),
            "The [all] tab can't be closed"
        );
    }
}
