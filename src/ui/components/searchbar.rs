use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::mpsc::UnboundedSender;

use super::contract::Component;
use crate::action::Action;

pub struct SearchBar {
    action_tx: Option<UnboundedSender<Action>>,
    phrase: String,
    current_match: usize,
    total_matches: usize,
    visible: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            phrase: String::new(),
            current_match: 0,
            total_matches: 0,
            visible: false,
        }
    }

    fn send(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    fn left_segment(&self) -> String {
        format!("/ {}", self.phrase)
    }

    fn right_segment(&self) -> String {
        if self.total_matches == 0 {
            String::new()
        } else {
            format!("({}/{})", self.current_match, self.total_matches)
        }
    }
}

impl Component for SearchBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if !self.visible {
            return Ok(None);
        }

        match key.code {
            KeyCode::Char(c) => {
                self.phrase.push(c);
            }
            KeyCode::Backspace => {
                self.phrase.pop();
            }
            KeyCode::Enter => {
                self.send(Action::SearchRequest(self.phrase.clone()));
            }
            KeyCode::Esc => {
                self.send(Action::ExitSearch);
            }
            _ => {}
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::EnterSearch => {
                self.visible = true;
                self.phrase.clear();
                self.current_match = 0;
                self.total_matches = 0;
            }
            Action::ExitSearch => {
                self.visible = false;
            }
            Action::SearchBarUpdate {
                current_match,
                total_matches,
            } => {
                self.current_match = current_match;
                self.total_matches = total_matches;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if !self.visible {
            return Ok(());
        }

        let left = self.left_segment();
        let right = self.right_segment();

        let available_width = area.width as usize;
        let right_len = right.chars().count();
        let left_max = available_width.saturating_sub(right_len);

        let left_truncated: String = left.chars().take(left_max).collect();
        let padding = available_width
            .saturating_sub(left_truncated.chars().count())
            .saturating_sub(right_len);

        let bar_style = Style::default().bg(Color::DarkGray).fg(Color::White);

        let line = Line::from(vec![
            Span::styled(left_truncated, bar_style),
            Span::styled(" ".repeat(padding), bar_style),
            Span::styled(right, bar_style),
        ]);

        frame.render_widget(Paragraph::new(line).style(bar_style), area);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::components::contract::Component;

    fn make_searchbar() -> SearchBar {
        SearchBar::new()
    }

    fn enter_search(bar: &mut SearchBar) {
        bar.update(Action::EnterSearch).unwrap();
    }

    fn type_char(bar: &mut SearchBar, c: char) {
        bar.handle_key_event(KeyEvent::from(KeyCode::Char(c)))
            .unwrap();
    }

    fn press_backspace(bar: &mut SearchBar) {
        bar.handle_key_event(KeyEvent::from(KeyCode::Backspace))
            .unwrap();
    }

    // ── visibility ───────────────────────────────────────────────────────────

    #[test]
    fn given_new_searchbar_when_created_then_not_visible() {
        let bar = make_searchbar();
        assert!(!bar.visible);
    }

    #[test]
    fn given_searchbar_when_enter_search_then_becomes_visible() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        assert!(bar.visible);
    }

    #[test]
    fn given_visible_searchbar_when_exit_search_then_becomes_hidden() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        bar.update(Action::ExitSearch).unwrap();
        assert!(!bar.visible);
    }

    // ── phrase buffer ─────────────────────────────────────────────────────────

    #[test]
    fn given_visible_searchbar_when_chars_typed_then_phrase_accumulates() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        type_char(&mut bar, 'f');
        type_char(&mut bar, 'o');
        type_char(&mut bar, 'o');
        assert_eq!(bar.phrase, "foo");
    }

    #[test]
    fn given_phrase_with_chars_when_backspace_then_last_char_removed() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        type_char(&mut bar, 'f');
        type_char(&mut bar, 'o');
        press_backspace(&mut bar);
        assert_eq!(bar.phrase, "f");
    }

    #[test]
    fn given_empty_phrase_when_backspace_then_no_panic() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        press_backspace(&mut bar); // must not panic
        assert_eq!(bar.phrase, "");
    }

    #[test]
    fn given_enter_search_when_called_twice_then_phrase_cleared() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        type_char(&mut bar, 'x');
        enter_search(&mut bar); // second EnterSearch clears phrase
        assert_eq!(bar.phrase, "");
    }

    // ── key events ignored when hidden ────────────────────────────────────────

    #[test]
    fn given_hidden_searchbar_when_char_typed_then_phrase_unchanged() {
        let mut bar = make_searchbar();
        // not visible — key must be ignored
        type_char(&mut bar, 'x');
        assert_eq!(bar.phrase, "");
    }

    // ── counter display ───────────────────────────────────────────────────────

    #[test]
    fn given_no_results_when_right_segment_then_empty_string() {
        let bar = make_searchbar();
        assert_eq!(bar.right_segment(), "");
    }

    #[test]
    fn given_search_bar_update_when_received_then_counter_updated() {
        let mut bar = make_searchbar();
        bar.update(Action::SearchBarUpdate {
            current_match: 3,
            total_matches: 10,
        })
        .unwrap();
        assert_eq!(bar.right_segment(), "(3/10)");
    }

    #[test]
    fn given_phrase_when_left_segment_then_shows_slash_prefix() {
        let mut bar = make_searchbar();
        enter_search(&mut bar);
        type_char(&mut bar, 'e');
        type_char(&mut bar, 'r');
        type_char(&mut bar, 'r');
        assert_eq!(bar.left_segment(), "/ err");
    }

    // ── action emission ───────────────────────────────────────────────────────

    #[test]
    fn given_visible_searchbar_when_enter_pressed_then_search_commit_sent() {
        use tokio::sync::mpsc;

        let mut bar = make_searchbar();
        let (tx, mut rx) = mpsc::unbounded_channel();
        bar.register_action_handler(tx).unwrap();

        enter_search(&mut bar);
        type_char(&mut bar, 'f');
        type_char(&mut bar, 'o');
        type_char(&mut bar, 'o');

        bar.handle_key_event(KeyEvent::from(KeyCode::Enter))
            .unwrap();

        let action = rx.try_recv().unwrap();
        assert_eq!(action, Action::SearchRequest("foo".to_string()));
    }

    #[test]
    fn given_visible_searchbar_when_esc_pressed_then_exit_search_sent() {
        use tokio::sync::mpsc;

        let mut bar = make_searchbar();
        let (tx, mut rx) = mpsc::unbounded_channel();
        bar.register_action_handler(tx).unwrap();

        enter_search(&mut bar);

        bar.handle_key_event(KeyEvent::from(KeyCode::Esc)).unwrap();

        let action = rx.try_recv().unwrap();
        assert_eq!(action, Action::ExitSearch);
    }
}
