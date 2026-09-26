#[derive(Default)]
pub struct Tab {
    cursor: usize,
    viewport: Viewport,
}

#[derive(Default)]
pub struct Viewport {
    top: usize,
    height: usize,
}

impl Tab {
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn move_up(&mut self, lines_to_move: usize) {
        self.cursor = self.cursor.saturating_sub(lines_to_move);
        self.viewport.keep_cursor_visible(self.cursor);
    }

    pub fn move_down(&mut self, lines_to_move: usize, line_count: usize) {
        let last_line = line_count.saturating_sub(1);
        self.cursor = self.cursor.saturating_add(lines_to_move).min(last_line);
        self.viewport.keep_cursor_visible(self.cursor);
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport.height = height;
        self.viewport.keep_cursor_visible(self.cursor);
    }
}

impl Viewport {
    pub fn top(&self) -> usize {
        self.top
    }

    fn keep_cursor_visible(&mut self, cursor: usize) {
        if cursor < self.top || self.height == 0 {
            self.top = cursor;
        } else if cursor >= self.top + self.height {
            self.top = cursor + 1 - self.height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab(viewport_height: usize) -> Tab {
        let mut tab = Tab::default();
        tab.set_viewport_height(viewport_height);
        tab
    }

    #[test]
    fn given_count_past_the_start_when_moving_up_then_cursor_stops_at_first_line() {
        let mut tab = make_tab(10);
        tab.move_down(5, 10);

        tab.move_up(usize::MAX);

        assert_eq!(tab.cursor(), 0);
    }

    #[test]
    fn given_count_past_the_end_when_moving_down_then_cursor_stops_at_last_line() {
        let mut tab = make_tab(10);

        tab.move_down(usize::MAX, 5);

        assert_eq!(tab.cursor(), 4);
    }

    #[test]
    fn given_empty_file_when_moving_down_then_cursor_stays_on_first_line() {
        let mut tab = make_tab(10);

        tab.move_down(1, 0);

        assert_eq!(tab.cursor(), 0);
        assert_eq!(tab.viewport().top(), 0);
    }

    #[test]
    fn given_cursor_on_last_visible_line_when_moving_down_then_view_scrolls_one_line() {
        let mut tab = make_tab(3);
        tab.move_down(2, 10);
        assert_eq!(tab.viewport().top(), 0);

        tab.move_down(1, 10);

        assert_eq!(tab.cursor(), 3);
        assert_eq!(tab.viewport().top(), 1);
    }

    #[test]
    fn given_cursor_moving_above_the_view_when_moving_up_then_view_starts_at_cursor() {
        let mut tab = make_tab(3);
        tab.move_down(5, 10);
        assert_eq!(tab.viewport().top(), 3);

        tab.move_up(3);

        assert_eq!(tab.cursor(), 2);
        assert_eq!(tab.viewport().top(), 2);
    }

    #[test]
    fn given_zero_height_viewport_when_moving_then_view_starts_at_cursor() {
        let mut tab = make_tab(0);

        tab.move_down(4, 10);

        assert_eq!(tab.viewport().top(), 4);
    }

    #[test]
    fn given_cursor_near_the_bottom_when_viewport_shrinks_then_cursor_stays_visible() {
        let mut tab = make_tab(10);
        tab.move_down(8, 20);

        tab.set_viewport_height(5);

        assert_eq!(tab.viewport().top(), 4);
    }
}
