use std::ops::Range;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
};

use super::{
    palette,
    visible_row::{MatchLayer, VisibleRow, visible_row},
};
use crate::domain::{
    bookmarks::Bookmarks,
    highlights::{HIGHLIGHT_COLOR_COUNT, Highlights},
    log_file::LogFile,
    tabs::TabState,
};

const BOOKMARK_MARKER: char = '●';
const BOOKMARK_COLUMN_WIDTH: usize = 2;
const GUTTER_SEPARATOR: &str = " │ ";
const LEFT_CUT_MARKER: &str = "<";
const RIGHT_CUT_MARKER: &str = ">";
const EMPTY_FILE_TEXT: &str = "empty file";
const LOG_VIEW_STYLE: Style = Style::new().fg(palette::FOREGROUND).bg(palette::BACKGROUND);
const FAINT_TEXT_STYLE: Style = Style::new().fg(palette::FOREGROUND_FAINT);
const GUTTER_STYLE: Style = FAINT_TEXT_STYLE.bg(palette::BACKGROUND);
const BOOKMARK_MARKER_STYLE: Style = GUTTER_STYLE.fg(palette::YELLOW);
const CURSOR_LINE_NUMBER_STYLE: Style = GUTTER_STYLE
    .fg(palette::ORANGE)
    .add_modifier(Modifier::BOLD);
const DARK_TEXT_STYLE: Style = Style::new().fg(palette::BACKGROUND);
const SEARCH_MATCH_STYLE: Style = DARK_TEXT_STYLE.bg(palette::YELLOW);
const HIGHLIGHT_STYLES: [Style; HIGHLIGHT_COLOR_COUNT] = [
    DARK_TEXT_STYLE.bg(palette::CORAL),
    DARK_TEXT_STYLE.bg(palette::TEAL),
    DARK_TEXT_STYLE.bg(palette::LAVENDER),
    DARK_TEXT_STYLE.bg(palette::GREEN),
    DARK_TEXT_STYLE.bg(palette::SKY),
    DARK_TEXT_STYLE.bg(palette::PINK),
    Style::new().bg(palette::CORAL_DARK),
    Style::new().bg(palette::TEAL_DARK),
    Style::new().bg(palette::LAVENDER_DARK),
    Style::new().bg(palette::GREEN_DARK),
    Style::new().bg(palette::SKY_DARK),
    Style::new().bg(palette::PINK_DARK),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    tab_state: &TabState,
    search_phrase: Option<&str>,
    highlights: &Highlights,
    bookmarks: &Bookmarks,
) {
    frame.buffer_mut().set_style(area, LOG_VIEW_STYLE);
    if log_file.file_size() == 0 {
        render_empty_file_text(frame, area);
        return;
    }
    let log_view_state = tab_state.log_view_state();
    let top_row = log_view_state.top_row();
    let tab_rows = match tab_state.read_rows(top_row..top_row + usize::from(area.height), log_file)
    {
        Ok(tab_rows) => tab_rows,
        Err(error) => {
            tracing::error!(%error, "reading visible lines failed");
            frame.render_widget(Line::raw(error.to_string()).fg(palette::RED), area);
            return;
        }
    };

    let line_number_width = line_number_width(log_file.line_count());
    let gutter_width = gutter_width(line_number_width);
    let left_column = log_view_state.left_column();
    let text_width = usize::from(area.width).saturating_sub(gutter_width);
    let visible_columns = left_column..left_column.saturating_add(text_width);

    for ((row_area, tab_row), row) in area.rows().zip(&tab_rows).zip(top_row..) {
        let is_bookmarked = bookmarks.contains(tab_row.line);
        let is_cursor_row = row == log_view_state.cursor_row();
        let row_background = if is_cursor_row {
            palette::BACKGROUND_HIGHLIGHT
        } else {
            palette::BACKGROUND
        };
        let mut display_line = gutter(
            tab_row.line,
            line_number_width,
            is_bookmarked,
            is_cursor_row,
        );
        let match_layers = match_layers(&tab_row.text, search_phrase, highlights);
        let VisibleRow {
            spans,
            is_cut_on_the_left,
            is_cut_on_the_right,
        } = visible_row(&tab_row.text, visible_columns.clone(), &match_layers);
        display_line.extend(spans);
        if is_cursor_row {
            display_line = display_line.bg(row_background);
        }
        frame.render_widget(display_line, row_area);

        render_cut_markers(
            frame.buffer_mut(),
            row_text_area(row_area, gutter_width),
            is_cut_on_the_left,
            is_cut_on_the_right,
            FAINT_TEXT_STYLE.bg(row_background),
        );
    }
}

