use ratatui::layout::{Rect, Size};

use crate::{
    action::{Action, AppAction, ViewAction},
    keymap::{KeyResolution, Keymap},
    session::Session,
    terminal::Event,
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

    pub fn handle(&mut self, event: Event) -> EventEffect {
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

    fn update_viewport_size(&mut self, screen_size: Size) {
        let log_view = ScreenAreas::new(Rect::from(screen_size)).log_view;
        let tab = &mut self.session.tab;
        tab.set_viewport_height(usize::from(log_view.height));
        tab.set_viewport_width(usize::from(log_view.width));
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

    fn make_app() -> App {
        App::new(
            Session::from_text("first\nsecond\nthird\n"),
            Size::new(20, 4),
        )
    }

    fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> EventEffect {
        app.handle(Event::Key(KeyEvent::new(code, modifiers)))
    }

    #[test]
    fn given_app_when_q_is_pressed_then_it_quits() {
        let mut app = make_app();

        let effect = press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);

        assert_eq!(effect, EventEffect::Quit);
    }

    #[test]
    fn given_app_when_ctrl_z_is_pressed_then_it_suspends() {
        let mut app = make_app();

        let effect = press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL);

        assert_eq!(effect, EventEffect::Suspend);
    }

    #[test]
    fn given_unmapped_key_shown_when_it_is_pressed_again_then_nothing_happens() {
        let mut app = make_app();
        press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);

        let effect = press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);

        assert_eq!(effect, EventEffect::Nothing);
    }

    #[test]
    fn given_app_when_j_is_pressed_then_cursor_moves_down_and_screen_is_redrawn() {
        let mut app = make_app();

        let effect = press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_cursor_on_second_line_when_k_is_pressed_then_cursor_moves_up() {
        let mut app = make_app();
        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.cursor(), 0);
    }

    #[test]
    fn given_app_when_count_digit_is_pressed_then_screen_is_redrawn_without_moving() {
        let mut app = make_app();

        let effect = press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 0);
    }

    #[test]
    fn given_pending_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        let mut app = make_app();
        press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);

        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut app = make_app();
        for _ in 0..2 {
            press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        }

        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_new_app_when_j_is_pressed_then_the_view_does_not_scroll() {
        let mut app = make_app();

        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.viewport().top(), 0);
    }

    #[test]
    fn given_app_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        let mut app = make_app();

        press(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT);

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line_number() {
        let mut app = make_app();
        press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);

        press(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT);

        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_app_when_ctrl_d_is_pressed_then_cursor_moves_half_a_page() {
        let mut app = make_app();

        let effect = press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);

        assert_eq!(effect, EventEffect::Redraw);
        assert_eq!(app.session.tab.cursor(), 1);
    }

    #[test]
    fn given_count_when_l_then_h_are_pressed_then_view_scrolls_right_then_left() {
        let mut app = make_app();
        press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);

        press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.viewport().left_column(), 2);
    }

    #[test]
    fn given_terminal_resized_to_100_columns_when_capital_l_is_pressed_then_view_scrolls_20_columns()
     {
        let mut app = make_app();
        app.handle(Event::Resize(Size::new(100, 10)));

        press(&mut app, KeyCode::Char('L'), KeyModifiers::SHIFT);

        assert_eq!(app.session.tab.viewport().left_column(), 20);
    }

    #[test]
    fn given_app_when_terminal_is_resized_then_screen_is_redrawn() {
        let mut app = make_app();

        let effect = app.handle(Event::Resize(Size::new(30, 10)));

        assert_eq!(effect, EventEffect::Redraw);
    }

    #[test]
    fn given_cursor_on_last_line_when_window_shrinks_then_view_scrolls_to_keep_it_visible() {
        let mut app = make_app();
        for _ in 0..2 {
            press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        }

        app.handle(Event::Resize(Size::new(20, 2)));

        assert_eq!(app.session.tab.viewport().top(), 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut app = make_app();

        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    }
}
