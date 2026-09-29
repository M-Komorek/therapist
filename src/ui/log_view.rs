use std::ops::Range;

use ratatui::{
    Frame,
    buffer::CellWidth,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
};

use crate::{log_file::LogFile, tabs::Tab};

const TAB_STOP: usize = 8;

pub fn render(frame: &mut Frame, area: Rect, log_file: &LogFile, tab: &Tab) {
    let first_line = tab.viewport().top();
    let texts = match log_file.lines(first_line..first_line + usize::from(area.height)) {
        Ok(texts) => texts,
        Err(error) => {
            frame.render_widget(Line::raw(error.to_string()), area);
            return;
        }
    };
    let number_width = number_width(log_file.line_count());
    let left_column = tab.viewport().left_column();
    let visible_columns = left_column..left_column.saturating_add(usize::from(area.width));
    for ((row, text), line) in area.rows().zip(&texts).zip(first_line..) {
        let mut display_line = Line::from(gutter(line, number_width));
        display_line.extend(visible_spans(text, visible_columns.clone()));
        if line == tab.cursor() {
            display_line = display_line.reversed();
        }
        frame.render_widget(display_line, row);
    }
}

fn number_width(line_count: usize) -> usize {
    line_count.max(1).ilog10() as usize + 1
}

fn gutter(line: usize, number_width: usize) -> String {
    format!(" {:>number_width$} │ ", line + 1)
}

/// Turns a log line into the spans shown inside the display `columns`. Tabs expand to spaces up to
/// the next stop, and ASCII control characters show dimmed in caret notation.
fn visible_spans(text: &str, columns: Range<usize>) -> Vec<Span<'static>> {
    if is_printable_ascii(text) {
        return vec![Span::raw(printable_ascii_window(text, columns))];
    }
    let mut row = VisibleRow::new(columns);
    let mut plain_text_start = 0;
    let controls = text
        .char_indices()
        .filter(|(_, character)| character.is_control());
    for (index, control) in controls {
        row.push_text(&text[plain_text_start..index]);
        row.push_control(control);
        plain_text_start = index + control.len_utf8();
    }
    row.push_text(&text[plain_text_start..]);
    row.spans
}

fn is_printable_ascii(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, b' '..=b'~'))
}

/// Printable ASCII takes one column per byte, so the window is a plain slice.
fn printable_ascii_window(text: &str, columns: Range<usize>) -> String {
    let start = columns.start.min(text.len());
    let end = columns.end.min(text.len());
    text[start..end].to_owned()
}

/// A display row cut to a window of columns, built from the left one symbol at a time. A symbol cut
/// in half by the left edge becomes spaces, so the rest of the row keeps its columns.
struct VisibleRow {
    columns: Range<usize>,
    column: usize,
    spans: Vec<Span<'static>>,
}

impl VisibleRow {
    fn new(columns: Range<usize>) -> Self {
        Self {
            columns,
            column: 0,
            spans: Vec::new(),
        }
    }

    fn push_text(&mut self, text: &str) {
        let span = Span::raw(text);
        for grapheme in span.styled_graphemes(Style::default()) {
            if self.is_full() {
                return;
            }
            let width = usize::from(grapheme.symbol.cell_width());
            self.push_symbol(grapheme.symbol, width, Style::default());
        }
    }

    fn push_control(&mut self, control: char) {
        if control == '\t' {
            let spaces = TAB_STOP - self.column % TAB_STOP;
            self.push_symbol(&" ".repeat(spaces), spaces, Style::default());
        } else if control.is_ascii_control() {
            self.push_symbol(&caret_notation(control), 2, Style::new().dim());
        }
        // Other control characters (C1) are dropped, as ratatui would draw nothing for them.
    }

    fn push_symbol(&mut self, symbol: &str, width: usize, style: Style) {
        if self.is_full() {
            return;
        }
        let next_column = self.column + width;
        if self.column >= self.columns.start {
            self.append(symbol, style);
        } else if next_column > self.columns.start {
            let visible_width = next_column - self.columns.start;
            self.append(&" ".repeat(visible_width), Style::default());
        }
        self.column = next_column;
    }

    fn append(&mut self, text: &str, style: Style) {
        match self.spans.last_mut() {
            Some(span) if span.style == style => span.content.to_mut().push_str(text),
            _ => self.spans.push(Span::styled(text.to_owned(), style)),
        }
    }

    fn is_full(&self) -> bool {
        self.column >= self.columns.end
    }
}

