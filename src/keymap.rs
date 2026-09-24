use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction};

pub fn action_for(key: KeyEvent) -> Option<Action> {
    let app_action = match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) => AppAction::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => AppAction::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => AppAction::Suspend,
        #[cfg(debug_assertions)]
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => AppAction::DebugPanic,
        _ => return None,
    };
    Some(Action::App(app_action))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
    }

    fn ctrl(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    #[test]
    fn given_q_when_mapping_then_app_quits() {
        assert_eq!(action_for(key('q')), Some(Action::App(AppAction::Quit)));
    }

    #[test]
    fn given_ctrl_c_when_mapping_then_app_quits() {
        assert_eq!(action_for(ctrl('c')), Some(Action::App(AppAction::Quit)));
    }

    #[test]
    fn given_ctrl_z_when_mapping_then_app_suspends() {
        assert_eq!(action_for(ctrl('z')), Some(Action::App(AppAction::Suspend)));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn given_ctrl_p_in_debug_build_when_mapping_then_app_panics_on_purpose() {
        assert_eq!(
            action_for(ctrl('p')),
            Some(Action::App(AppAction::DebugPanic))
        );
    }

    #[test]
    fn given_unmapped_key_when_mapping_then_there_is_no_action() {
        assert_eq!(action_for(key('x')), None);
    }

    #[test]
    fn given_mapped_key_with_other_modifier_when_mapping_then_there_is_no_action() {
        assert_eq!(action_for(ctrl('q')), None);
    }
}
