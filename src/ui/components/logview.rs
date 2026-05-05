use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Rect, Size},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::contract::Component;

use crate::{action::Action, domain::source::LogSource};

pub struct LogView {
    source: Arc<dyn LogSource>,
    viewport_top: usize,
    viewport_height: usize,
    viewport_width: usize,
    horizontal_scroll: usize,
    selected_line: usize,
}

impl LogView {
    const LINE_NUMBER_SUFIX: &str = ": ";

    pub fn new(source: Arc<dyn LogSource>) -> Self {
        Self {
            source,
            viewport_top: 0,
            viewport_height: 0,
            viewport_width: 0,
            horizontal_scroll: 0,
            selected_line: 0,
        }
    }

    fn total_lines(&self) -> usize {
        self.source.line_count()
    }

    fn last_line_idx(&self) -> usize {
        self.source.line_count().saturating_sub(1)
    }

    fn clamp_and_scroll_viewport(&mut self) {
        let last = self.last_line_idx();
        self.selected_line = self.selected_line.min(last);

        // Scroll down if selected line is below the viewport.
        if self.selected_line >= self.viewport_top + self.viewport_height {
            self.viewport_top = self.selected_line + 1 - self.viewport_height;
        }

        // Scroll up if selected line is above the viewport.
        if self.selected_line < self.viewport_top {
            self.viewport_top = self.selected_line;
        }
    }

    fn move_up(&mut self, lines: usize) {
        self.selected_line = self.selected_line.saturating_sub(lines);
        self.clamp_and_scroll_viewport();
    }

    fn move_down(&mut self, lines: usize) {
        let last = self.last_line_idx();
        self.selected_line = self.selected_line.saturating_add(lines).min(last);
        self.clamp_and_scroll_viewport();
    }

    fn half_page_up(&mut self) {
        let half = (self.viewport_height / 2).max(1);
        self.move_up(half);
    }

    fn half_page_down(&mut self) {
        let half = (self.viewport_height / 2).max(1);
        self.move_down(half);
    }

    fn go_top(&mut self) {
        self.selected_line = 0;
        self.viewport_top = 0;
    }

    fn go_bottom(&mut self) {
        self.selected_line = self.last_line_idx();
        self.clamp_and_scroll_viewport();
    }

    fn scroll_right(&mut self, cols: usize) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_add(cols);
    }

    fn scroll_left(&mut self, cols: usize) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(cols);
    }

    fn gutter_width(&self) -> usize {
        let digit_count = self.total_lines().max(1).ilog10() as usize + 1;
        digit_count + Self::LINE_NUMBER_SUFIX.chars().count()
    }
}

impl Component for LogView {
    fn init(&mut self, area: Size) -> color_eyre::Result<()> {
        self.viewport_height = area.height as usize;
        self.viewport_width = area.width as usize;
        Ok(())
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::ScrollUp(n) => self.move_up(n),
            Action::ScrollDown(n) => self.move_down(n),
            Action::ScrollRight(n) => self.scroll_right(n),
            Action::ScrollLeft(n) => self.scroll_left(n),
            Action::HalfPageUp => self.half_page_up(),
            Action::HalfPageDown => self.half_page_down(),
            Action::GoTop => self.go_top(),
            Action::GoBottom => self.go_bottom(),
            _ => {}
        }

