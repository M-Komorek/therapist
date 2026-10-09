const WIDE_SCROLL_PERCENT: usize = 20;

#[derive(Default)]
pub struct LogViewState {
    cursor: usize,
    top_line: usize,
    left_column: usize,
    height: usize,
    width: usize,
}

impl LogViewState {
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn top_line(&self) -> usize {
        self.top_line
    }

    pub fn left_column(&self) -> usize {
        self.left_column
    }

    pub fn move_up(&mut self, lines_to_move: usize) {
        self.cursor = self.cursor.saturating_sub(lines_to_move);
        self.keep_cursor_visible();
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
        self.scroll_up_with_cursor(self.half_page());
    }

    pub fn half_page_down(&mut self, line_count: usize) {
        self.scroll_down_with_cursor(self.half_page(), line_count);
    }

    pub fn page_up(&mut self) {
        self.scroll_up_with_cursor(self.full_page());
    }

    pub fn page_down(&mut self, line_count: usize) {
        self.scroll_down_with_cursor(self.full_page(), line_count);
    }

    pub fn scroll_left(&mut self, columns_to_scroll: usize) {
        self.left_column = self.left_column.saturating_sub(columns_to_scroll);
    }

    pub fn scroll_right(&mut self, columns_to_scroll: usize) {
        self.left_column = self.left_column.saturating_add(columns_to_scroll);
    }

    pub fn scroll_left_wide(&mut self) {
        self.scroll_left(self.wide_scroll());
    }

    pub fn scroll_right_wide(&mut self) {
        self.scroll_right(self.wide_scroll());
    }

    pub fn scroll_to_first_column(&mut self) {
        self.left_column = 0;
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.keep_cursor_visible();
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    fn go_to_line(&mut self, line: usize, line_count: usize) {
        self.cursor = line.min(last_line(line_count));
        self.keep_cursor_visible();
    }

    fn scroll_up_with_cursor(&mut self, lines_to_scroll: usize) {
        self.scroll_top_line_up(lines_to_scroll);
        self.move_up(lines_to_scroll);
    }

    fn scroll_down_with_cursor(&mut self, lines_to_scroll: usize, line_count: usize) {
        self.scroll_top_line_down(lines_to_scroll, line_count);
        self.move_down(lines_to_scroll, line_count);
    }

    fn scroll_top_line_up(&mut self, lines_to_scroll: usize) {
        self.top_line = self.top_line.saturating_sub(lines_to_scroll);
    }

    fn scroll_top_line_down(&mut self, lines_to_scroll: usize, line_count: usize) {
        let top_of_last_page = line_count.saturating_sub(self.height);
        if self.top_line < top_of_last_page {
            self.top_line = self
                .top_line
                .saturating_add(lines_to_scroll)
                .min(top_of_last_page);
        }
    }

    fn keep_cursor_visible(&mut self) {
        if self.cursor < self.top_line || self.height == 0 {
            self.top_line = self.cursor;
        } else if self.cursor >= self.top_line + self.height {
            self.top_line = self.cursor + 1 - self.height;
        }
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
}

fn last_line(line_count: usize) -> usize {
    line_count.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log_view_state(height: usize) -> LogViewState {
        let mut log_view_state = LogViewState::default();
        log_view_state.set_height(height);
        log_view_state
    }

    #[test]
    fn given_count_past_the_start_when_moving_up_then_cursor_stops_at_first_line() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(5, 10);

        log_view_state.move_up(usize::MAX);

        assert_eq!(log_view_state.cursor(), 0);
    }

    #[test]
    fn given_count_past_the_end_when_moving_down_then_cursor_stops_at_last_line() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_down(usize::MAX, 5);

        assert_eq!(log_view_state.cursor(), 4);
    }

    #[test]
    fn given_empty_file_when_moving_down_then_cursor_stays_on_first_line() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_down(1, 0);

