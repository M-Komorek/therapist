use std::ops::Range;

use ratatui::{Frame, layout::Rect, style::Stylize, text::Line};

use super::visible_row::visible_spans;
use crate::domain::{bookmarks::Bookmarks, log_file::LogFile, log_view_state::LogViewState};

const BOOKMARK_MARKER: char = '●';

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    log_view_state: &LogViewState,
    highlighted_phrase: Option<&str>,
    bookmarks: &Bookmarks,
) {
    let top_line = log_view_state.top_line();
    let line_texts = match log_file.read_lines(top_line..top_line + usize::from(area.height)) {
        Ok(line_texts) => line_texts,
        Err(error) => {
            tracing::error!(%error, "reading visible lines failed");
            frame.render_widget(Line::raw(error.to_string()), area);
            return;
        }
    };
    let line_number_width = line_number_width(log_file.line_count());
    let left_column = log_view_state.left_column();
    let visible_columns = left_column..left_column.saturating_add(usize::from(area.width));
    for ((row_area, line_text), line) in area.rows().zip(&line_texts).zip(top_line..) {
        let is_bookmarked = bookmarks.contains(line);
        let mut display_line = Line::from(gutter(line, line_number_width, is_bookmarked));
        let highlight_ranges = highlight_ranges(line_text, highlighted_phrase);
        display_line.extend(visible_spans(
            line_text,
            visible_columns.clone(),
            &highlight_ranges,
        ));
        if line == log_view_state.cursor_line() {
            display_line = display_line.reversed();
        }
        frame.render_widget(display_line, row_area);
    }
}

fn highlight_ranges(line_text: &str, highlighted_phrase: Option<&str>) -> Vec<Range<usize>> {
    let Some(phrase) = highlighted_phrase.filter(|phrase| !phrase.is_empty()) else {
        return Vec::new();
    };

    line_text
        .match_indices(phrase)
        .map(|(match_start, _)| match_start..match_start + phrase.len())
        .collect()
}

fn line_number_width(line_count: usize) -> usize {
    line_count.max(1).ilog10() as usize + 1
}

