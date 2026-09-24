use std::path::{Path, PathBuf};

use crate::{
    action::{Action, AppAction},
    keymap,
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
    file: PathBuf,
}

impl App {
    pub fn new(file: PathBuf) -> Self {
        Self { file }
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn handle(&mut self, event: Event) -> EventEffect {
        match event {
            Event::Key(key) => keymap::action_for(key).map_or(EventEffect::Nothing, dispatch),
            Event::Resize => EventEffect::Redraw,
        }
    }
}

fn dispatch(action: Action) -> EventEffect {
    match action {
        Action::App(AppAction::Quit) => EventEffect::Quit,
        Action::App(AppAction::Suspend) => EventEffect::Suspend,
        #[cfg(debug_assertions)]
        Action::App(AppAction::DebugPanic) => panic!("Debug panic key pressed"),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    fn make_app() -> App {
        App::new(PathBuf::from("app.log"))
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
