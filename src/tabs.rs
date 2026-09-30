const WIDE_SCROLL_PERCENT: usize = 20;

#[derive(Default)]
pub struct Tab {
    cursor: usize,
    viewport: Viewport,
}

#[derive(Default)]
pub struct Viewport {
    top: usize,
    height: usize,
    left_column: usize,
    width: usize,
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
        self.go_to_line(self.cursor.saturating_add(lines_to_move), line_count);
    }

    pub fn go_to_line_number(&mut self, line_number: usize, line_count: usize) {
        self.go_to_line(line_number.saturating_sub(1), line_count);
    }

    pub fn go_to_last_line(&mut self, line_count: usize) {
        self.go_to_line(last_line(line_count), line_count);
    }

    pub fn half_page_up(&mut self) {
        self.scroll_up_with_cursor(self.viewport.half_page());
    }

    pub fn half_page_down(&mut self, line_count: usize) {
        self.scroll_down_with_cursor(self.viewport.half_page(), line_count);
    }

    pub fn page_up(&mut self) {
        self.scroll_up_with_cursor(self.viewport.full_page());
    }

    pub fn page_down(&mut self, line_count: usize) {
        self.scroll_down_with_cursor(self.viewport.full_page(), line_count);
    }

    pub fn scroll_left(&mut self, columns: usize) {
        self.viewport.left_column = self.viewport.left_column.saturating_sub(columns);
    }

    pub fn scroll_right(&mut self, columns: usize) {
        self.viewport.left_column = self.viewport.left_column.saturating_add(columns);
    }

    pub fn scroll_left_wide(&mut self) {
        self.scroll_left(self.viewport.wide_scroll());
    }

    pub fn scroll_right_wide(&mut self) {
        self.scroll_right(self.viewport.wide_scroll());
    }

    pub fn scroll_to_first_column(&mut self) {
        self.viewport.left_column = 0;
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport.height = height;
        self.viewport.keep_cursor_visible(self.cursor);
    }

    pub fn set_viewport_width(&mut self, width: usize) {
        self.viewport.width = width;
    }

    fn go_to_line(&mut self, line: usize, line_count: usize) {
        self.cursor = line.min(last_line(line_count));
        self.viewport.keep_cursor_visible(self.cursor);
    }

    fn scroll_up_with_cursor(&mut self, lines: usize) {
        self.viewport.scroll_up(lines);
        self.move_up(lines);
    }

    fn scroll_down_with_cursor(&mut self, lines: usize, line_count: usize) {
        self.viewport.scroll_down(lines, line_count);
        self.move_down(lines, line_count);
    }
}

impl Viewport {
    pub fn top(&self) -> usize {
        self.top
    }

    pub fn left_column(&self) -> usize {
        self.left_column
    }

    fn wide_scroll(&self) -> usize {
        (self.width * WIDE_SCROLL_PERCENT / 100).max(1)
    }

    fn half_page(&self) -> usize {
        (self.height / 2).max(1)
    }

    fn full_page(&self) -> usize {
        self.height.max(1)
    }

    fn scroll_up(&mut self, lines: usize) {
        self.top = self.top.saturating_sub(lines);
    }

    fn scroll_down(&mut self, lines: usize, line_count: usize) {
        let top_of_last_page = line_count.saturating_sub(self.height);
        if self.top < top_of_last_page {
            self.top = self.top.saturating_add(lines).min(top_of_last_page);
        }
    }

    fn keep_cursor_visible(&mut self, cursor: usize) {
        if cursor < self.top || self.height == 0 {
            self.top = cursor;
        } else if cursor >= self.top + self.height {
            self.top = cursor + 1 - self.height;
        }
    }
}

