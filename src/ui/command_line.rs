use ratatui::{Frame, buffer::CellWidth, layout::Rect, text::Line};

pub fn render(frame: &mut Frame, area: Rect, prefix: char, text: &str, text_before_cursor: &str) {
    frame.render_widget(Line::raw(format!("{prefix}{text}")), area);
    let prefix_width = 1;
    let cursor_column = text_before_cursor.cell_width();
    frame.set_cursor_position((
        (area.x + prefix_width).saturating_add(cursor_column),
        area.y,
    ));
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, layout::Position};

    use super::*;
    use crate::ui::tests::rendered_rows;

    fn render_command_line(text: &str, text_before_cursor: &str) -> (String, Position) {
        let mut terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
        terminal
            .draw(|frame| {
                let bottom_row = Rect::new(0, 1, 10, 1);
                render(frame, bottom_row, ':', text, text_before_cursor);
            })
            .unwrap();
        let rows = rendered_rows(terminal.backend().buffer());
        (rows[1].clone(), terminal.get_cursor_position().unwrap())
    }

    #[test]
    fn given_text_when_rendering_then_it_follows_the_prefix() {
        let (row, _) = render_command_line("5000", "5000");

        assert_eq!(row, ":5000");
    }

    #[test]
    fn given_cursor_inside_the_text_when_rendering_then_the_terminal_cursor_is_after_the_prefix_in_its_row()
     {
        let (_, cursor_position) = render_command_line("5000", "50");

        assert_eq!(cursor_position, Position::new(3, 1));
    }

    #[test]
    fn given_wide_character_before_the_cursor_when_rendering_then_the_cursor_counts_its_display_columns()
     {
        let (_, cursor_position) = render_command_line("漢字", "漢");

        assert_eq!(cursor_position, Position::new(3, 1));
    }

    #[test]
    fn given_empty_text_when_rendering_then_only_the_prefix_shows_and_the_cursor_is_after_it() {
        let (row, cursor_position) = render_command_line("", "");

        assert_eq!(row, ":");
        assert_eq!(cursor_position, Position::new(1, 1));
    }
}
