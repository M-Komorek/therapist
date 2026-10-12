const WIDE_SCROLL_PERCENT: usize = 20;
const CURSOR_CONTEXT_ROWS: usize = 5;

#[derive(Default)]
pub struct LogViewState {
    cursor_row: usize,
    top_row: usize,
    left_column: usize,
    height: usize,
    width: usize,
}

impl LogViewState {
    pub(super) fn with_size_of(log_view_state: &LogViewState) -> Self {
        Self {
            height: log_view_state.height,
            width: log_view_state.width,
            ..Self::default()
        }
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn top_row(&self) -> usize {
        self.top_row
    }

    pub fn left_column(&self) -> usize {
        self.left_column
    }

    pub fn move_cursor_up(&mut self, rows_to_move: usize, row_count: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(rows_to_move);
        self.scroll_to_show_cursor_context(row_count);
    }

    pub fn move_cursor_down(&mut self, rows_to_move: usize, row_count: usize) {
        self.move_cursor_to_row(self.cursor_row.saturating_add(rows_to_move), row_count);
    }

    pub fn move_cursor_to_last_row(&mut self, row_count: usize) {
        self.jump_cursor_to_row(last_row(row_count), row_count);
    }

    pub(super) fn jump_cursor_to_row(&mut self, row: usize, row_count: usize) {
        self.cursor_row = row.min(last_row(row_count));
        if !self.shows(self.cursor_row) {
            self.centre_on(self.cursor_row, row_count);
        }
        self.scroll_to_show_cursor_context(row_count);
    }

    pub fn scroll_half_page_up(&mut self, row_count: usize) {
        self.scroll_up_with_cursor(self.half_page(), row_count);
    }

    pub fn scroll_half_page_down(&mut self, row_count: usize) {
        self.scroll_down_with_cursor(self.half_page(), row_count);
    }

    pub fn scroll_page_up(&mut self, row_count: usize) {
        self.scroll_up_with_cursor(self.full_page(), row_count);
    }

    pub fn scroll_page_down(&mut self, row_count: usize) {
        self.scroll_down_with_cursor(self.full_page(), row_count);
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

    pub(super) fn set_height(&mut self, height: usize, row_count: usize) {
        self.height = height;
        self.scroll_to_show_cursor_context(row_count);
    }

    pub(super) fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    fn move_cursor_to_row(&mut self, row: usize, row_count: usize) {
        self.cursor_row = row.min(last_row(row_count));
        self.scroll_to_show_cursor_context(row_count);
    }

    fn scroll_up_with_cursor(&mut self, rows_to_scroll: usize, row_count: usize) {
        self.scroll_top_row_up(rows_to_scroll);
        self.cursor_row = self.cursor_row.saturating_sub(rows_to_scroll);
        self.move_cursor_to_show_its_context(row_count);
    }

    fn scroll_down_with_cursor(&mut self, rows_to_scroll: usize, row_count: usize) {
        self.scroll_top_row_down(rows_to_scroll, row_count);
        self.cursor_row = self
            .cursor_row
            .saturating_add(rows_to_scroll)
            .min(last_row(row_count));
        self.move_cursor_to_show_its_context(row_count);
    }

    fn scroll_top_row_up(&mut self, rows_to_scroll: usize) {
        self.top_row = self.top_row.saturating_sub(rows_to_scroll);
    }

    fn scroll_top_row_down(&mut self, rows_to_scroll: usize, row_count: usize) {
        let top_of_last_page = self.top_of_last_page(row_count);
        if self.top_row < top_of_last_page {
            self.top_row = self
                .top_row
                .saturating_add(rows_to_scroll)
                .min(top_of_last_page);
        }
    }

    fn shows(&self, row: usize) -> bool {
        (self.top_row..self.top_row + self.height).contains(&row)
    }

    fn centre_on(&mut self, row: usize, row_count: usize) {
        let centred_top_row = row.saturating_sub(self.height / 2);
        self.top_row = centred_top_row.min(self.top_of_last_page(row_count));
    }

    fn top_of_last_page(&self, row_count: usize) -> usize {
        row_count.saturating_sub(self.height)
    }

    fn scroll_to_show_cursor_context(&mut self, row_count: usize) {
        let context_rows = self.cursor_context_rows();
        let top_row_for_context_below = (self.cursor_row + context_rows + 1)
            .saturating_sub(self.height)
            .min(self.top_of_last_page(row_count));
        let top_row_for_context_above = self.cursor_row.saturating_sub(context_rows);

        self.top_row = self
            .top_row
            .max(top_row_for_context_below)
            .min(top_row_for_context_above);
    }

    fn move_cursor_to_show_its_context(&mut self, row_count: usize) {
        let context_rows = self.cursor_context_rows();
        let first_cursor_row = if self.top_row == 0 {
            0
        } else {
            self.top_row + context_rows
        };
        let last_cursor_row = if self.top_row >= self.top_of_last_page(row_count) {
            last_row(row_count)
        } else {
            (self.top_row + self.height).saturating_sub(context_rows + 1)
        };

        self.cursor_row = self.cursor_row.max(first_cursor_row).min(last_cursor_row);
    }

    fn cursor_context_rows(&self) -> usize {
        CURSOR_CONTEXT_ROWS.min(self.height.saturating_sub(1) / 2)
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

fn last_row(row_count: usize) -> usize {
    row_count.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log_view_state(height: usize) -> LogViewState {
        let mut log_view_state = LogViewState::default();
        log_view_state.set_height(height, 0);
        log_view_state
    }

    #[test]
    fn given_moved_and_scrolled_state_when_a_new_one_takes_its_size_then_only_the_size_is_copied() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.set_width(80);
        log_view_state.move_cursor_down(50, 100);
        log_view_state.scroll_right(5);

        let mut sized_like_it = LogViewState::with_size_of(&log_view_state);

        assert_eq!(sized_like_it.cursor_row(), 0);
        assert_eq!(sized_like_it.left_column(), 0);
        sized_like_it.scroll_page_down(100);
        assert_eq!(sized_like_it.top_row(), 10);
        sized_like_it.scroll_right_wide();
        assert_eq!(sized_like_it.left_column(), 16);
    }

    #[test]
    fn given_count_past_the_start_when_moving_up_then_cursor_stops_at_first_row() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_down(5, 10);

        log_view_state.move_cursor_up(usize::MAX, 10);

        assert_eq!(log_view_state.cursor_row(), 0);
        assert_eq!(log_view_state.top_row(), 0);
    }

    #[test]
    fn given_count_past_the_end_when_moving_down_then_cursor_stops_at_last_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_cursor_down(usize::MAX, 5);

        assert_eq!(log_view_state.cursor_row(), 4);
    }

    #[test]
    fn given_no_rows_when_moving_down_then_cursor_stays_on_first_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_cursor_down(1, 0);

        assert_eq!(log_view_state.cursor_row(), 0);
        assert_eq!(log_view_state.top_row(), 0);
    }

