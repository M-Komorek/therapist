use std::{collections::BTreeSet, ops::Bound};

#[derive(Debug, PartialEq)]
pub struct BookmarkJump {
    pub line: usize,
    pub wrapped: bool,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum BookmarkError {
    #[error("No bookmarks")]
    NoBookmarks,
    #[error("No bookmarks in this tab")]
    NoBookmarksShown,
}

#[derive(Default)]
pub struct Bookmarks {
    bookmarked_lines: BTreeSet<usize>,
}

impl Bookmarks {
    pub fn toggle(&mut self, line: usize) {
        if !self.bookmarked_lines.remove(&line) {
            self.bookmarked_lines.insert(line);
        }
    }

    pub fn contains(&self, line: usize) -> bool {
        self.bookmarked_lines.contains(&line)
    }

    pub fn next_bookmark_from(
        &self,
        line: usize,
        is_shown: impl Fn(usize) -> bool,
    ) -> Result<BookmarkJump, BookmarkError> {
        let is_shown = |bookmarked_line: &&usize| is_shown(**bookmarked_line);
        let bookmark_below = self
            .bookmarked_lines
            .range((Bound::Excluded(line), Bound::Unbounded))
            .find(is_shown);
        let wrap_target = self.bookmarked_lines.iter().find(is_shown);
        self.jump_or_wrap(bookmark_below, wrap_target)
    }

    pub fn previous_bookmark_from(
        &self,
        line: usize,
        is_shown: impl Fn(usize) -> bool,
    ) -> Result<BookmarkJump, BookmarkError> {
        let is_shown = |bookmarked_line: &&usize| is_shown(**bookmarked_line);
        let bookmark_above = self.bookmarked_lines.range(..line).rev().find(is_shown);
        let wrap_target = self.bookmarked_lines.iter().rev().find(is_shown);
        self.jump_or_wrap(bookmark_above, wrap_target)
    }

    fn jump_or_wrap(
        &self,
        nearest_bookmark: Option<&usize>,
        wrap_target: Option<&usize>,
    ) -> Result<BookmarkJump, BookmarkError> {
        let wrapped = nearest_bookmark.is_none();

        nearest_bookmark
            .or(wrap_target)
            .map(|&line| BookmarkJump { line, wrapped })
            .ok_or_else(|| self.no_bookmark_error())
    }