fn render_empty_file_text(frame: &mut Frame, area: Rect) {
    let Some(middle_row_area) = area.rows().nth(usize::from(area.height / 2)) else {
        return;
    };

    let empty_file_line = Line::styled(EMPTY_FILE_TEXT, FAINT_TEXT_STYLE).centered();
    frame.render_widget(empty_file_line, middle_row_area);
}

fn row_text_area(row_area: Rect, gutter_width: usize) -> Rect {
    let gutter_width = u16::try_from(gutter_width).unwrap_or(u16::MAX);
    Rect {
        x: row_area.x.saturating_add(gutter_width),
        width: row_area.width.saturating_sub(gutter_width),
        ..row_area
    }
}

fn render_cut_markers(
    buffer: &mut Buffer,
    row_text_area: Rect,
    is_cut_on_the_left: bool,
    is_cut_on_the_right: bool,
    marker_style: Style,
) {
    if row_text_area.is_empty() {
        return;
    }

    let row = row_text_area.y;
    if is_cut_on_the_left {
        buffer[(row_text_area.left(), row)]
            .set_symbol(LEFT_CUT_MARKER)
            .set_style(marker_style);
    }
    if is_cut_on_the_right {
        buffer[(row_text_area.right() - 1, row)]
            .set_symbol(RIGHT_CUT_MARKER)
            .set_style(marker_style);
    }
}

fn match_layers(
    line_text: &str,
    search_phrase: Option<&str>,
    highlights: &Highlights,
) -> Vec<MatchLayer> {
    let search_layer = search_phrase.map(|phrase| MatchLayer {
        match_ranges: match_ranges(line_text, phrase),
        style: SEARCH_MATCH_STYLE,
    });
    let highlight_layers_newest_first =
        highlights.oldest_first().rev().map(|highlight| MatchLayer {
            match_ranges: match_ranges(line_text, &highlight.phrase),
            style: HIGHLIGHT_STYLES[highlight.color_index],
        });

    search_layer
        .into_iter()
        .chain(highlight_layers_newest_first)
        .collect()
}

fn match_ranges(line_text: &str, phrase: &str) -> Vec<Range<usize>> {
    if phrase.is_empty() {
        return Vec::new();
    }

    line_text
        .match_indices(phrase)
        .map(|(match_start, _)| match_start..match_start + phrase.len())
        .collect()
}

fn line_number_width(line_count: usize) -> usize {
    line_count.max(1).ilog10() as usize + 1
}

fn gutter(
    line: usize,
    line_number_width: usize,
    is_bookmarked: bool,
    is_cursor_row: bool,
) -> Line<'static> {
    let marker = if is_bookmarked { BOOKMARK_MARKER } else { ' ' };
    let line_number_style = if is_cursor_row {
        CURSOR_LINE_NUMBER_STYLE
    } else {
        GUTTER_STYLE
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), BOOKMARK_MARKER_STYLE),
        Span::styled(
            format!("{:>line_number_width$}", line + 1),
            line_number_style,
        ),
        Span::styled(GUTTER_SEPARATOR, GUTTER_STYLE),
    ])
}

