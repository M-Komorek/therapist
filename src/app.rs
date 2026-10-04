use ratatui::layout::{Rect, Size};

use crate::{
    action::{Action, AppAction, ViewAction},
    domain::{jobs::JobMessage, log_file::Indexing, session::Session},
    infra::terminal::Event,
    keymap::{KeyResolution, Keymap},
    ui::ScreenAreas,
};

#[derive(Debug, PartialEq)]
pub enum EventEffect {
    Nothing,
    Redraw,
    Suspend,
    Quit,
}

pub struct App {
    session: Session,
    keymap: Keymap,
}

impl App {
    pub fn new(session: Session, screen_size: Size) -> Self {
        let mut app = Self {
            session,
            keymap: Keymap::default(),
        };
        app.update_viewport_size(screen_size);
        app
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn last_keys_text(&self) -> String {
        self.keymap.last_keys_text()
    }

    pub fn handle_event(&mut self, event: Event) -> EventEffect {
        match event {
            Event::Key(key) => match self.keymap.resolve(key) {
                KeyResolution::Action(action) => self.dispatch(action),
                KeyResolution::LastKeysChanged => EventEffect::Redraw,
                KeyResolution::Ignored => EventEffect::Nothing,
            },
            Event::Resize(screen_size) => {
                self.update_viewport_size(screen_size);
                EventEffect::Redraw
            }
        }
    }

    pub fn handle_job_message(&mut self, job_message: JobMessage) -> EventEffect {
        let log_file = &mut self.session.log_file;
        match job_message {
            JobMessage::IndexProgress(line_index) => log_file.set_line_index(line_index),
            JobMessage::Done => {
                tracing::info!(line_count = log_file.line_count(), "file indexed");
                log_file.set_indexing(Indexing::Done);
            }
            JobMessage::Failed(error) => {
                tracing::error!(%error, "indexing failed");
                log_file.set_indexing(Indexing::Failed);
            }
        }
        EventEffect::Redraw
    }

    pub fn has_running_jobs(&self) -> bool {
        self.session.log_file.indexing() == Indexing::Running
    }

    fn update_viewport_size(&mut self, screen_size: Size) {
        let log_view_area = ScreenAreas::new(Rect::from(screen_size)).log_view;
        let tab = &mut self.session.tab;
        tab.set_viewport_height(usize::from(log_view_area.height));
        tab.set_viewport_width(usize::from(log_view_area.width));
    }

    fn dispatch(&mut self, action: Action) -> EventEffect {
        match action {
            Action::App(app_action) => dispatch_app(app_action),
            Action::View(view_action) => self.dispatch_view(view_action),
        }
    }

    fn dispatch_view(&mut self, view_action: ViewAction) -> EventEffect {
        let line_count = self.session.log_file.line_count();
        let tab = &mut self.session.tab;
        match view_action {
            ViewAction::MoveUp(lines_to_move) => tab.move_up(lines_to_move),
            ViewAction::MoveDown(lines_to_move) => tab.move_down(lines_to_move, line_count),
            ViewAction::GoToLineNumber(line_number) => {
                tab.go_to_line_number(line_number, line_count)
            }
            ViewAction::GoToLastLine => tab.go_to_last_line(line_count),
            ViewAction::HalfPageUp => tab.half_page_up(),
            ViewAction::HalfPageDown => tab.half_page_down(line_count),
            ViewAction::PageUp => tab.page_up(),
            ViewAction::PageDown => tab.page_down(line_count),
            ViewAction::ScrollLeft(columns) => tab.scroll_left(columns),
            ViewAction::ScrollRight(columns) => tab.scroll_right(columns),
            ViewAction::ScrollLeftWide => tab.scroll_left_wide(),
            ViewAction::ScrollRightWide => tab.scroll_right_wide(),
            ViewAction::ScrollToFirstColumn => tab.scroll_to_first_column(),
        }
        EventEffect::Redraw
    }
}

fn dispatch_app(app_action: AppAction) -> EventEffect {
    match app_action {
        AppAction::Quit => EventEffect::Quit,
        AppAction::Suspend => EventEffect::Suspend,
        #[cfg(debug_assertions)]
        AppAction::DebugPanic => panic!("Debug panic key pressed"),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::domain::{jobs::JobError, log_file::LineIndex};

    fn make_app() -> App {
        App::new(
            Session::from_text("first\nsecond\nthird\n"),
            Size::new(20, 4),
        )
    }

    fn make_indexing_app(text: &str) -> (App, Vec<LineIndex>) {
        let session = Session::unindexed_from_text(text);
        let mut reported_indexes = Vec::new();
        session
            .log_file
            .indexer()
            .run(|line_index| reported_indexes.push(line_index))
            .unwrap();
        (App::new(session, Size::new(20, 4)), reported_indexes)
    }

    fn deliver(app: &mut App, reported_indexes: Vec<LineIndex>) {
        for line_index in reported_indexes {
            app.handle_job_message(JobMessage::IndexProgress(line_index));
        }
    }

    fn type_keys(app: &mut App, typed: &str) -> EventEffect {
        let mut effect = EventEffect::Nothing;
        for character in typed.chars() {
            let modifiers = if character.is_uppercase() {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            effect = app.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                modifiers,
            )));
        }
        effect
    }