    #[test]
    fn given_cursor_with_five_rows_below_it_when_moving_down_then_view_scrolls_one_row() {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_down(14, 100);
        assert_eq!(log_view_state.top_row(), 0);

        log_view_state.move_cursor_down(1, 100);

        assert_eq!(log_view_state.cursor_row(), 15);
        assert_eq!(log_view_state.top_row(), 1);
    }

    #[test]
    fn given_cursor_with_five_rows_above_it_when_moving_up_then_view_scrolls_one_row() {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_down(30, 100);
        log_view_state.move_cursor_up(9, 100);
        assert_eq!(log_view_state.top_row(), 16);

        log_view_state.move_cursor_up(1, 100);

        assert_eq!(log_view_state.cursor_row(), 20);
        assert_eq!(log_view_state.top_row(), 15);
    }

    #[test]
    fn given_view_at_the_end_when_moving_down_then_cursor_reaches_the_last_row() {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_down(98, 100);
        assert_eq!(log_view_state.top_row(), 80);

        log_view_state.move_cursor_down(1, 100);

        assert_eq!(log_view_state.cursor_row(), 99);
        assert_eq!(log_view_state.top_row(), 80);
    }

    #[test]
    fn given_five_row_viewport_when_moving_down_then_two_rows_stay_below_the_cursor() {
        let mut log_view_state = make_log_view_state(5);
        log_view_state.move_cursor_down(2, 100);
        assert_eq!(log_view_state.top_row(), 0);

        log_view_state.move_cursor_down(1, 100);

        assert_eq!(log_view_state.top_row(), 1);
    }

