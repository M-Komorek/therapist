use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    // Platform
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,

    // LogView
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollRight(usize),
    ScrollLeft(usize),
    HalfPageUp,
    HalfPageDown,
    GoTop,
    GoBottom,

    // StatusBar
    StatusUpdate {
        current_line: usize,
        total_lines: usize,
        memory_consumption: usize,
    },
}
