use ratatui::layout::{Constraint, Layout, Rect};

pub struct ScreenAreas {
    pub log_view: Rect,
    pub bottom_row: Rect,
}

impl ScreenAreas {
    pub fn new(screen: Rect) -> Self {
        let [log_view, bottom_row] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(screen);
        Self {
            log_view,
            bottom_row,
        }
    }
}
