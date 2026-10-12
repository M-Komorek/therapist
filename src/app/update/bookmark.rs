use super::{
    Effect,
    app::{show_error, show_info},
    log_view::{cursor_line, jump_cursor_to_line},
};
use crate::{
    action::BookmarkAction,
    app::Model,
    domain::bookmarks::{BookmarkError, BookmarkJump},
};

const WRAPPED_TO_TOP_MESSAGE: &str = "bookmarks hit BOTTOM, continuing at TOP";
const WRAPPED_TO_BOTTOM_MESSAGE: &str = "bookmarks hit TOP, continuing at BOTTOM";

pub(super) fn dispatch_bookmark(model: &mut Model, bookmark_action: BookmarkAction) -> Effect {
    match bookmark_action {
        BookmarkAction::Toggle => toggle_bookmark(model),
        BookmarkAction::JumpToNext => jump_to_next_bookmark(model),
        BookmarkAction::JumpToPrevious => jump_to_previous_bookmark(model),
    }
    Effect::Redraw
}

fn toggle_bookmark(model: &mut Model) {
    let Some(cursor_line) = cursor_line(model) else {
        return;
    };

    model.bookmarks.toggle(cursor_line);
}

fn jump_to_next_bookmark(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let active_tab = model.tabs.active_tab();
    let bookmark_jump = model
        .bookmarks
        .next_bookmark_from(cursor_line, |line| active_tab.contains_line(line));
    apply_bookmark_jump(model, bookmark_jump, WRAPPED_TO_TOP_MESSAGE);
}

fn jump_to_previous_bookmark(model: &mut Model) {
    let cursor_line = cursor_line(model).unwrap_or_default();
    let active_tab = model.tabs.active_tab();
    let bookmark_jump = model
        .bookmarks
        .previous_bookmark_from(cursor_line, |line| active_tab.contains_line(line));
    apply_bookmark_jump(model, bookmark_jump, WRAPPED_TO_BOTTOM_MESSAGE);
}

fn apply_bookmark_jump(
    model: &mut Model,
    bookmark_jump: Result<BookmarkJump, BookmarkError>,
    wrap_message: &str,
) {
    match bookmark_jump {
        Ok(bookmark_jump) => jump_cursor_to_bookmark(model, bookmark_jump, wrap_message),
        Err(bookmark_error) => show_error(model, bookmark_error.to_string()),
    }
}

fn jump_cursor_to_bookmark(model: &mut Model, bookmark_jump: BookmarkJump, wrap_message: &str) {
    jump_cursor_to_line(model, bookmark_jump.line);
    if bookmark_jump.wrapped {
        show_info(model, wrap_message.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::app::{
        StatusMessage,
        update::test_helpers::{SEARCHED_TEXT, make_model, make_model_with_text},
    };

    #[test]
    fn given_cursor_on_the_second_line_when_bb_is_pressed_then_that_line_is_bookmarked() {
        let mut model = make_model();
        model.type_keys("j");

        let effect = model.type_keys("bb");

        assert_eq!(effect, Effect::Redraw);
        assert!(model.bookmarks.contains(1));
        assert!(!model.bookmarks.contains(0));
    }

    #[test]
    fn given_bookmarked_cursor_line_when_bb_is_pressed_again_then_the_bookmark_is_removed() {
        let mut model = make_model();
        model.type_keys("bb");

        model.type_keys("bb");

        assert!(!model.bookmarks.contains(0));
    }

    #[test]
    fn given_empty_file_when_bb_is_pressed_then_nothing_is_bookmarked() {
        let mut model = make_model_with_text("");

        model.type_keys("bb");

        assert!(!model.bookmarks.contains(0));
    }

    #[test]
    fn given_bookmarks_above_and_below_when_bn_then_bp_are_pressed_then_cursor_goes_down_then_up() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("2Gbb5Gbb3G");

        model.type_keys("bn");
        assert_eq!(model.cursor_line(), Some(4));

        model.type_keys("bp");
        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_cursor_on_the_last_bookmark_when_bn_is_pressed_then_it_wraps_to_the_first_with_a_message()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("2Gbb5Gbb");

        model.type_keys("bn");

        assert_eq!(model.cursor_line(), Some(1));
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info(WRAPPED_TO_TOP_MESSAGE.to_owned()))
        );
    }

    #[test]
    fn given_cursor_on_the_first_bookmark_when_bp_is_pressed_then_it_wraps_to_the_last_with_a_message()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("2Gbb5Gbb2G");

        model.type_keys("bp");

        assert_eq!(model.cursor_line(), Some(4));
        assert_eq!(
            model.status_message,
            Some(StatusMessage::Info(WRAPPED_TO_BOTTOM_MESSAGE.to_owned()))
        );
    }

    #[test]
    fn given_bookmark_on_a_line_not_in_the_grep_tab_when_bn_is_pressed_there_then_it_is_skipped() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("3Gbb5Gbb");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();

        model.type_keys("bn");

        assert_eq!(model.cursor_line(), Some(4));
        assert_eq!(model.status_message, None);
    }

    #[test]
    fn given_bookmarks_only_on_lines_not_in_the_grep_tab_when_bn_is_pressed_there_then_an_error_says_so()
     {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        model.type_keys("3Gbb");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();

        model.type_keys("bn");

        assert_eq!(
            model.status_message,
            Some(StatusMessage::Error(
                BookmarkError::NoBookmarksShown.to_string()
            ))
        );
    }

    #[test]
    fn given_no_bookmarks_when_bn_or_bp_is_pressed_then_an_error_says_so() {
        let mut model = make_model_with_text(SEARCHED_TEXT);
        let no_bookmarks = Some(StatusMessage::Error(BookmarkError::NoBookmarks.to_string()));

        model.type_keys("bn");
        assert_eq!(model.status_message, no_bookmarks);

        model.type_keys("bp");
        assert_eq!(model.status_message, no_bookmarks);
        assert_eq!(model.cursor_line(), Some(0));
    }
}