fn caret_notation(control: char) -> String {
    format!("^{}", char::from(control as u8 ^ 0x40))
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};

    use super::*;
    use crate::{session::Session, ui::tests::rendered_rows};

    fn render_session(session: &Session, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &session.log_file, &session.tab))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_text(text: &str, cursor: usize, width: u16, height: u16) -> Buffer {
        let mut session = Session::from_text(text);
        session.tab.set_viewport_height(usize::from(height));
        session.tab.move_down(cursor, session.log_file.line_count());
        render_session(&session, width, height)
    }

    fn render_scrolled(text: &str, columns: usize, width: u16, height: u16) -> Buffer {
        let mut session = Session::from_text(text);
        session.tab.scroll_right(columns);
        render_session(&session, width, height)
    }

    #[test]
    fn given_scrolled_view_when_rendering_then_numbers_are_right_aligned_in_the_gutter() {
        let text: String = (1..=12)
            .map(|line_number| format!("line {line_number}\n"))
            .collect();

        let buffer = render_text(&text, 9, 20, 3);

        assert_eq!(
            rendered_rows(&buffer),
            ["  8 │ line 8", "  9 │ line 9", " 10 │ line 10"]
        );
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

        assert_eq!(rendered_rows(&buffer), [" 1 │ abc"]);
    }

    #[test]
    fn given_tabs_when_rendering_then_they_expand_to_stops_every_eight_columns() {
        let buffer = render_text("ab\tc\td\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │ ab      c       d"]);
    }

    #[test]
    fn given_wide_characters_and_emoji_before_tabs_when_rendering_then_tabs_still_end_on_stops() {
        let buffer = render_text("漢\t❤\u{fe0f}\tx\n", 0, 30, 1);

        let x_column = (0..30).find(|&column| buffer[(column, 0)].symbol() == "x");
        assert_eq!(x_column, Some(5 + 16));
    }

    #[test]
    fn given_c1_control_before_a_tab_when_rendering_then_it_is_dropped_without_moving_the_tab() {
        let buffer = render_text("a\u{85}\tx\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │ a       x"]);
    }

    #[test]
    fn given_control_characters_when_rendering_then_they_show_dimmed_in_caret_notation() {
        let buffer = render_text("a\x1b[31mb\rc\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │ a^[[31mb^Mc"]);
        let is_dim = |column| buffer[(column, 0)].modifier.contains(Modifier::DIM);
        assert!(is_dim(6) && is_dim(7));
        assert!(!is_dim(5) && !is_dim(8));
    }

    #[test]
    fn given_del_character_when_rendering_then_it_shows_in_caret_notation() {
        let buffer = render_text("a\x7fb\n", 0, 30, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │ a^?b"]);
    }

    #[test]
    fn given_nul_and_del_when_converting_then_they_use_standard_caret_notation() {
        assert_eq!(caret_notation('\0'), "^@");
        assert_eq!(caret_notation('\x7f'), "^?");
    }

    #[test]
    fn given_empty_file_when_rendering_then_the_view_is_blank() {
        let buffer = render_text("", 0, 10, 2);

        assert_eq!(rendered_rows(&buffer), ["", ""]);
    }

    #[test]
    fn given_scrolled_view_when_rendering_then_every_line_is_cut_and_the_gutter_stays() {
        let buffer = render_scrolled("abcdef\nxyz\n", 2, 20, 2);

        assert_eq!(rendered_rows(&buffer), [" 1 │ cdef", " 2 │ z"]);
    }

    #[test]
    fn given_wide_character_cut_by_the_left_edge_when_rendering_then_its_visible_half_is_a_space() {
        let buffer = render_scrolled("漢字x\n", 1, 14, 1);

        let symbol_at = |column| buffer[(column, 0)].symbol().to_owned();
        assert_eq!(symbol_at(5), " ");
        assert_eq!(symbol_at(6), "字");
        assert_eq!(symbol_at(8), "x");
    }

    #[test]
    fn given_tab_left_of_the_edge_when_scrolled_then_text_after_it_keeps_its_tab_stop() {
        let buffer = render_scrolled("a\tb\n", 4, 14, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │     b"]);
    }

    #[test]
    fn given_control_character_left_of_the_edge_when_scrolled_then_its_caret_notation_is_skipped() {
        let buffer = render_scrolled("\x1bXY\n", 2, 14, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │ XY"]);
    }

    #[test]
    fn given_view_scrolled_past_the_end_of_a_line_when_rendering_then_only_the_gutter_shows() {
        let buffer = render_scrolled("abc\n", 10, 14, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │"]);
    }

    #[test]
    fn given_caret_notation_cut_by_the_left_edge_when_scrolled_then_its_visible_half_is_a_space() {
        let buffer = render_scrolled("\x1bXY\n", 1, 14, 1);

        assert_eq!(rendered_rows(&buffer), [" 1 │  XY"]);
    }

    #[test]
    fn given_arabic_ligature_before_a_tab_when_rendering_then_the_tab_still_ends_on_a_stop() {
        let buffer = render_text("\u{644}\u{627}\tx\n", 0, 30, 1);

        let x_column = (0..30).find(|&column| buffer[(column, 0)].symbol() == "x");
        assert_eq!(x_column, Some(5 + 8));
    }

    #[test]
    fn given_controls_inside_and_past_the_window_when_cutting_then_only_the_window_is_kept() {
        let visible = visible_spans("abc\x1bdefgh\x1b", 2..7);

        let texts: Vec<&str> = visible.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(texts, ["c", "^[", "de"]);
        assert_eq!(visible[1].style, Style::new().dim());
    }
}
