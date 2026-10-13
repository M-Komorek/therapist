use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    action::{
        Action, AppAction, BookmarkAction, CommandLineKind, LogViewAction, SearchAction, TabAction,
    },
    domain::search::SearchDirection,
};

pub struct KeyBinding {
    pub key_sequences: &'static [&'static [KeyEvent]],
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
                key_sequences: &[&[char_key('j')], &[plain(KeyCode::Down)]],
                action: log_view(LogViewAction::MoveCursorDown(1)),
                description: "Move the cursor down [count] lines",
            },
            KeyBinding {
                key_sequences: &[&[char_key('k')], &[plain(KeyCode::Up)]],
                action: log_view(LogViewAction::MoveCursorUp(1)),
                description: "Move the cursor up [count] lines",
            },
            KeyBinding {
                key_sequences: &[&[char_key('g'), char_key('g')]],
                action: log_view(LogViewAction::MoveCursorToLineNumber(1)),
                description: "Go to the first line, or to line [count]",
            },
            KeyBinding {
                key_sequences: &[&[char_key('G')]],
                action: log_view(LogViewAction::MoveCursorToLastLine),
                description: "Go to the last line, or to line [count]",
            },
            KeyBinding {
                key_sequences: &[&[ctrl('d')]],
                action: log_view(LogViewAction::ScrollHalfPageDown),
                description: "Scroll half a page down",
            },
            KeyBinding {
                key_sequences: &[&[ctrl('u')]],
                action: log_view(LogViewAction::ScrollHalfPageUp),
                description: "Scroll half a page up",
            },
            KeyBinding {
                key_sequences: &[&[ctrl('f')]],
                action: log_view(LogViewAction::ScrollPageDown),
                description: "Scroll a page down",
            },
            KeyBinding {
                key_sequences: &[&[ctrl('b')]],
                action: log_view(LogViewAction::ScrollPageUp),
                description: "Scroll a page up",
            },
        ],
    },
    KeyBindingGroup {
        title: "Scrolling sideways",
        bindings: &[
            KeyBinding {
                key_sequences: &[&[char_key('h')], &[plain(KeyCode::Left)]],
                action: log_view(LogViewAction::ScrollLeft(1)),
                description: "Scroll left [count] columns",
            },
            KeyBinding {
                key_sequences: &[&[char_key('l')], &[plain(KeyCode::Right)]],
                action: log_view(LogViewAction::ScrollRight(1)),
                description: "Scroll right [count] columns",
            },
            KeyBinding {
                key_sequences: &[&[char_key('H')]],
                action: log_view(LogViewAction::ScrollLeftWide),
                description: "Scroll left by a fifth of the width",
            },
            KeyBinding {
                key_sequences: &[&[char_key('L')]],
                action: log_view(LogViewAction::ScrollRightWide),
                description: "Scroll right by a fifth of the width",
            },
            KeyBinding {
                key_sequences: &[&[char_key('0')]],
                action: log_view(LogViewAction::ScrollToFirstColumn),
                description: "Scroll back to the first column",
            },
        ],
    },
    KeyBindingGroup {
        title: "Searching",
        bindings: &[
            KeyBinding {
                key_sequences: &[&[char_key('/')]],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Search(
                    SearchDirection::Forward,
                ))),
                description: "Search forward",
            },
            KeyBinding {
                key_sequences: &[&[char_key('?')]],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Search(
                    SearchDirection::Backward,
                ))),
                description: "Search backward",
            },
            KeyBinding {
                key_sequences: &[&[char_key('n')]],
                action: search(SearchAction::JumpToNextMatch),
                description: "Jump to the next match",
            },
            KeyBinding {
                key_sequences: &[&[char_key('N')]],
                action: search(SearchAction::JumpToPreviousMatch),
                description: "Jump to the previous match",
            },
            KeyBinding {
                key_sequences: &[&[plain(KeyCode::Esc)]],
                action: search(SearchAction::Clear),
                description: "Clear the search",
            },
        ],
    },
    KeyBindingGroup {
        title: "Bookmarks",
        bindings: &[
            KeyBinding {
                key_sequences: &[&[char_key('b'), char_key('b')]],
                action: bookmark(BookmarkAction::Toggle),
                description: "Toggle the bookmark on the cursor line",
            },
            KeyBinding {
                key_sequences: &[&[char_key('b'), char_key('n')]],
                action: bookmark(BookmarkAction::JumpToNext),
                description: "Jump to the next bookmark",
            },
            KeyBinding {
                key_sequences: &[&[char_key('b'), char_key('p')]],
                action: bookmark(BookmarkAction::JumpToPrevious),
                description: "Jump to the previous bookmark",
            },
        ],
    },
    KeyBindingGroup {
        title: "Grep tabs",
        bindings: &[
            KeyBinding {
                key_sequences: &[&[char_key('&')]],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Grep)),
                description: "Grep the active tab's lines into a new tab",
            },
            KeyBinding {
                key_sequences: &[&[char_key('t'), char_key('n')]],
                action: tab(TabAction::SwitchToNext),
                description: "Switch to the next tab",
            },
            KeyBinding {
                key_sequences: &[&[char_key('t'), char_key('p')]],
                action: tab(TabAction::SwitchToPrevious),
                description: "Switch to the previous tab",
            },
            KeyBinding {
                key_sequences: &[&[char_key('t'), char_key('c')]],
                action: tab(TabAction::CloseActive),
                description: "Close the active tab",
            },
            KeyBinding {
                key_sequences: &[&[char_key('t'), char_key('a')]],
                action: tab(TabAction::ShowCursorLineInAllLinesTab),
                description: "Show the cursor's line in the [all] tab",
            },
        ],
    },
    KeyBindingGroup {
        title: "General",
        bindings: &[
            KeyBinding {
                key_sequences: &[&[char_key(':')]],
                action: app(AppAction::OpenCommandLine(CommandLineKind::Command)),
                description: "Open the command line",
            },
            KeyBinding {
                key_sequences: &[&[plain(KeyCode::F(1))]],
                action: app(AppAction::OpenHelp),
                description: "Show this help",
            },
            KeyBinding {
                key_sequences: &[&[ctrl('z')]],
                action: app(AppAction::Suspend),
                description: "Suspend to the shell",
            },
            KeyBinding {
                key_sequences: &[&[char_key('q')], &[ctrl('c')]],
                action: app(AppAction::Quit),
                description: "Quit",
            },
            #[cfg(debug_assertions)]
            KeyBinding {
                key_sequences: &[&[ctrl('p')]],
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
    let complete_binding =
        all_key_bindings().find(|key_binding| key_binding.key_sequences.contains(&keys));
    if let Some(key_binding) = complete_binding {
        return BindingMatch::Complete(&key_binding.action);
    }
    let is_prefix = all_key_sequences().any(|key_sequence| key_sequence.starts_with(keys));
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
        Action::LogView(LogViewAction::ScrollLeft(_)) => log_view(LogViewAction::ScrollLeft(count)),
        Action::LogView(LogViewAction::ScrollRight(_)) => {
            log_view(LogViewAction::ScrollRight(count))
        }
        Action::LogView(
            LogViewAction::MoveCursorToLineNumber(_) | LogViewAction::MoveCursorToLastLine,
        ) => log_view(LogViewAction::MoveCursorToLineNumber(count)),
        _ => action,
    }
}

fn all_key_bindings() -> impl Iterator<Item = &'static KeyBinding> {
    KEY_BINDING_GROUPS
        .iter()
        .flat_map(|key_binding_group| key_binding_group.bindings)
}

fn all_key_sequences() -> impl Iterator<Item = &'static [KeyEvent]> {
    all_key_bindings().flat_map(|key_binding| key_binding.key_sequences.iter().copied())
}

pub(super) const fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) const fn char_key(character: char) -> KeyEvent {
    plain(KeyCode::Char(character))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_every_pair_of_key_sequences_when_compared_then_neither_equals_nor_starts_the_other() {
        let key_sequences: Vec<&[KeyEvent]> = all_key_sequences().collect();

        for (sequence_position, keys) in key_sequences.iter().enumerate() {
            for other_keys in &key_sequences[sequence_position + 1..] {
                assert!(
                    !keys.starts_with(other_keys) && !other_keys.starts_with(keys),
                    "{keys:?} conflicts with {other_keys:?}"
                );
            }
        }
    }
}
