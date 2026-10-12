use ratatui::layout::{Rect, Size};

use super::Effect;
use crate::{
    action::LogViewAction,
    app::{Model, ScreenAreas},
};

pub fn resize_log_view(model: &mut Model, screen_size: Size) -> Effect {
    let log_view_area = ScreenAreas::new(Rect::from(screen_size)).log_view;
    model.tabs.set_log_view_size(
        usize::from(log_view_area.height),
        usize::from(log_view_area.width),
    );
    Effect::Redraw
}

pub(super) fn dispatch_log_view(model: &mut Model, log_view_action: LogViewAction) -> Effect {
    let line_count = model.log_file.line_count();
    let active_tab = model.tabs.active_tab_mut();
    let row_count = active_tab.row_count(line_count);
    let log_view_state = active_tab.log_view_state_mut();
    match log_view_action {
        LogViewAction::MoveCursorUp(rows_to_move) => log_view_state.move_cursor_up(rows_to_move),
        LogViewAction::MoveCursorDown(rows_to_move) => {
            log_view_state.move_cursor_down(rows_to_move, row_count)
        }
        LogViewAction::MoveCursorToLineNumber(line_number) => {
            active_tab.move_cursor_to_line_number(line_number, line_count)
        }
        LogViewAction::MoveCursorToLastLine => log_view_state.move_cursor_to_last_row(row_count),
        LogViewAction::ScrollHalfPageUp => log_view_state.scroll_half_page_up(),
        LogViewAction::ScrollHalfPageDown => log_view_state.scroll_half_page_down(row_count),
        LogViewAction::ScrollPageUp => log_view_state.scroll_page_up(),
        LogViewAction::ScrollPageDown => log_view_state.scroll_page_down(row_count),
        LogViewAction::ScrollLeft(columns_to_scroll) => {
            log_view_state.scroll_left(columns_to_scroll)
        }
        LogViewAction::ScrollRight(columns_to_scroll) => {
            log_view_state.scroll_right(columns_to_scroll)
        }
        LogViewAction::ScrollLeftWide => log_view_state.scroll_left_wide(),
        LogViewAction::ScrollRightWide => log_view_state.scroll_right_wide(),
        LogViewAction::ScrollToFirstColumn => log_view_state.scroll_to_first_column(),
    }
    Effect::Redraw
}

pub(super) fn cursor_line(model: &Model) -> Option<usize> {
    let line_count = model.log_file.line_count();
    model.tabs.active_tab().cursor_line(line_count)
}

pub(super) fn jump_cursor_to_line(model: &mut Model, line: usize) {
    let line_count = model.log_file.line_count();
    model
        .tabs
        .active_tab_mut()
        .jump_cursor_to_line(line, line_count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::update::test_helpers::{command_line_text, make_model, run_command};

    #[test]
    fn given_line_number_when_submitted_then_cursor_goes_there_and_the_command_line_closes() {
        let mut model = make_model();

        let effect = run_command(&mut model, "2");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(command_line_text(&model), None);
    }

    #[test]
    fn given_number_past_the_end_when_submitted_then_cursor_goes_to_the_last_line() {
        let mut model = make_model();

        run_command(&mut model, "99");

        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_cursor_below_first_line_when_zero_is_submitted_then_cursor_goes_to_the_first_line() {
        let mut model = make_model();
        model.type_keys("j");

        run_command(&mut model, "0");

        assert_eq!(model.cursor_line(), Some(0));
    }

    #[test]
    fn given_open_command_line_when_terminal_is_resized_then_it_stays_open_with_its_text() {
        let mut model = make_model();
        model.type_keys(":5");

        resize_log_view(&mut model, Size::new(40, 10));

        assert_eq!(command_line_text(&model), Some("5"));
    }

    #[test]
    fn given_app_when_j_is_pressed_then_cursor_moves_down_and_screen_is_redrawn() {
        let mut model = make_model();

        let effect = model.type_keys("j");

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_cursor_on_second_line_when_k_is_pressed_then_cursor_moves_up() {
        let mut model = make_model();
        model.type_keys("j");

        model.type_keys("k");

        assert_eq!(model.cursor_line(), Some(0));
    }

    #[test]
    fn given_pending_count_when_j_is_pressed_then_cursor_moves_that_many_lines() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("j");

        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_cursor_on_last_line_when_j_is_pressed_then_cursor_stays_there() {
        let mut model = make_model();
        model.type_keys("jj");

        model.type_keys("j");

        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_new_app_when_j_is_pressed_then_the_view_does_not_scroll() {
        let mut model = make_model();

        model.type_keys("j");

        assert_eq!(model.tabs.active_tab().log_view_state().top_row(), 0);
    }

    #[test]
    fn given_app_when_capital_g_is_pressed_then_cursor_goes_to_last_line() {
        let mut model = make_model();

        model.type_keys("G");

        assert_eq!(model.cursor_line(), Some(2));
    }

    #[test]
    fn given_count_when_capital_g_is_pressed_then_cursor_goes_to_that_line_number() {
        let mut model = make_model();
        model.type_keys("2");

        model.type_keys("G");

        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_app_when_ctrl_d_is_pressed_then_cursor_moves_half_a_page() {
        let mut model = make_model();

        let effect = model.press_ctrl('d');

        assert_eq!(effect, Effect::Redraw);
        assert_eq!(model.cursor_line(), Some(1));
    }

    #[test]
    fn given_count_when_l_then_h_are_pressed_then_view_scrolls_right_then_left() {
        let mut model = make_model();
        model.type_keys("3");
        model.type_keys("l");

        model.type_keys("h");

        assert_eq!(model.tabs.active_tab().log_view_state().left_column(), 2);
    }

    #[test]
    fn given_terminal_resized_to_100_columns_when_capital_l_is_pressed_then_view_scrolls_20_columns()
     {
        let mut model = make_model();
        resize_log_view(&mut model, Size::new(100, 10));

        model.type_keys("L");

        assert_eq!(model.tabs.active_tab().log_view_state().left_column(), 20);
    }

    #[test]
    fn given_app_when_terminal_is_resized_then_screen_is_redrawn() {
        let mut model = make_model();

        let effect = resize_log_view(&mut model, Size::new(30, 10));

        assert_eq!(effect, Effect::Redraw);
    }

    #[test]
    fn given_cursor_on_last_line_when_window_shrinks_then_view_scrolls_to_keep_it_visible() {
        let mut model = make_model();
        model.type_keys("jj");

        resize_log_view(&mut model, Size::new(20, 3));

        assert_eq!(model.tabs.active_tab().log_view_state().top_row(), 2);
    }
}
