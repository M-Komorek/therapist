use ratatui::{Frame, text::Text};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let session = app.session();
    let placeholder = format!(
        "line {}/{}\nq to quit",
        session.tab.cursor() + 1,
        session.log_file.line_count()
    );
    frame.render_widget(Text::raw(placeholder), frame.area());
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::session::Session;

    #[test]
    fn given_app_when_rendering_then_cursor_position_and_quit_hint_are_shown() {
        let app = App::new(Session::from_text("first\nsecond\nthird\n"));
        let mut terminal = Terminal::new(TestBackend::new(9, 2)).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["line 1/3", "q to quit"]);
    }
}