    fn press_ctrl(app: &mut App, character: char) -> EventEffect {
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char(character),
            KeyModifiers::CONTROL,
        )))
    }

    #[test]
    fn given_app_when_q_is_pressed_then_it_quits() {
        let mut app = make_app();

        let effect = type_keys(&mut app, "q");

        assert_eq!(effect, EventEffect::Quit);
    }

    #[test]
    fn given_app_when_ctrl_z_is_pressed_then_it_suspends() {
        let mut app = make_app();

        let effect = press_ctrl(&mut app, 'z');

        assert_eq!(effect, EventEffect::Suspend);
    }

    #[test]
    fn given_unmapped_key_shown_when_it_is_pressed_again_then_nothing_happens() {
        let mut app = make_app();
        type_keys(&mut app, "x");

        let effect = type_keys(&mut app, "x");

        assert_eq!(effect, EventEffect::Nothing);
    }

    #[test]
    fn given_app_when_j_is_pressed_then_cursor_moves_down_and_screen_is_redrawn() {
        let mut app = make_app();

        let effect = type_keys(&mut app, "j");

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_cursor_on_second_line_when_k_is_pressed_then_cursor_moves_up() {
        let mut app = make_app();
        type_keys(&mut app, "j");

        type_keys(&mut app, "k");

        assert_eq!(app.session.tab.cursor(), 0);
    }

    #[test]
    fn given_app_when_count_digit_is_pressed_then_screen_is_redrawn_without_moving() {
        let mut app = make_app();

        let effect = type_keys(&mut app, "2");

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 0);
    }

    #[test]
    fn given_pending_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        let mut app = make_app();
        type_keys(&mut app, "2");

        type_keys(&mut app, "j");

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut app = make_app();
        type_keys(&mut app, "jj");

        type_keys(&mut app, "j");

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_new_app_when_j_is_pressed_then_the_view_does_not_scroll() {
        let mut app = make_app();

        type_keys(&mut app, "j");

        assert_eq!(app.session.tab.viewport().top(), 0);
    }

    #[test]
    fn given_app_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        let mut app = make_app();

        type_keys(&mut app, "G");

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line_number() {
        let mut app = make_app();
        type_keys(&mut app, "2");

        type_keys(&mut app, "G");

        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_app_when_ctrl_d_is_pressed_then_cursor_moves_half_a_page() {
        let mut app = make_app();

        let effect = press_ctrl(&mut app, 'd');

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_count_when_l_then_h_are_pressed_then_view_scrolls_right_then_left() {
        let mut app = make_app();
        type_keys(&mut app, "3");
        type_keys(&mut app, "l");

        type_keys(&mut app, "h");

        assert_eq!(app.session.tab.viewport().left_column(), 2);
    }

    #[test]
    fn given_terminal_resized_to_100_columns_when_capital_l_is_pressed_then_view_scrolls_20_columns()
     {
        let mut app = make_app();
        app.handle_event(Event::Resize(Size::new(100, 10)));

        type_keys(&mut app, "L");

        assert_eq!(app.session.tab.viewport().left_column(), 20);
    }

    #[test]
    fn given_app_when_terminal_is_resized_then_screen_is_redrawn() {
        let mut app = make_app();

        let effect = app.handle_event(Event::Resize(Size::new(30, 10)));

        assert_eq!(effect, EventEffect::Redraw);
    }

    #[test]
    fn given_cursor_on_last_line_when_window_shrinks_then_view_scrolls_to_keep_it_visible() {
        let mut app = make_app();
        type_keys(&mut app, "jj");

        app.handle_event(Event::Resize(Size::new(20, 2)));

        assert_eq!(app.session.tab.viewport().top(), 2);
    }

    #[test]
    fn given_unindexed_file_when_app_starts_then_a_job_is_running() {
        let (app, _reported_indexes) = make_indexing_app("a\nb\n");

        assert!(app.has_running_jobs());
    }

    #[test]
    fn given_indexing_app_when_progress_arrives_then_the_screen_is_redrawn() {
        let (mut app, mut reported_indexes) = make_indexing_app("a\nb\n");

        let effect = app.handle_job_message(JobMessage::IndexProgress(reported_indexes.remove(0)));

        assert_eq!(effect, EventEffect::Redraw);
    }

    #[test]
    fn given_progress_delivered_when_capital_g_is_pressed_then_cursor_goes_to_last_indexed_line() {
        let (mut app, reported_indexes) = make_indexing_app("first\nsecond\nthird\n");
        deliver(&mut app, reported_indexes);

        type_keys(&mut app, "G");

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_indexing_app_when_done_arrives_then_no_job_is_running() {
        let (mut app, reported_indexes) = make_indexing_app("a\nb\n");
        deliver(&mut app, reported_indexes);

        app.handle_job_message(JobMessage::Done);

        assert!(!app.has_running_jobs());
    }

    #[test]
    fn given_indexing_app_when_indexing_fails_then_no_job_is_running_and_indexed_lines_stay() {
        let (mut app, reported_indexes) = make_indexing_app("a\nb\n");
        deliver(&mut app, reported_indexes);

        app.handle_job_message(JobMessage::Failed(JobError::Panicked));

        assert!(!app.has_running_jobs());
        assert_eq!(app.session.log_file.line_count(), 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut app = make_app();

        press_ctrl(&mut app, 'p');
    }
}
