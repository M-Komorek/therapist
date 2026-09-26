use crate::{
    action::{Action, AppAction, ViewAction},
    keymap,
    session::Session,
    terminal::Event,
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
}

impl App {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn handle(&mut self, event: Event) -> EventEffect {
        match event {
            Event::Key(key) => {
                keymap::action_for(key).map_or(EventEffect::Nothing, |action| self.dispatch(action))
            }
            Event::Resize => EventEffect::Redraw,
        }
    }

    fn dispatch(&mut self, action: Action) -> EventEffect {
        match action {
            Action::App(app_action) => dispatch_app(app_action),
            Action::View(view_action) => self.dispatch_view(view_action),
        }
    }

    fn dispatch_view(&mut self, view_action: ViewAction) -> EventEffect {
        match view_action {
            ViewAction::MoveUp(lines_to_move) => self.session.tab.move_up(lines_to_move),
            ViewAction::MoveDown(lines_to_move) => self
                .session
                .tab
                .move_down(lines_to_move, self.session.log_file.line_count()),
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
        App::new(Session::from_text("first\nsecond\nthird\n"))
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
    fn given_app_when_unmapped_key_is_pressed_then_nothing_happens() {
        let mut app = make_app();

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
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut app = make_app();
        for _ in 0..2 {
            press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        }

        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);

        assert_eq!(app.session.tab.cursor(), 2);
    }

    #[test]
    fn given_app_when_terminal_is_resized_then_screen_is_redrawn() {
        let mut app = make_app();

        let effect = app.handle(Event::Resize);

        assert_eq!(effect, EventEffect::Redraw);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Debug panic key pressed")]
    fn given_debug_build_when_ctrl_p_is_pressed_then_app_panics() {
        let mut app = make_app();

        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    }
}