        if let Action::Resize(width, height) = action {
            self.viewport_width = width as usize;
            self.viewport_height = height as usize;
            self.clamp_and_scroll_viewport();
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        self.viewport_height = area.height as usize;
        self.viewport_width = area.width as usize;

        let gutter_width = self.gutter_width();
        let log_width = self.viewport_width.saturating_sub(gutter_width);

        let visible_count = self.viewport_height.min(self.total_lines());
        let batch = self
            .source
            .get_lines(self.viewport_top, self.viewport_top + visible_count);

        let lines: Vec<Line> = batch
            .into_iter()
            .enumerate()
            .map(|(row, content)| {
                let line_idx = self.viewport_top + row;
                let digit_count = gutter_width - Self::LINE_NUMBER_SUFIX.chars().count();

                let gutter = format!(
                    "{:>width$}{}",
                    line_idx + 1,
                    Self::LINE_NUMBER_SUFIX,
                    width = digit_count
                );

                let content_display: String = content
                    .chars()
                    .skip(self.horizontal_scroll)
                    .take(log_width)
                    .collect();

                if line_idx == self.selected_line {
                    let highlight = Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD);
                    let content_display = format!("{:<width$}", content_display, width = log_width);
                    Line::from(vec![
                        Span::styled(gutter, highlight),
                        Span::styled(content_display, highlight),
                    ])
                } else {
                    let gutter_style = Style::default().fg(Color::DarkGray);
                    Line::from(vec![
                        Span::styled(gutter, gutter_style),
                        Span::styled(content_display, Style::default()),
                    ])
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));

        frame.render_widget(paragraph, area);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ratatui::layout::Size;

    use super::{Component, LogView};
    use crate::{action::Action, domain::source::LogSource};

    impl LogView {
        fn selected_line(&self) -> usize {
            self.selected_line
        }

        fn viewport_top(&self) -> usize {
            self.viewport_top
        }

        fn horizontal_scroll(&self) -> usize {
            self.horizontal_scroll
        }

        fn viewport_height(&self) -> usize {
            self.viewport_height
        }
    }

    struct LogSourceMock {
        lines: Vec<String>,
    }

    impl LogSourceMock {
        fn with_line_count(count: usize) -> Self {
            let lines = (0..count).map(|i| format!("line {i}")).collect();
            Self { lines }
        }
    }

    impl LogSource for LogSourceMock {
        fn line_count(&self) -> usize {
            self.lines.len()
        }

        fn get_line(&self, line_idx: usize) -> color_eyre::Result<String> {
            self.lines
                .get(line_idx)
                .cloned()
                .ok_or_else(|| color_eyre::eyre::eyre!("index {line_idx} out of bounds"))
        }

        fn get_lines(&self, start: usize, end: usize) -> Vec<String> {
            let end = end.min(self.lines.len());
            self.lines[start.min(end)..end].to_vec()
        }
    }

    fn make_logview(line_count: usize, viewport_height: usize) -> LogView {
        let source = Arc::new(LogSourceMock::with_line_count(line_count));
        let mut log_view = LogView::new(source);
        log_view
            .init(Size {
                width: 80,
                height: viewport_height as u16,
            })
            .unwrap();
        log_view
    }

    fn scroll_down(log_view: &mut LogView, n: usize) {
        log_view.update(Action::ScrollDown(n)).unwrap();
    }

    fn scroll_up(log_view: &mut LogView, n: usize) {
        log_view.update(Action::ScrollUp(n)).unwrap();
    }

    // --- Vertical navigation ---

    #[test]
    fn given_logview_at_top_when_scroll_down_by_one_then_selected_line_increments() {
        let mut log_view = make_logview(10, 5);
        scroll_down(&mut log_view, 1);
        assert_eq!(log_view.selected_line(), 1);
        assert_eq!(log_view.viewport_top(), 0);
    }

    #[test]
    fn given_logview_at_top_when_scroll_up_then_selected_line_stays_at_zero() {
        let mut log_view = make_logview(10, 5);
        scroll_up(&mut log_view, 1);
        assert_eq!(log_view.selected_line(), 0);
    }

    #[test]
    fn given_logview_at_last_line_when_scroll_down_then_selected_line_stays_at_last() {
        let mut log_view = make_logview(10, 5);
        scroll_down(&mut log_view, 9); // go to line 9
        scroll_down(&mut log_view, 5); // try to go past end
        assert_eq!(log_view.selected_line(), 9);
    }

    #[test]
    fn given_logview_at_top_when_scroll_down_and_up_then_selected_line_follows() {
        let mut log_view = make_logview(10, 5);
        scroll_down(&mut log_view, 5);
        scroll_up(&mut log_view, 2);
        assert_eq!(log_view.selected_line(), 3);
    }

    // --- Viewport scrolling ---

    #[test]
    fn given_selection_at_viewport_bottom_when_scroll_down_then_viewport_top_advances() {
        let mut log_view = make_logview(20, 5);
        scroll_down(&mut log_view, 4); // sel=4, last visible in viewport [0..4]
        scroll_down(&mut log_view, 1); // sel=5, must scroll viewport
        assert_eq!(log_view.selected_line(), 5);
        assert_eq!(log_view.viewport_top(), 1);
    }

    #[test]
    fn given_selection_at_viewport_bottom_when_scroll_up_then_viewport_top_not_advances() {
        let mut log_view = make_logview(20, 5);
        // After scroll_down(5): sel=5, top=1 (viewport shows lines 1..5)
        scroll_down(&mut log_view, 5);
        assert_eq!(log_view.viewport_top(), 1);
        // scroll_up(1): sel=4, still within viewport [1..5] — top stays at 1
        scroll_up(&mut log_view, 1);
        assert_eq!(log_view.selected_line(), 4);
        assert_eq!(log_view.viewport_top(), 1);
    }

    #[test]
    fn given_selection_at_viewport_top_edge_when_scroll_up_then_viewport_top_retreats() {
        let mut log_view = make_logview(20, 5);
        // Manually position: sel=5, top=5 (sel is exactly at top of viewport)
        scroll_down(&mut log_view, 5); // sel=5, top=1
        // Scroll down more to push top up: sel=9, top=5
        scroll_down(&mut log_view, 4);
        assert_eq!(log_view.selected_line(), 9);
        assert_eq!(log_view.viewport_top(), 5);
        // Now scroll up: sel=8, which is above viewport.
        // Scroll up to sel=5 — still visible (top=5, sel=5 is exactly at top).
        scroll_up(&mut log_view, 4);
        assert_eq!(log_view.selected_line(), 5);
        assert_eq!(log_view.viewport_top(), 5);
        // One more up: sel=4, now below viewport_top(5) → top retreats to 4
        scroll_up(&mut log_view, 1);
        assert_eq!(log_view.selected_line(), 4);
        assert_eq!(log_view.viewport_top(), 4);
    }

    #[test]
    fn given_large_jump_down_when_scroll_down_then_viewport_top_follows_selection() {
        let mut log_view = make_logview(20, 5);
        scroll_down(&mut log_view, 15); // sel=15, viewport=[11..15]
        assert_eq!(log_view.selected_line(), 15);
        assert_eq!(log_view.viewport_top(), 11);
    }

    // --- GoTop / GoBottom ---

    #[test]
    fn given_logview_scrolled_down_when_go_top_then_selection_and_viewport_reset() {
        let mut log_view = make_logview(20, 5);
        scroll_down(&mut log_view, 15);
        log_view.update(Action::GoTop).unwrap();
        assert_eq!(log_view.selected_line(), 0);
        assert_eq!(log_view.viewport_top(), 0);
    }

    #[test]
    fn given_logview_at_top_when_go_bottom_then_selection_is_last_line() {
        let mut log_view = make_logview(20, 5);
        log_view.update(Action::GoBottom).unwrap();
        assert_eq!(log_view.selected_line(), 19);
        assert_eq!(log_view.viewport_top(), 15);
    }

    // --- HalfPageDown / HalfPageUp ---

    #[test]
    fn given_logview_at_top_when_half_page_down_then_selection_moves_by_half_viewport() {
        let mut log_view = make_logview(20, 10);
        log_view.update(Action::HalfPageDown).unwrap();
        assert_eq!(log_view.selected_line(), 5);
    }

    #[test]
    fn given_logview_at_mid_when_half_page_up_then_selection_moves_by_half_viewport() {
        let mut log_view = make_logview(20, 10);
        scroll_down(&mut log_view, 10);
        log_view.update(Action::HalfPageUp).unwrap();
        assert_eq!(log_view.selected_line(), 5);
    }

    #[test]
    fn given_viewport_height_zero_when_half_page_down_then_moves_by_at_least_one() {
        let mut log_view = make_logview(10, 0);
        log_view.update(Action::HalfPageDown).unwrap();
        assert_eq!(log_view.selected_line(), 1);
    }

    // --- Horizontal scroll ---

    #[test]
    fn given_logview_at_left_when_scroll_right_then_horizontal_scroll_increases() {
        let mut log_view = make_logview(10, 5);
        log_view.update(Action::ScrollRight(3)).unwrap();
        assert_eq!(log_view.horizontal_scroll(), 3);
    }

    #[test]
    fn given_logview_scrolled_right_when_scroll_left_then_horizontal_scroll_decreases() {
        let mut log_view = make_logview(10, 5);
        log_view.update(Action::ScrollRight(5)).unwrap();
        log_view.update(Action::ScrollLeft(3)).unwrap();
        assert_eq!(log_view.horizontal_scroll(), 2);
    }

    #[test]
    fn given_logview_at_left_edge_when_scroll_left_then_horizontal_scroll_stays_at_zero() {
        let mut log_view = make_logview(10, 5);
        log_view.update(Action::ScrollLeft(1)).unwrap();
        assert_eq!(log_view.horizontal_scroll(), 0);
    }

    // --- Resize ---

    #[test]
    fn given_logview_when_resize_then_viewport_dimensions_update() {
        let mut log_view = make_logview(10, 5);
        log_view.update(Action::Resize(100, 20)).unwrap();
        assert_eq!(log_view.viewport_height(), 20);
    }

    #[test]
    fn given_selection_beyond_new_viewport_when_resize_smaller_then_viewport_top_clamped() {
        let mut log_view = make_logview(20, 10);
        scroll_down(&mut log_view, 15); // sel=15, top=6
        log_view.update(Action::Resize(80, 3)).unwrap();
        // viewport_height=3, sel=15 must be visible: top in [13..15]
        let top = log_view.viewport_top();
        assert!(log_view.selected_line() >= top);
        assert!(log_view.selected_line() < top + 3);
    }

    // --- Gutter width ---

    #[test]
    fn given_zero_lines_when_gutter_width_then_returns_digit_count_for_one_plus_suffix() {
        // 0 lines → max(1) = 1 → 1 digit + 2 suffix chars = 3
        let log_view = make_logview(0, 5);
        assert_eq!(log_view.gutter_width(), 3);
    }

    #[test]
    fn given_nine_lines_when_gutter_width_then_returns_three() {
        // 9 lines → 1 digit + 2 = 3
        let log_view = make_logview(9, 5);
        assert_eq!(log_view.gutter_width(), 3);
    }

    #[test]
    fn given_ten_lines_when_gutter_width_then_returns_four() {
        // 10 lines → 2 digits + 2 = 4
        let log_view = make_logview(10, 5);
        assert_eq!(log_view.gutter_width(), 4);
    }

    #[test]
    fn given_nine_hundred_ninety_nine_lines_when_gutter_width_then_returns_five() {
        // 999 lines → 3 digits + 2 = 5
        let log_view = make_logview(999, 5);
        assert_eq!(log_view.gutter_width(), 5);
    }

    #[test]
    fn given_one_thousand_lines_when_gutter_width_then_returns_six() {
        // 1000 lines → 4 digits + 2 = 6
        let log_view = make_logview(1000, 5);
        assert_eq!(log_view.gutter_width(), 6);
    }

    // --- Edge cases ---

    #[test]
    fn given_empty_source_when_scroll_down_then_no_panic_and_selection_stays_zero() {
        let mut log_view = make_logview(0, 5);
        scroll_down(&mut log_view, 1);
        assert_eq!(log_view.selected_line(), 0);
    }

    #[test]
    fn given_single_line_source_when_go_bottom_then_selection_is_zero() {
        let mut log_view = make_logview(1, 5);
        log_view.update(Action::GoBottom).unwrap();
        assert_eq!(log_view.selected_line(), 0);
    }

    #[test]
    fn given_single_line_source_when_scroll_down_then_selection_stays_at_zero() {
        let mut log_view = make_logview(1, 5);
        scroll_down(&mut log_view, 5);
        assert_eq!(log_view.selected_line(), 0);
    }
}