fn gutter(line: usize, line_number_width: usize, is_bookmarked: bool) -> String {
    let marker = if is_bookmarked { BOOKMARK_MARKER } else { ' ' };
    format!("{marker} {:>line_number_width$} │ ", line + 1)
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        style::{Color, Modifier},
    };

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_log_view(
        log_file: &LogFile,
        log_view_state: &LogViewState,
        width: u16,
        height: u16,
    ) -> Buffer {
        render_highlighted(log_file, log_view_state, None, width, height)
    }

    fn render_highlighted(
        log_file: &LogFile,
        log_view_state: &LogViewState,
        highlighted_phrase: Option<&str>,
        width: u16,
        height: u16,
    ) -> Buffer {
        render_to_buffer(width, height, |frame| {
            render(
                frame,
                frame.area(),
                log_file,
                log_view_state,
                highlighted_phrase,
                &Bookmarks::default(),
            );
        })
    }

    fn render_text(text: &str, cursor_line: usize, width: u16, height: u16) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut log_view_state = LogViewState::default();
        log_view_state.set_height(usize::from(height));
        log_view_state.move_cursor_down(cursor_line, log_file.line_count());
        render_log_view(&log_file, &log_view_state, width, height)
    }

    fn render_scrolled(text: &str, columns_to_scroll: usize, width: u16, height: u16) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut log_view_state = LogViewState::default();
        log_view_state.scroll_right(columns_to_scroll);
        render_log_view(&log_file, &log_view_state, width, height)
    }

    #[test]
    fn given_scrolled_view_when_rendering_then_numbers_are_right_aligned_in_the_gutter() {
        let text: String = (1..=12)
            .map(|line_number| format!("line {line_number}\n"))
            .collect();

        let buffer = render_text(&text, 9, 20, 3);

        assert_eq!(
            rendered_rows(&buffer),
            ["   8 │ line 8", "   9 │ line 9", "  10 │ line 10"]
        );
    }

    #[test]
    fn given_highlighted_phrase_when_rendering_then_every_occurrence_on_visible_lines_is_highlighted()
     {
        let log_file = LogFile::from_text("ERROR a ERROR\nok\nERROR\n");

        let buffer = render_highlighted(&log_file, &LogViewState::default(), Some("ERROR"), 20, 3);

        let is_highlighted = |column, row| buffer[(column, row)].bg == Color::Yellow;
        let gutter_width = 6;
        assert!(is_highlighted(gutter_width, 0) && is_highlighted(gutter_width + 12, 0));
        assert!(!is_highlighted(gutter_width + 6, 0));
        assert!(!is_highlighted(gutter_width, 1));
        assert!(is_highlighted(gutter_width + 4, 2));
    }

    #[test]
    fn given_no_highlighted_phrase_when_rendering_then_nothing_is_highlighted() {
        let log_file = LogFile::from_text("ERROR\n");

        let buffer = render_highlighted(&log_file, &LogViewState::default(), None, 20, 1);

        assert_ne!(buffer[(6, 0)].bg, Color::Yellow);
    }

    #[test]
    fn given_empty_phrase_when_finding_highlights_then_there_are_none() {
        assert!(highlight_ranges("abc", Some("")).is_empty());
    }

    #[test]
    fn given_bookmarked_line_when_rendering_then_its_gutter_starts_with_the_marker() {
        let log_file = LogFile::from_text("a\nb\n");
        let mut bookmarks = Bookmarks::default();
        bookmarks.toggle(1);

        let buffer = render_to_buffer(20, 2, |frame| {
            let log_view_state = LogViewState::default();
            render(
                frame,
                frame.area(),
                &log_file,
                &log_view_state,
                None,
                &bookmarks,
            );
        });

        assert_eq!(rendered_rows(&buffer), ["  1 │ a", "● 2 │ b"]);
    }

    #[test]
    fn given_cursor_line_when_rendering_then_its_whole_row_is_reversed() {
        let buffer = render_text("a\nb\nc\n", 1, 10, 3);

        let is_reversed = |column, row| buffer[(column, row)].modifier.contains(Modifier::REVERSED);
        assert!(is_reversed(0, 1) && is_reversed(9, 1));
        assert!(!is_reversed(0, 0) && !is_reversed(0, 2));
    }

    #[test]
    fn given_line_wider_than_the_area_when_rendering_then_it_is_cut_at_the_edge() {
        let buffer = render_text("abcdefghij\n", 0, 8, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ ab"]);
    }

    #[test]
    fn given_tabs_when_rendering_then_they_expand_to_stops_every_eight_columns() {
        let buffer = render_text("ab\tc\td\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ ab      c       d"]);
    }

    #[test]
    fn given_wide_characters_and_emoji_before_tabs_when_rendering_then_tabs_still_end_on_stops() {
        let buffer = render_text("漢\t❤\u{fe0f}\tx\n", 0, 30, 1);

        let x_column = (0..30).find(|&column| buffer[(column, 0)].symbol() == "x");
        assert_eq!(x_column, Some(6 + 16));
    }

    #[test]
    fn given_c1_control_before_a_tab_when_rendering_then_it_is_dropped_without_moving_the_tab() {
        let buffer = render_text("a\u{85}\tx\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ a       x"]);
    }

    #[test]
    fn given_control_characters_when_rendering_then_they_show_dimmed_in_caret_notation() {
        let buffer = render_text("a\x1b[31mb\rc\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ a^[[31mb^Mc"]);
        let is_dim = |column| buffer[(column, 0)].modifier.contains(Modifier::DIM);
        assert!(is_dim(7) && is_dim(8));
        assert!(!is_dim(6) && !is_dim(9));
    }

    #[test]
    fn given_del_character_when_rendering_then_it_shows_in_caret_notation() {
        let buffer = render_text("a\x7fb\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ a^?b"]);
    }

    #[test]
    fn given_non_ascii_past_the_visible_columns_when_rendering_then_the_visible_part_is_unchanged()
    {
        let buffer = render_text("abcdefgh\t漢\x1b\n", 0, 8, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ ab"]);
    }

    #[test]
    fn given_empty_file_when_rendering_then_the_view_is_blank() {
        let buffer = render_text("", 0, 10, 2);

        assert_eq!(rendered_rows(&buffer), ["", ""]);
    }

    #[test]
    fn given_scrolled_view_when_rendering_then_every_line_is_cut_and_the_gutter_stays() {
        let buffer = render_scrolled("abcdef\nxyz\n", 2, 20, 2);

        assert_eq!(rendered_rows(&buffer), ["  1 │ cdef", "  2 │ z"]);
    }

    #[test]
    fn given_wide_character_cut_by_the_left_edge_when_rendering_then_its_visible_half_is_a_space() {
        let buffer = render_scrolled("漢字x\n", 1, 14, 1);

        let symbol_at = |column| buffer[(column, 0)].symbol().to_owned();
        assert_eq!(symbol_at(6), " ");
        assert_eq!(symbol_at(7), "字");
        assert_eq!(symbol_at(9), "x");
    }

    #[test]
    fn given_tab_left_of_the_edge_when_scrolled_then_text_after_it_keeps_its_tab_stop() {
        let buffer = render_scrolled("a\tb\n", 4, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │     b"]);
    }

    #[test]
    fn given_control_character_left_of_the_edge_when_scrolled_then_its_caret_notation_is_skipped() {
        let buffer = render_scrolled("\x1bXY\n", 2, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ XY"]);
    }

    #[test]
    fn given_view_scrolled_past_the_end_of_a_line_when_rendering_then_only_the_gutter_shows() {
        let buffer = render_scrolled("abc\n", 10, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │"]);
    }

    #[test]
    fn given_caret_notation_cut_by_the_left_edge_when_scrolled_then_its_visible_half_is_a_space() {
        let buffer = render_scrolled("\x1bXY\n", 1, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │  XY"]);
    }

    #[test]
    fn given_arabic_ligature_before_a_tab_when_rendering_then_the_tab_still_ends_on_a_stop() {
        let buffer = render_text("\u{644}\u{627}\tx\n", 0, 30, 1);

        let x_column = (0..30).find(|&column| buffer[(column, 0)].symbol() == "x");
        assert_eq!(x_column, Some(6 + 8));
    }
}
