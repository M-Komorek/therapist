use std::ops::Range;

use ratatui::{Frame, layout::Rect, style::Stylize, text::Line};

use super::visible_row::visible_spans;
use crate::domain::{bookmarks::Bookmarks, log_file::LogFile, tabs::TabState};

const BOOKMARK_MARKER: char = '●';

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    tab_state: &TabState,
    highlighted_phrase: Option<&str>,
    bookmarks: &Bookmarks,
) {
    let log_view_state = tab_state.log_view_state();
    let top_row = log_view_state.top_row();
    let tab_rows = match tab_state.read_rows(top_row..top_row + usize::from(area.height), log_file)
    {
        Ok(tab_rows) => tab_rows,
        Err(error) => {
            tracing::error!(%error, "reading visible lines failed");
            frame.render_widget(Line::raw(error.to_string()), area);
            return;
        }
    };
    let line_number_width = line_number_width(log_file.line_count());
    let left_column = log_view_state.left_column();
    let visible_columns = left_column..left_column.saturating_add(usize::from(area.width));
    for ((row_area, tab_row), row) in area.rows().zip(&tab_rows).zip(top_row..) {
        let is_bookmarked = bookmarks.contains(tab_row.line);
        let mut display_line = Line::from(gutter(tab_row.line, line_number_width, is_bookmarked));
        let highlight_ranges = highlight_ranges(&tab_row.text, highlighted_phrase);
        display_line.extend(visible_spans(
            &tab_row.text,
            visible_columns.clone(),
            &highlight_ranges,
        ));
        if row == log_view_state.cursor_row() {
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
    use crate::{
        domain::tabs::Tabs,
        ui::tests::{render_to_buffer, rendered_rows},
    };

    fn make_tabs(width: u16, height: u16) -> Tabs {
        let mut tabs = Tabs::default();
        tabs.set_log_view_size(usize::from(height), usize::from(width));
        tabs
    }

    fn render_log_view(log_file: &LogFile, tabs: &Tabs, width: u16, height: u16) -> Buffer {
        render_with(log_file, tabs, None, &Bookmarks::default(), width, height)
    }

    fn render_with(
        log_file: &LogFile,
        tabs: &Tabs,
        highlighted_phrase: Option<&str>,
        bookmarks: &Bookmarks,
        width: u16,
        height: u16,
    ) -> Buffer {
        render_to_buffer(width, height, |frame| {
            render(
                frame,
                frame.area(),
                log_file,
                tabs.active_tab(),
                highlighted_phrase,
                bookmarks,
            );
        })
    }

    fn render_text(text: &str, cursor_row: usize, width: u16, height: u16) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut tabs = make_tabs(width, height);
        tabs.active_tab_mut()
            .log_view_state_mut()
            .move_cursor_down(cursor_row, log_file.line_count());
        render_log_view(&log_file, &tabs, width, height)
    }

    fn render_scrolled(text: &str, columns_to_scroll: usize, width: u16, height: u16) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut tabs = make_tabs(width, height);
        tabs.active_tab_mut()
            .log_view_state_mut()
            .scroll_right(columns_to_scroll);
        render_log_view(&log_file, &tabs, width, height)
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
        let tabs = make_tabs(20, 3);

        let buffer = render_with(
            &log_file,
            &tabs,
            Some("ERROR"),
            &Bookmarks::default(),
            20,
            3,
        );

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

        let buffer = render_log_view(&log_file, &make_tabs(20, 1), 20, 1);

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

        let buffer = render_with(&log_file, &make_tabs(20, 2), None, &bookmarks, 20, 2);

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
