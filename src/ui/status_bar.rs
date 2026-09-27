use ratatui::{Frame, layout::Rect, text::Line};

use crate::{log_file::LogFile, tabs::Tab};

pub fn render(frame: &mut Frame, area: Rect, log_file: &LogFile, tab: &Tab) {
    let position = position(tab.cursor(), log_file.line_count());
    frame.render_widget(Line::raw(log_file.file_name()), area);
    frame.render_widget(Line::raw(position).right_aligned(), area);
}

fn position(cursor: usize, line_count: usize) -> String {
    if line_count == 0 {
        return "0/0".to_owned();
    }
    let line_number = cursor + 1;
    let percent = line_number * 100 / line_count;
    format!("{line_number}/{line_count} ({percent}%)")
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{session::Session, ui::tests::rendered_rows};

    fn render_status(text: &str, cursor: usize) -> String {
        let mut session = Session::from_text(text);
        session.tab.move_down(cursor, session.log_file.line_count());
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &session.log_file, &session.tab))
            .unwrap();
        rendered_rows(terminal.backend().buffer()).remove(0)
    }

    #[test]
    fn given_cursor_on_second_of_four_lines_when_rendering_then_name_is_left_and_position_is_right()
    {
        assert_eq!(render_status("a\nb\nc\nd\n", 1), "app.log    2/4 (50%)");
    }

    #[test]
    fn given_empty_file_when_rendering_then_position_is_zero_of_zero() {
        assert_eq!(render_status("", 0), "app.log          0/0");
    }

    #[test]
    fn given_large_file_when_formatting_position_then_percent_is_rounded_down() {
        assert_eq!(position(1203, 88213), "1204/88213 (1%)");
    }
}
