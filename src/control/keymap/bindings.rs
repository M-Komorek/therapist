use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    action::{
        Action, AppAction, BookmarkAction, CommandLineKind, HighlightAction, LogViewAction,
        SearchAction, TabAction,
    },
    domain::search::SearchDirection,
};

pub struct KeyBinding {
    pub key_sequence: &'static [KeyEvent],
    action: Action,
    pub description: &'static str,
}

pub struct KeyBindingGroup {
    pub title: &'static str,
    pub bindings: &'static [KeyBinding],
}

pub const KEY_BINDING_GROUPS: &[KeyBindingGroup] = &[
    KeyBindingGroup {
        title: "Moving",
        bindings: &[
            KeyBinding {
                key_sequence: &[char_key('j')],
                action: log_view(LogViewAction::MoveCursorDown(1)),
                description: "Move the cursor down [count] lines",
            },
            KeyBinding {
                key_sequence: &[char_key('k')],
                action: log_view(LogViewAction::MoveCursorUp(1)),
                description: "Move the cursor up [count] lines",
            },
            KeyBinding {
                key_sequence: &[char_key('g'), char_key('g')],
                action: log_view(LogViewAction::MoveCursorToLineNumber(1)),
                description: "Go to the first line",
            },
            KeyBinding {
                key_sequence: &[char_key('G')],
                action: log_view(LogViewAction::MoveCursorToLastLine),
                description: "Go to the last line, or to line [count]",
            },
            KeyBinding {
                key_sequence: &[ctrl('d')],
                action: log_view(LogViewAction::ScrollHalfPageDown),
                description: "Scroll half a page down",
            },
            KeyBinding {
                key_sequence: &[ctrl('u')],
                action: log_view(LogViewAction::ScrollHalfPageUp),
                description: "Scroll half a page up",
            },
            KeyBinding {
                key_sequence: &[ctrl('f')],
                action: log_view(LogViewAction::ScrollPageDown),
                description: "Scroll a page down",
            },
            KeyBinding {
                key_sequence: &[ctrl('b')],
                action: log_view(LogViewAction::ScrollPageUp),
                description: "Scroll a page up",
            },
        ],
    },
    KeyBindingGroup {
        title: "Scrolling sideways",
        bindings: &[
            KeyBinding {
                key_sequence: &[char_key('h')],
                action: log_view(LogViewAction::ScrollLeftWide(1)),
                description: "Scroll left [count] wide steps",
            },
            KeyBinding {
                key_sequence: &[char_key('l')],
                action: log_view(LogViewAction::ScrollRightWide(1)),
                description: "Scroll right [count] wide steps",
            },
            KeyBinding {
                key_sequence: &[char_key('0')],
                action: log_view(LogViewAction::ScrollToFirstColumn),
                description: "Scroll back to the first column",
            },
        ],
    },
    KeyBindingGroup {
        title: "Searching",
        bindings: &[
            KeyBinding {
                key_sequence: &[char_key('/')],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Search(
                    SearchDirection::Forward,
                ))),
                description: "Search forward",
            },
            KeyBinding {
                key_sequence: &[char_key('?')],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Search(
                    SearchDirection::Backward,
                ))),
                description: "Search backward",
            },
            KeyBinding {
                key_sequence: &[char_key('n')],
                action: search(SearchAction::JumpToNextMatch),
                description: "Jump to the next match",
            },
            KeyBinding {
                key_sequence: &[char_key('N')],
                action: search(SearchAction::JumpToPreviousMatch),
                description: "Jump to the previous match",
            },
            KeyBinding {
                key_sequence: &[plain(KeyCode::Esc)],
                action: search(SearchAction::Clear),
                description: "Clear the search",
            },
        ],
    },
    KeyBindingGroup {
        title: "Bookmarks",
        bindings: &[
            KeyBinding {
                key_sequence: &[space(), char_key('b')],
                action: bookmark(BookmarkAction::Toggle),
                description: "Toggle the bookmark on the cursor line",
            },
            KeyBinding {
                key_sequence: &[space(), char_key('B')],
                action: bookmark(BookmarkAction::RemoveAll),
                description: "Remove all bookmarks",
            },
            KeyBinding {
                key_sequence: &[char_key(']')],
                action: bookmark(BookmarkAction::JumpToNext),
                description: "Jump to the next bookmark",
            },
            KeyBinding {
                key_sequence: &[char_key('[')],
                action: bookmark(BookmarkAction::JumpToPrevious),
                description: "Jump to the previous bookmark",
            },
        ],
    },
    KeyBindingGroup {
        title: "Tabs",
        bindings: &[
            KeyBinding {
                key_sequence: &[space(), char_key('g')],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Grep)),
                description: "Grep the active tab's lines into a new tab",
            },
            KeyBinding {
                key_sequence: &[char_key('H')],
                action: tab(TabAction::SwitchToPrevious),
                description: "Switch to the previous tab",
            },
            KeyBinding {
                key_sequence: &[char_key('L')],
                action: tab(TabAction::SwitchToNext),
                description: "Switch to the next tab",
            },
            KeyBinding {
                key_sequence: &[space(), char_key('x')],
                action: tab(TabAction::CloseActive),
                description: "Close the active tab",
            },
            KeyBinding {
                key_sequence: &[space(), char_key('X')],
                action: tab(TabAction::CloseAllGrepTabs),
                description: "Close every grep tab",
            },
        ],
    },
    KeyBindingGroup {
        title: "Highlights",
        bindings: &[
            KeyBinding {
                key_sequence: &[space(), char_key('h')],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Highlight)),
                description: "Toggle the highlight of a phrase",
            },
            KeyBinding {
                key_sequence: &[space(), char_key('H')],
                action: highlight(HighlightAction::RemoveAll),
                description: "Remove all highlights",
            },
        ],
    },
    KeyBindingGroup {
        title: "General",
        bindings: &[
            KeyBinding {
                key_sequence: &[plain(KeyCode::F(1))],
                action: app(AppAction::OpenHelp),
                description: "Show this help",
            },
            KeyBinding {
                key_sequence: &[ctrl('z')],
                action: app(AppAction::Suspend),
                description: "Suspend to the shell",
            },
            KeyBinding {
                key_sequence: &[char_key('q')],
                action: app(AppAction::Quit),
                description: "Quit",
            },
            #[cfg(debug_assertions)]
            KeyBinding {
                key_sequence: &[ctrl('p')],
                action: app(AppAction::DebugPanic),
                description: "Panic on purpose (debug builds)",
            },
        ],
    },
];