    fn no_bookmark_error(&self) -> BookmarkError {
        if self.bookmarked_lines.is_empty() {
            BookmarkError::NoBookmarks
        } else {
            BookmarkError::NoBookmarksShown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown_everywhere(_line: usize) -> bool {
        true
    }

    fn make_bookmarks(bookmarked_lines: &[usize]) -> Bookmarks {
        let mut bookmarks = Bookmarks::default();
        for &line in bookmarked_lines {
            bookmarks.toggle(line);
        }
        bookmarks
    }

    fn jump_to(line: usize) -> Result<BookmarkJump, BookmarkError> {
        Ok(BookmarkJump {
            line,
            wrapped: false,
        })
    }

    fn wrap_to(line: usize) -> Result<BookmarkJump, BookmarkError> {
        Ok(BookmarkJump {
            line,
            wrapped: true,
        })
    }

    #[test]
    fn given_no_bookmarks_when_a_line_is_toggled_then_it_is_bookmarked() {
        let mut bookmarks = Bookmarks::default();

        bookmarks.toggle(4);

        assert!(bookmarks.contains(4));
        assert!(!bookmarks.contains(3) && !bookmarks.contains(5));
    }

    #[test]
    fn given_bookmarked_line_when_it_is_toggled_again_then_it_is_not_bookmarked() {
        let mut bookmarks = make_bookmarks(&[4]);

        bookmarks.toggle(4);

        assert!(!bookmarks.contains(4));
    }

    #[test]
    fn given_two_bookmarks_when_one_is_toggled_off_then_the_other_stays() {
        let mut bookmarks = make_bookmarks(&[4, 9]);

        bookmarks.toggle(4);

        assert!(!bookmarks.contains(4));
        assert!(bookmarks.contains(9));
    }

    #[test]
    fn given_no_bookmarks_when_jumping_either_way_then_there_are_no_bookmarks() {
        let bookmarks = Bookmarks::default();

        assert_eq!(
            bookmarks.next_bookmark_from(0, shown_everywhere),
            Err(BookmarkError::NoBookmarks)
        );
        assert_eq!(
            bookmarks.previous_bookmark_from(0, shown_everywhere),
            Err(BookmarkError::NoBookmarks)
        );
    }

    #[test]
    fn given_cursor_between_bookmarks_when_jumping_then_it_goes_to_the_nearest_in_that_direction() {
        let bookmarks = make_bookmarks(&[3, 7, 12]);

        assert_eq!(
            bookmarks.next_bookmark_from(8, shown_everywhere),
            jump_to(12)
        );
        assert_eq!(
            bookmarks.previous_bookmark_from(8, shown_everywhere),
            jump_to(7)
        );
    }

    #[test]
    fn given_cursor_on_a_bookmark_when_jumping_then_it_goes_to_the_next_one_in_that_direction() {
        let bookmarks = make_bookmarks(&[3, 7, 12]);

        assert_eq!(
            bookmarks.next_bookmark_from(7, shown_everywhere),
            jump_to(12)
        );
        assert_eq!(
            bookmarks.previous_bookmark_from(7, shown_everywhere),
            jump_to(3)
        );
    }

    #[test]
    fn given_cursor_on_or_below_the_last_bookmark_when_jumping_to_the_next_then_it_wraps_to_the_first()
     {
        let bookmarks = make_bookmarks(&[3, 7, 12]);

        assert_eq!(
            bookmarks.next_bookmark_from(12, shown_everywhere),
            wrap_to(3)
        );
        assert_eq!(
            bookmarks.next_bookmark_from(20, shown_everywhere),
            wrap_to(3)
        );
    }

    #[test]
    fn given_cursor_on_or_above_the_first_bookmark_when_jumping_to_the_previous_then_it_wraps_to_the_last()
     {
        let bookmarks = make_bookmarks(&[3, 7, 12]);

        assert_eq!(
            bookmarks.previous_bookmark_from(3, shown_everywhere),
            wrap_to(12)
        );
        assert_eq!(
            bookmarks.previous_bookmark_from(0, shown_everywhere),
            wrap_to(12)
        );
    }

    #[test]
    fn given_only_bookmark_under_the_cursor_when_jumping_then_it_wraps_to_itself() {
        let bookmarks = make_bookmarks(&[5]);

        assert_eq!(
            bookmarks.next_bookmark_from(5, shown_everywhere),
            wrap_to(5)
        );
        assert_eq!(
            bookmarks.previous_bookmark_from(5, shown_everywhere),
            wrap_to(5)
        );
    }

    #[test]
    fn given_cursor_on_the_largest_line_when_jumping_to_the_next_then_it_wraps_without_overflowing()
    {
        let bookmarks = make_bookmarks(&[3]);

        assert_eq!(
            bookmarks.next_bookmark_from(usize::MAX, shown_everywhere),
            wrap_to(3)
        );
    }

    #[test]
    fn given_bookmarks_on_lines_that_are_not_shown_when_jumping_then_they_are_skipped() {
        let bookmarks = make_bookmarks(&[3, 7, 12]);
        let is_shown = |line| line != 7;

        assert_eq!(bookmarks.next_bookmark_from(3, is_shown), jump_to(12));
        assert_eq!(bookmarks.previous_bookmark_from(12, is_shown), jump_to(3));
    }

    #[test]
    fn given_first_bookmarks_not_shown_when_wrapping_then_the_first_shown_one_is_the_target() {
        let bookmarks = make_bookmarks(&[3, 7, 12]);
        let is_shown = |line| line != 3;

        assert_eq!(bookmarks.next_bookmark_from(12, is_shown), wrap_to(7));
        assert_eq!(bookmarks.previous_bookmark_from(7, is_shown), wrap_to(12));
    }

    #[test]
    fn given_bookmarks_but_none_shown_when_jumping_then_none_is_shown_is_the_error() {
        let bookmarks = make_bookmarks(&[3, 7]);

        assert_eq!(
            bookmarks.next_bookmark_from(0, |_line| false),
            Err(BookmarkError::NoBookmarksShown)
        );
    }

    #[test]
    fn given_bookmark_errors_when_displayed_then_they_say_what_happened() {
        assert_eq!(BookmarkError::NoBookmarks.to_string(), "No bookmarks");
        assert_eq!(
            BookmarkError::NoBookmarksShown.to_string(),
            "No bookmarks in this tab"
        );
    }
}