fn last_line(line_count: usize) -> usize {
    line_count.saturating_sub(1)
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

    #[test]
    fn given_line_below_the_view_when_going_to_its_number_then_it_becomes_the_bottom_row() {
        let mut tab = make_tab(10);

        tab.go_to_line_number(50, 100);

        assert_eq!(tab.cursor(), 49);
        assert_eq!(tab.viewport().top(), 40);
    }

    #[test]
    fn given_line_number_past_the_end_when_going_to_it_then_cursor_stops_at_last_line() {
        let mut tab = make_tab(10);

        tab.go_to_line_number(500, 100);

        assert_eq!(tab.cursor(), 99);
    }

    #[test]
    fn given_line_number_zero_when_going_to_it_then_cursor_goes_to_first_line() {
        let mut tab = make_tab(10);
        tab.move_down(5, 100);

        tab.go_to_line_number(0, 100);

        assert_eq!(tab.cursor(), 0);
    }

    #[test]
    fn given_long_file_when_going_to_last_line_then_it_is_the_bottom_row() {
        let mut tab = make_tab(10);

        tab.go_to_last_line(100);

        assert_eq!(tab.cursor(), 99);
        assert_eq!(tab.viewport().top(), 90);
    }

    #[test]
    fn given_empty_file_when_going_to_last_line_then_cursor_stays_on_first_line() {
        let mut tab = make_tab(10);

        tab.go_to_last_line(0);

        assert_eq!(tab.cursor(), 0);
        assert_eq!(tab.viewport().top(), 0);
    }

    #[test]
    fn given_cursor_on_third_row_when_half_page_down_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut tab = make_tab(10);
        tab.move_down(2, 100);

        tab.half_page_down(100);

        assert_eq!(tab.viewport().top(), 5);
        assert_eq!(tab.cursor(), 7);
    }

    #[test]
    fn given_view_near_the_end_when_half_page_down_then_view_stops_at_last_page_and_cursor_moves_on()
     {
        let mut tab = make_tab(10);
        tab.move_down(17, 20);
        tab.move_up(9);
        assert_eq!(tab.viewport().top(), 8);

        tab.half_page_down(20);

        assert_eq!(tab.viewport().top(), 10);
        assert_eq!(tab.cursor(), 13);
    }

    #[test]
    fn given_last_page_shown_when_half_page_down_then_cursor_stops_at_last_line() {
        let mut tab = make_tab(10);
        tab.go_to_last_line(20);
        tab.move_up(3);

        tab.half_page_down(20);

        assert_eq!(tab.viewport().top(), 10);
        assert_eq!(tab.cursor(), 19);
    }

    #[test]
    fn given_view_past_the_last_page_after_resize_when_half_page_down_then_view_does_not_move_back()
    {
        let mut tab = make_tab(5);
        tab.go_to_last_line(20);
        tab.set_viewport_height(10);
        assert_eq!(tab.viewport().top(), 15);

        tab.half_page_down(20);

        assert_eq!(tab.viewport().top(), 15);
    }

    #[test]
    fn given_scrolled_view_when_half_page_up_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut tab = make_tab(10);
        tab.go_to_line_number(33, 100);
        assert_eq!(tab.viewport().top(), 23);

        tab.half_page_up();

        assert_eq!(tab.viewport().top(), 18);
        assert_eq!(tab.cursor(), 27);
    }

    #[test]
    fn given_view_at_the_top_when_half_page_up_then_cursor_still_moves_up() {
        let mut tab = make_tab(10);
        tab.move_down(7, 100);

        tab.half_page_up();

        assert_eq!(tab.viewport().top(), 0);
        assert_eq!(tab.cursor(), 2);
    }

    #[test]
    fn given_view_at_the_top_when_paging_down_and_up_then_view_moves_a_full_page_each_time() {
        let mut tab = make_tab(10);

        tab.page_down(100);
        assert_eq!(tab.viewport().top(), 10);
        assert_eq!(tab.cursor(), 10);

        tab.page_up();
        assert_eq!(tab.viewport().top(), 0);
        assert_eq!(tab.cursor(), 0);
    }

    #[test]
    fn given_one_row_viewport_when_half_page_down_then_view_moves_one_line() {
        let mut tab = make_tab(1);

        tab.half_page_down(100);

        assert_eq!(tab.viewport().top(), 1);
        assert_eq!(tab.cursor(), 1);
    }

    #[test]
    fn given_empty_file_when_paging_down_then_nothing_moves() {
        let mut tab = make_tab(10);

        tab.half_page_down(0);
        tab.page_down(0);

        assert_eq!(tab.viewport().top(), 0);
        assert_eq!(tab.cursor(), 0);
    }

    #[test]
    fn given_view_at_first_column_when_scrolling_left_then_it_stays_at_column_zero() {
        let mut tab = make_tab(10);

        tab.scroll_left(5);

        assert_eq!(tab.viewport().left_column(), 0);
    }

    #[test]
    fn given_count_when_scrolling_right_then_left_then_view_moves_by_those_columns() {
        let mut tab = make_tab(10);

        tab.scroll_right(3);
        assert_eq!(tab.viewport().left_column(), 3);

        tab.scroll_left(1);
        assert_eq!(tab.viewport().left_column(), 2);
    }

    #[test]
    fn given_scrolled_view_when_scrolling_left_past_the_start_then_it_stops_at_column_zero() {
        let mut tab = make_tab(10);
        tab.scroll_right(3);

        tab.scroll_left(usize::MAX);

        assert_eq!(tab.viewport().left_column(), 0);
    }

    #[test]
    fn given_80_column_viewport_when_scrolling_wide_then_view_moves_16_columns() {
        let mut tab = make_tab(10);
        tab.set_viewport_width(80);

        tab.scroll_right_wide();
        assert_eq!(tab.viewport().left_column(), 16);

        tab.scroll_left_wide();
        assert_eq!(tab.viewport().left_column(), 0);
    }

    #[test]
    fn given_viewport_narrower_than_five_columns_when_scrolling_wide_then_view_moves_one_column() {
        let mut tab = make_tab(10);
        tab.set_viewport_width(4);

        tab.scroll_right_wide();

        assert_eq!(tab.viewport().left_column(), 1);
    }

    #[test]
    fn given_scrolled_view_when_scrolling_to_first_column_then_it_is_at_column_zero() {
        let mut tab = make_tab(10);
        tab.scroll_right(42);

        tab.scroll_to_first_column();

        assert_eq!(tab.viewport().left_column(), 0);
    }

    #[test]
    fn given_scrolled_view_when_moving_down_then_horizontal_scroll_is_kept() {
        let mut tab = make_tab(10);
        tab.scroll_right(7);

        tab.move_down(3, 100);

        assert_eq!(tab.viewport().left_column(), 7);
    }
}
