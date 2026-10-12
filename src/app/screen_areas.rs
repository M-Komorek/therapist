use ratatui::layout::{Constraint, Layout, Rect};

pub struct ScreenAreas {
    pub tab_bar: Rect,
    pub log_view: Rect,
    pub bottom_row: Rect,
}

impl ScreenAreas {
    pub fn new(screen: Rect) -> Self {
        let [tab_bar, log_view, bottom_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(screen);
        Self {
            tab_bar,
            log_view,
            bottom_row,
        }
    }
}