    #[test]
    fn given_zero_height_viewport_when_moving_then_view_starts_at_cursor() {
        let mut log_view_state = make_log_view_state(0);

        log_view_state.move_cursor_down(4, 10);

        assert_eq!(log_view_state.top_row(), 4);
    }

    #[test]
    fn given_cursor_in_the_lower_half_when_viewport_shrinks_then_view_scrolls_to_keep_its_context()
    {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_down(10, 100);

        log_view_state.set_height(10, 100);

        assert_eq!(log_view_state.top_row(), 5);
    }

    #[test]
    fn given_cursor_on_the_last_row_when_viewport_shrinks_then_view_stops_at_the_last_page() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_to_last_row(20);

        log_view_state.set_height(5, 20);

        assert_eq!(log_view_state.top_row(), 15);
    }

    #[test]
    fn given_row_below_the_view_when_jumping_to_it_then_it_is_centred() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.jump_cursor_to_row(49, 100);

        assert_eq!(log_view_state.cursor_row(), 49);
        assert_eq!(log_view_state.top_row(), 44);
    }

    #[test]
    fn given_row_above_the_view_when_jumping_to_it_then_it_is_centred() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_to_last_row(100);

        log_view_state.jump_cursor_to_row(30, 100);

        assert_eq!(log_view_state.cursor_row(), 30);
        assert_eq!(log_view_state.top_row(), 25);
    }

    #[test]
    fn given_row_on_the_screen_with_its_context_when_jumping_to_it_then_the_view_stays() {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.jump_cursor_to_row(49, 100);
        assert_eq!(log_view_state.top_row(), 39);

        log_view_state.jump_cursor_to_row(45, 100);

        assert_eq!(log_view_state.cursor_row(), 45);
        assert_eq!(log_view_state.top_row(), 39);
    }

    #[test]
    fn given_row_in_the_bottom_context_rows_when_jumping_to_it_then_view_scrolls_the_least() {
        let mut log_view_state = make_log_view_state(20);

        log_view_state.jump_cursor_to_row(17, 100);

        assert_eq!(log_view_state.cursor_row(), 17);
        assert_eq!(log_view_state.top_row(), 3);
    }

    #[test]
    fn given_view_at_the_end_when_jumping_to_the_first_row_then_the_view_starts_at_it() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_to_last_row(100);

        log_view_state.jump_cursor_to_row(0, 100);

        assert_eq!(log_view_state.cursor_row(), 0);
        assert_eq!(log_view_state.top_row(), 0);
    }

    #[test]
    fn given_row_past_the_end_when_jumping_to_it_then_cursor_stops_at_the_last_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.jump_cursor_to_row(500, 100);

        assert_eq!(log_view_state.cursor_row(), 99);
        assert_eq!(log_view_state.top_row(), 90);
    }

    #[test]
    fn given_zero_height_viewport_when_jumping_then_view_starts_at_cursor() {
        let mut log_view_state = make_log_view_state(0);

        log_view_state.jump_cursor_to_row(40, 100);

        assert_eq!(log_view_state.top_row(), 40);
    }

    #[test]
    fn given_many_rows_when_going_to_the_last_row_then_it_is_the_bottom_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_cursor_to_last_row(100);

        assert_eq!(log_view_state.cursor_row(), 99);
        assert_eq!(log_view_state.top_row(), 90);
    }

    #[test]
    fn given_no_rows_when_going_to_the_last_row_then_cursor_stays_on_the_first_row() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.move_cursor_to_last_row(0);

        assert_eq!(log_view_state.cursor_row(), 0);
        assert_eq!(log_view_state.top_row(), 0);
    }

    #[test]
    fn given_cursor_in_the_middle_when_half_page_down_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_down(7, 100);

        log_view_state.scroll_half_page_down(100);

        assert_eq!(log_view_state.top_row(), 10);
        assert_eq!(log_view_state.cursor_row(), 17);
    }

    #[test]
    fn given_view_at_the_top_when_half_page_down_then_cursor_is_pushed_below_the_context_rows() {
        let mut log_view_state = make_log_view_state(20);

        log_view_state.scroll_half_page_down(100);

        assert_eq!(log_view_state.top_row(), 10);
        assert_eq!(log_view_state.cursor_row(), 15);
    }

    #[test]
    fn given_view_near_the_end_when_half_page_down_then_view_stops_at_last_page_and_cursor_moves_on()
     {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.jump_cursor_to_row(35, 50);
        assert_eq!(log_view_state.top_row(), 25);

        log_view_state.scroll_half_page_down(50);

        assert_eq!(log_view_state.top_row(), 30);
        assert_eq!(log_view_state.cursor_row(), 45);
    }

    #[test]
    fn given_last_page_shown_when_half_page_down_then_cursor_stops_at_last_row() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_to_last_row(20);
        log_view_state.move_cursor_up(3, 20);

        log_view_state.scroll_half_page_down(20);

        assert_eq!(log_view_state.top_row(), 10);
        assert_eq!(log_view_state.cursor_row(), 19);
    }

    #[test]
    fn given_view_past_the_last_page_after_resize_when_half_page_down_then_view_does_not_move_back()
    {
        let mut log_view_state = make_log_view_state(5);
        log_view_state.move_cursor_to_last_row(20);
        log_view_state.set_height(10, 20);
        assert_eq!(log_view_state.top_row(), 15);

        log_view_state.scroll_half_page_down(20);

        assert_eq!(log_view_state.top_row(), 15);
    }

    #[test]
    fn given_scrolled_view_when_half_page_up_then_view_scrolls_and_cursor_keeps_its_row() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.jump_cursor_to_row(32, 100);
        assert_eq!(log_view_state.top_row(), 27);

        log_view_state.scroll_half_page_up(100);

        assert_eq!(log_view_state.top_row(), 22);
        assert_eq!(log_view_state.cursor_row(), 27);
    }

    #[test]
    fn given_cursor_on_the_last_row_when_half_page_up_then_cursor_is_pushed_above_the_context_rows()
    {
        let mut log_view_state = make_log_view_state(20);
        log_view_state.move_cursor_to_last_row(100);

        log_view_state.scroll_half_page_up(100);

        assert_eq!(log_view_state.top_row(), 70);
        assert_eq!(log_view_state.cursor_row(), 84);
    }

    #[test]
    fn given_view_at_the_top_when_half_page_up_then_cursor_still_moves_up() {
        let mut log_view_state = make_log_view_state(10);
        log_view_state.move_cursor_down(7, 100);

        log_view_state.scroll_half_page_up(100);

        assert_eq!(log_view_state.top_row(), 0);
        assert_eq!(log_view_state.cursor_row(), 2);
    }

    #[test]
    fn given_view_at_the_top_when_paging_down_and_up_then_view_moves_a_full_page_each_time() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.scroll_page_down(100);
        assert_eq!(log_view_state.top_row(), 10);
        assert_eq!(log_view_state.cursor_row(), 14);

        log_view_state.scroll_page_up(100);
        assert_eq!(log_view_state.top_row(), 0);
        assert_eq!(log_view_state.cursor_row(), 4);
    }

    #[test]
    fn given_one_row_viewport_when_half_page_down_then_view_moves_one_row() {
        let mut log_view_state = make_log_view_state(1);

        log_view_state.scroll_half_page_down(100);

        assert_eq!(log_view_state.top_row(), 1);
        assert_eq!(log_view_state.cursor_row(), 1);
    }

    #[test]
    fn given_no_rows_when_paging_down_then_nothing_moves() {
        let mut log_view_state = make_log_view_state(10);

        log_view_state.scroll_half_page_down(0);
        log_view_state.scroll_page_down(0);

        assert_eq!(log_view_state.top_row(), 0);
        assert_eq!(log_view_state.cursor_row(), 0);
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

        log_view_state.move_cursor_down(3, 100);

        assert_eq!(log_view_state.left_column(), 7);
    }
}