pub(super) enum BindingMatch {
    Complete(&'static Action),
    Partial,
    NotBound,
}

pub(super) fn find_binding(keys: &[KeyEvent]) -> BindingMatch {
    let complete_binding = all_key_bindings().find(|key_binding| key_binding.key_sequence == keys);
    if let Some(key_binding) = complete_binding {
        return BindingMatch::Complete(&key_binding.action);
    }
    let is_prefix =
        all_key_bindings().any(|key_binding| key_binding.key_sequence.starts_with(keys));
    if is_prefix {
        BindingMatch::Partial
    } else {
        BindingMatch::NotBound
    }
}

pub(super) fn apply_count(action: Action, count: Option<usize>) -> Action {
    let Some(count) = count else {
        return action;
    };
    match action {
        Action::LogView(LogViewAction::MoveCursorDown(_)) => {
            log_view(LogViewAction::MoveCursorDown(count))
        }
        Action::LogView(LogViewAction::MoveCursorUp(_)) => {
            log_view(LogViewAction::MoveCursorUp(count))
        }
        Action::LogView(LogViewAction::ScrollLeftWide(_)) => {
            log_view(LogViewAction::ScrollLeftWide(count))
        }
        Action::LogView(LogViewAction::ScrollRightWide(_)) => {
            log_view(LogViewAction::ScrollRightWide(count))
        }
        Action::LogView(LogViewAction::MoveCursorToLastLine) => {
            log_view(LogViewAction::MoveCursorToLineNumber(count))
        }
        _ => action,
    }
}

fn all_key_bindings() -> impl Iterator<Item = &'static KeyBinding> {
    KEY_BINDING_GROUPS
        .iter()
        .flat_map(|key_binding_group| key_binding_group.bindings)
}

pub(super) const fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) const fn char_key(character: char) -> KeyEvent {
    plain(KeyCode::Char(character))
}

pub(super) const fn space() -> KeyEvent {
    char_key(' ')
}

pub(super) const fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

pub(super) const fn app(app_action: AppAction) -> Action {
    Action::App(app_action)
}

pub(super) const fn log_view(log_view_action: LogViewAction) -> Action {
    Action::LogView(log_view_action)
}

pub(super) const fn search(search_action: SearchAction) -> Action {
    Action::Search(search_action)
}

pub(super) const fn bookmark(bookmark_action: BookmarkAction) -> Action {
    Action::Bookmark(bookmark_action)
}

pub(super) const fn tab(tab_action: TabAction) -> Action {
    Action::Tab(tab_action)
}

pub(super) const fn highlight(highlight_action: HighlightAction) -> Action {
    Action::Highlight(highlight_action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_every_pair_of_key_sequences_when_compared_then_neither_equals_nor_starts_the_other() {
        let key_sequences: Vec<&[KeyEvent]> = all_key_bindings()
            .map(|key_binding| key_binding.key_sequence)
            .collect();

        for (sequence_position, keys) in key_sequences.iter().enumerate() {
            for other_keys in &key_sequences[sequence_position + 1..] {
                assert!(
                    !keys.starts_with(other_keys) && !other_keys.starts_with(keys),
                    "{keys:?} conflicts with {other_keys:?}"
                );
            }
        }
    }

    #[test]
    fn given_every_pair_of_key_bindings_when_compared_then_their_actions_differ() {
        let actions: Vec<&Action> = all_key_bindings()
            .map(|key_binding| &key_binding.action)
            .collect();

        for (action_position, action) in actions.iter().enumerate() {
            assert!(
                !actions[action_position + 1..].contains(action),
                "{action:?} is bound twice"
            );
        }
    }
}
