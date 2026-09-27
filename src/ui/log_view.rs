use ratatui::{
    Frame,
    buffer::CellWidth,
    layout::Rect,
    style::Stylize,
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
    for ((row, text), line) in area.rows().zip(&texts).zip(first_line..) {
        let mut display_line = Line::from(gutter(line, number_width));
        display_line.extend(display_spans(text));
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

fn display_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut plain_text = String::new();
    let mut unmeasured_start = 0;
    let mut column = 0;
    for character in text.chars() {
        if !character.is_control() {
            plain_text.push(character);
            continue;
        }
        // Measured as a whole, so emoji sequences get the width ratatui gives them.
        column += usize::from(plain_text[unmeasured_start..].cell_width());
        if character == '\t' {
            let spaces = TAB_STOP - column % TAB_STOP;
            plain_text.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else if character.is_ascii_control() {
            spans.push(Span::raw(std::mem::take(&mut plain_text)));
            spans.push(caret_notation(character).dim());
            column += 2;
        }
        // Other control characters (C1) are dropped, as ratatui would draw nothing for them.
        unmeasured_start = plain_text.len();
    }
    spans.push(Span::raw(plain_text));
    spans
}

fn caret_notation(control: char) -> String {
    format!("^{}", char::from(control as u8 ^ 0x40))
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};

    use super::*;
    use crate::{session::Session, ui::tests::rendered_rows};

    fn render_text(text: &str, cursor: usize, width: u16, height: u16) -> Buffer {
        let mut session = Session::from_text(text);
        session.tab.set_viewport_height(usize::from(height));
        session.tab.move_down(cursor, session.log_file.line_count());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &session.log_file, &session.tab))
            .unwrap();
        terminal.backend().buffer().clone()
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
    fn given_nul_and_del_when_converting_then_they_use_standard_caret_notation() {
        assert_eq!(caret_notation('\0'), "^@");
        assert_eq!(caret_notation('\x7f'), "^?");
    }

    #[test]
    fn given_empty_file_when_rendering_then_the_view_is_blank() {
        let buffer = render_text("", 0, 10, 2);

        assert_eq!(rendered_rows(&buffer), ["", ""]);
    }
}