        assert_eq!(log_view_state.cursor(), 0);
        assert_eq!(log_view_state.top_line(), 0);
    }

    #[test]
    fn given_cursor_on_last_visible_line_when_moving_down_then_view_scrolls_one_line() {
        let mut log_view_state = make_log_view_state(3);
        log_view_state.move_down(2, 10);
        assert_eq!(log_view_state.top_line(), 0);

        log_view_state.move_down(1, 10);

        assert_eq!(log_view_state.cursor(), 3);
        assert_eq!(log_view_state.top_line(), 1);
    }

    #[test]
    fn given_cursor_moving_above_the_view_when_moving_up_then_view_starts_at_cursor() {
        let mut log_view_state = make_log_view_state(3);
        log_view_state.move_down(5, 10);
        assert_eq!(log_view_state.top_line(), 3);

        log_view_state.move_up(3);

        assert_eq!(log_view_state.cursor(), 2);
        assert_eq!(log_view_state.top_line(), 2);
    }

    #[test]
    fn given_zero_height_viewport_when_moving_then_view_starts_at_cursor() {
        let mut log_view_state = make_log_view_state(0);

        log_view_state.move_down(4, 10);

        assert_eq!(log_view_state.top_line(), 4);
    }

    #[test]
    fn given_cursor_near_the_bottom_when_viewport_shrinks_then_cursor_stays_visible() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(8, 20);

        log_view_state.set_height(5);

        assert_eq!(log_view_state.top_line(), 4);
    }

    #[test]
    fn given_line_below_the_view_when_going_to_its_number_then_it_becomes_the_bottom_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.go_to_line_number(50, 100);

        assert_eq!(log_view_state.cursor(), 49);
        assert_eq!(log_view_state.top_line(), 40);
    }

    #[test]
    fn given_line_number_past_the_end_when_going_to_it_then_cursor_stops_at_last_line() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.go_to_line_number(500, 100);

        assert_eq!(log_view_state.cursor(), 99);
    }

    #[test]
    fn given_line_number_zero_when_going_to_it_then_cursor_goes_to_first_line() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(5, 100);

        log_view_state.go_to_line_number(0, 100);

        assert_eq!(log_view_state.cursor(), 0);
    }

    #[test]
    fn given_long_file_when_going_to_last_line_then_it_is_the_bottom_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.go_to_last_line(100);

        assert_eq!(log_view_state.cursor(), 99);
        assert_eq!(log_view_state.top_line(), 90);
    }

    #[test]
    fn given_empty_file_when_going_to_last_line_then_cursor_stays_on_first_line() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.go_to_last_line(0);

        assert_eq!(log_view_state.cursor(), 0);
        assert_eq!(log_view_state.top_line(), 0);
    }

    #[test]
    fn given_cursor_on_third_row_when_half_page_down_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(2, 100);

        log_view_state.half_page_down(100);

        assert_eq!(log_view_state.top_line(), 5);
        assert_eq!(log_view_state.cursor(), 7);
    }

    #[test]
    fn given_view_near_the_end_when_half_page_down_then_view_stops_at_last_page_and_cursor_moves_on()
     {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(17, 20);
        log_view_state.move_up(9);
        assert_eq!(log_view_state.top_line(), 8);

        log_view_state.half_page_down(20);

        assert_eq!(log_view_state.top_line(), 10);
        assert_eq!(log_view_state.cursor(), 13);
    }

    #[test]
    fn given_last_page_shown_when_half_page_down_then_cursor_stops_at_last_line() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.go_to_last_line(20);
        log_view_state.move_up(3);

        log_view_state.half_page_down(20);

        assert_eq!(log_view_state.top_line(), 10);
        assert_eq!(log_view_state.cursor(), 19);
    }

    #[test]
    fn given_view_past_the_last_page_after_resize_when_half_page_down_then_view_does_not_move_back()
    {
        let mut log_view_state = make_log_view_state(5);
        log_view_state.go_to_last_line(20);
        log_view_state.set_height(10);
        assert_eq!(log_view_state.top_line(), 15);

        log_view_state.half_page_down(20);

        assert_eq!(log_view_state.top_line(), 15);
    }

    #[test]
    fn given_scrolled_view_when_half_page_up_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.go_to_line_number(33, 100);
        assert_eq!(log_view_state.top_line(), 23);

        log_view_state.half_page_up();

        assert_eq!(log_view_state.top_line(), 18);
        assert_eq!(log_view_state.cursor(), 27);
    }

    #[test]
    fn given_view_at_the_top_when_half_page_up_then_cursor_still_moves_up() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_down(7, 100);

        log_view_state.half_page_up();

        assert_eq!(log_view_state.top_line(), 0);
        assert_eq!(log_view_state.cursor(), 2);
    }

    #[test]
    fn given_view_at_the_top_when_paging_down_and_up_then_view_moves_a_full_page_each_time() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.page_down(100);
        assert_eq!(log_view_state.top_line(), 10);
        assert_eq!(log_view_state.cursor(), 10);

        log_view_state.page_up();
        assert_eq!(log_view_state.top_line(), 0);
        assert_eq!(log_view_state.cursor(), 0);
    }

    #[test]
    fn given_one_row_viewport_when_half_page_down_then_view_moves_one_line() {
        let mut log_view_state = make_log_view_state(1);

        log_view_state.half_page_down(100);

        assert_eq!(log_view_state.top_line(), 1);
        assert_eq!(log_view_state.cursor(), 1);
    }

    #[test]
    fn given_empty_file_when_paging_down_then_nothing_moves() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.half_page_down(0);
        log_view_state.page_down(0);

        assert_eq!(log_view_state.top_line(), 0);
        assert_eq!(log_view_state.cursor(), 0);
    }

    #[test]
    fn given_view_at_first_column_when_scrolling_left_then_it_stays_at_column_zero() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.scroll_left(5);

        assert_eq!(log_view_state.left_column(), 0);
    }

    #[test]
    fn given_count_when_scrolling_right_then_left_then_view_moves_by_those_columns() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.scroll_right(3);
        assert_eq!(log_view_state.left_column(), 3);

        log_view_state.scroll_left(1);
        assert_eq!(log_view_state.left_column(), 2);
    }

    #[test]
    fn given_scrolled_view_when_scrolling_left_past_the_start_then_it_stops_at_column_zero() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.scroll_right(3);

        log_view_state.scroll_left(usize::MAX);

        assert_eq!(log_view_state.left_column(), 0);
    }

    #[test]
    fn given_80_column_viewport_when_scrolling_wide_then_view_moves_16_columns() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.set_width(80);

        log_view_state.scroll_right_wide();
        assert_eq!(log_view_state.left_column(), 16);

        log_view_state.scroll_left_wide();
        assert_eq!(log_view_state.left_column(), 0);
    }

    #[test]
    fn given_viewport_narrower_than_five_columns_when_scrolling_wide_then_view_moves_one_column() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.set_width(4);

        log_view_state.scroll_right_wide();

        assert_eq!(log_view_state.left_column(), 1);
    }

    #[test]
    fn given_scrolled_view_when_scrolling_to_first_column_then_it_is_at_column_zero() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.scroll_right(42);

        log_view_state.scroll_to_first_column();

        assert_eq!(log_view_state.left_column(), 0);
    }

    #[test]
    fn given_scrolled_view_when_moving_down_then_horizontal_scroll_is_kept() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.scroll_right(7);

        log_view_state.move_down(3, 100);

        assert_eq!(log_view_state.left_column(), 7);
    }
}
