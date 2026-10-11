mod matching;
mod state;

pub use matching::find_matching_lines;
pub use state::{MatchJump, SearchError, SearchState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    pub phrase: String,
    pub direction: SearchDirection,
}

impl SearchDirection {
    pub fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }

    pub fn prefix(self) -> char {
        match self {
            Self::Forward => '/',
            Self::Backward => '?',
        }
    }
}
