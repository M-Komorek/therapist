use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, AppAction, ViewAction};

pub fn action_for(key: KeyEvent) -> Option<Action> {
    app_action_for(key)
        .map(Action::App)
        .or_else(|| view_action_for(key).map(Action::View))
}

fn app_action_for(key: KeyEvent) -> Option<AppAction> {
    let app_action = match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) => AppAction::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => AppAction::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => AppAction::Suspend,
        #[cfg(debug_assertions)]
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => AppAction::DebugPanic,
        _ => return None,
    };
    Some(app_action)
}

fn view_action_for(key: KeyEvent) -> Option<ViewAction> {
    let view_action = match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => ViewAction::MoveDown(1),
        (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => ViewAction::MoveUp(1),
        _ => return None,
    };
    Some(view_action)
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
    fn given_j_or_down_arrow_when_mapping_then_cursor_moves_down_one_line() {
        let down_arrow = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let move_down = Some(Action::View(ViewAction::MoveDown(1)));

        assert_eq!(action_for(key('j')), move_down);
        assert_eq!(action_for(down_arrow), move_down);
    }

    #[test]
    fn given_k_or_up_arrow_when_mapping_then_cursor_moves_up_one_line() {
        let up_arrow = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let move_up = Some(Action::View(ViewAction::MoveUp(1)));

        assert_eq!(action_for(key('k')), move_up);
        assert_eq!(action_for(up_arrow), move_up);
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