fn gutter_width(line_number_width: usize) -> usize {
    BOOKMARK_COLUMN_WIDTH + line_number_width + GUTTER_SEPARATOR.chars().count()
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, style::Color};

    use super::*;
    use crate::{
        domain::tabs::Tabs,
        ui::tests::{render_to_buffer, rendered_rows},
    };

    fn make_tabs(width: u16, height: u16) -> Tabs {
        let mut tabs = Tabs::default();
        tabs.set_log_view_size(usize::from(height), usize::from(width), 0);
        tabs
    }

    fn render_log_view(log_file: &LogFile, tabs: &Tabs, width: u16, height: u16) -> Buffer {
        render_with(
            log_file,
            tabs,
            None,
            &Highlights::default(),
            &Bookmarks::default(),
            width,
            height,
        )
    }

    fn render_with(
        log_file: &LogFile,
        tabs: &Tabs,
        search_phrase: Option<&str>,
        highlights: &Highlights,
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
                search_phrase,
                highlights,
                bookmarks,
            );
        })
    }

    fn render_highlighted(
        text: &str,
        search_phrase: Option<&str>,
        highlighted_phrases: &[&str],
        width: u16,
    ) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut highlights = Highlights::default();
        for phrase in highlighted_phrases {
            highlights.toggle((*phrase).to_owned());
        }
        render_with(
            &log_file,
            &make_tabs(width, 1),
            search_phrase,
            &highlights,
            &Bookmarks::default(),
            width,
            1,
        )
    }

    fn backgrounds(buffer: &Buffer, columns: Range<u16>) -> Vec<Color> {
        columns.map(|column| buffer[(column, 0)].bg).collect()
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
            ["   9 │ line 9", "  10 │ line 10", "  11 │ line 11"]
        );
    }

    #[test]
    fn given_search_phrase_when_rendering_then_every_occurrence_on_visible_lines_is_highlighted() {
        let log_file = LogFile::from_text("ERROR a ERROR\nok\nERROR\n");
        let tabs = make_tabs(20, 3);

        let buffer = render_with(
            &log_file,
            &tabs,
            Some("ERROR"),
            &Highlights::default(),
            &Bookmarks::default(),
            20,
            3,
        );

        let is_highlighted = |column, row| buffer[(column, row)].bg == palette::YELLOW;
        let gutter_width = 6;
        assert!(is_highlighted(gutter_width, 0) && is_highlighted(gutter_width + 12, 0));
        assert!(!is_highlighted(gutter_width + 6, 0));
        assert!(!is_highlighted(gutter_width, 1));
        assert!(is_highlighted(gutter_width + 4, 2));
    }

    #[test]
    fn given_no_search_phrase_when_rendering_then_nothing_is_highlighted() {
        let log_file = LogFile::from_text("ERROR\n");

        let buffer = render_log_view(&log_file, &make_tabs(20, 1), 20, 1);

        assert_ne!(buffer[(6, 0)].bg, palette::YELLOW);
    }

    #[test]
    fn given_empty_phrase_when_finding_matches_then_there_are_none() {
        assert!(match_ranges("abc", "").is_empty());
    }

    #[test]
    fn given_two_highlights_when_rendering_then_each_phrase_has_its_own_bright_color_and_dark_text()
    {
        let buffer = render_highlighted("ERROR user=42\n", None, &["ERROR", "user=42"], 20);

        assert_eq!(backgrounds(&buffer, 6..11), [palette::CORAL; 5]);
        assert_ne!(buffer[(11, 0)].bg, palette::CORAL);
        assert_eq!(backgrounds(&buffer, 12..19), [palette::TEAL; 7]);
        assert_eq!(buffer[(6, 0)].fg, palette::BACKGROUND);
    }

    #[test]
    fn given_seven_highlights_when_rendering_then_the_seventh_is_a_darker_shade_with_the_log_text_color()
     {
        let phrases = ["a", "b", "c", "d", "e", "f", "g"];

        let buffer = render_highlighted("a b c d e f g\n", None, &phrases, 20);

        let seventh_phrase_cell = &buffer[(18, 0)];
        assert_eq!(seventh_phrase_cell.bg, palette::CORAL_DARK);
        assert_eq!(seventh_phrase_cell.fg, palette::FOREGROUND);
    }

    #[test]
    fn given_search_match_inside_a_highlight_when_rendering_then_the_search_match_is_on_top() {
        let buffer = render_highlighted("user=42\n", Some("42"), &["user=42"], 20);

        assert_eq!(backgrounds(&buffer, 6..11), [palette::CORAL; 5]);
        assert_eq!(backgrounds(&buffer, 11..13), [palette::YELLOW; 2]);
    }

    #[test]
    fn given_overlapping_highlights_when_rendering_then_the_newer_one_is_on_top() {
        let buffer = render_highlighted("took=442ms\n", None, &["took=4", "442"], 20);

        assert_eq!(backgrounds(&buffer, 6..11), [palette::CORAL; 5]);
        assert_eq!(backgrounds(&buffer, 11..14), [palette::TEAL; 3]);
    }

    #[test]
    fn given_highlight_on_the_cursor_row_when_rendering_then_it_keeps_its_color_over_the_tint() {
        let log_file = LogFile::from_text("an ERROR\n");
        let mut highlights = Highlights::default();
        highlights.toggle("ERROR".to_owned());

        let buffer = render_with(
            &log_file,
            &make_tabs(20, 1),
            None,
            &highlights,
            &Bookmarks::default(),
            20,
            1,
        );

        assert_eq!(
            backgrounds(&buffer, 6..9),
            [palette::BACKGROUND_HIGHLIGHT; 3]
        );
        assert_eq!(backgrounds(&buffer, 9..14), [palette::CORAL; 5]);
    }

    #[test]
    fn given_bookmarked_line_when_rendering_then_its_gutter_starts_with_the_marker() {
        let log_file = LogFile::from_text("a\nb\n");
        let mut bookmarks = Bookmarks::default();
        bookmarks.toggle(1);

        let buffer = render_with(
            &log_file,
            &make_tabs(20, 2),
            None,
            &Highlights::default(),
            &bookmarks,
            20,
            2,
        );

        assert_eq!(rendered_rows(&buffer), ["  1 │ a", "● 2 │ b"]);
        assert_eq!(buffer[(0, 1)].fg, palette::YELLOW);
    }

    #[test]
    fn given_cursor_row_when_rendering_then_its_text_is_tinted_but_not_its_gutter() {
        let buffer = render_text("a\nb\nc\n", 1, 10, 3);

        let is_tinted = |column, row| buffer[(column, row)].bg == palette::BACKGROUND_HIGHLIGHT;
        assert!(is_tinted(6, 1) && is_tinted(9, 1));
        assert!(!is_tinted(0, 1) && !is_tinted(5, 1));
        assert!(!is_tinted(6, 0) && !is_tinted(6, 2));
    }

    #[test]
    fn given_cursor_row_when_rendering_then_only_its_line_number_is_orange_and_bold() {
        let buffer = render_text("a\nb\nc\n", 1, 10, 3);

        assert_eq!(buffer[(2, 1)].fg, palette::ORANGE);
        assert!(buffer[(2, 1)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(2, 0)].fg, palette::FOREGROUND_FAINT);
    }

    #[test]
    fn given_rows_past_the_end_of_the_file_when_rendering_then_they_show_the_background() {
        let buffer = render_text("a\n", 0, 10, 3);

        assert_eq!(buffer[(0, 2)].bg, palette::BACKGROUND);
        assert_eq!(buffer[(9, 2)].bg, palette::BACKGROUND);
    }

    #[test]
    fn given_line_wider_than_the_area_when_rendering_then_a_faint_marker_covers_its_last_column() {
        let buffer = render_text("abcdefghij\n", 0, 8, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ a>"]);
        assert_eq!(buffer[(7, 0)].fg, palette::FOREGROUND_FAINT);
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
    fn given_control_characters_when_rendering_then_they_show_muted_in_caret_notation() {
        let buffer = render_text("a\x1bb\rc\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ a^[b^Mc"]);
        let is_muted = |column| buffer[(column, 0)].fg == palette::FOREGROUND_MUTED;
        assert!(is_muted(7) && is_muted(8));
        assert!(!is_muted(6) && !is_muted(9));
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

        assert_eq!(rendered_rows(&buffer), ["  1 │ a>"]);
    }

    #[test]
    fn given_empty_file_when_rendering_then_empty_file_shows_faintly_in_the_middle() {
        let buffer = render_text("", 0, 20, 3);

        assert_eq!(rendered_rows(&buffer), ["", "     empty file", ""]);
        assert_eq!(buffer[(5, 1)].fg, palette::FOREGROUND_FAINT);
    }

    #[test]
    fn given_scrolled_view_when_rendering_then_every_line_is_cut_and_the_gutter_stays() {
        let buffer = render_scrolled("abcdef\nxyz\n", 2, 20, 2);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <def", "  2 │ <"]);
    }

    #[test]
    fn given_wide_character_cut_by_the_left_edge_when_rendering_then_the_marker_covers_its_visible_half()
     {
        let buffer = render_scrolled("漢字x\n", 1, 14, 1);

        let symbol_at = |column| buffer[(column, 0)].symbol().to_owned();
        assert_eq!(symbol_at(6), "<");
        assert_eq!(symbol_at(7), "字");
        assert_eq!(symbol_at(9), "x");
    }

    #[test]
    fn given_tab_left_of_the_edge_when_scrolled_then_text_after_it_keeps_its_tab_stop() {
        let buffer = render_scrolled("a\tb\n", 4, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <   b"]);
    }

    #[test]
    fn given_control_character_left_of_the_edge_when_scrolled_then_its_caret_notation_is_skipped() {
        let buffer = render_scrolled("\x1bXY\n", 2, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <Y"]);
    }

    #[test]
    fn given_view_scrolled_past_the_end_of_a_line_when_rendering_then_only_the_gutter_and_the_marker_show()
     {
        let buffer = render_scrolled("abc\n", 10, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <"]);
    }

    #[test]
    fn given_cursor_row_cut_on_both_sides_when_rendering_then_its_markers_keep_the_tint() {
        let buffer = render_scrolled("abcdefghij\n", 2, 10, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <de>"]);
        for column in [6, 9] {
            assert_eq!(buffer[(column, 0)].fg, palette::FOREGROUND_FAINT);
            assert_eq!(buffer[(column, 0)].bg, palette::BACKGROUND_HIGHLIGHT);
        }
    }

    #[test]
    fn given_area_narrower_than_the_gutter_when_rendering_a_cut_line_then_no_marker_is_drawn() {
        let buffer = render_scrolled("abcdef\n", 2, 4, 1);

        assert_eq!(rendered_rows(&buffer), ["  1"]);
    }

    #[test]
    fn given_caret_notation_cut_by_the_left_edge_when_scrolled_then_the_marker_covers_its_visible_half()
     {
        let buffer = render_scrolled("\x1bXY\n", 1, 14, 1);

        assert_eq!(rendered_rows(&buffer), ["  1 │ <XY"]);
    }

    #[test]
    fn given_arabic_ligature_before_a_tab_when_rendering_then_the_tab_still_ends_on_a_stop() {
        let buffer = render_text("\u{644}\u{627}\tx\n", 0, 30, 1);

        let x_column = (0..30).find(|&column| buffer[(column, 0)].symbol() == "x");
        assert_eq!(x_column, Some(6 + 8));
    }
}
