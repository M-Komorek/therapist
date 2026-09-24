use ratatui::{Frame, text::Text};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let placeholder = format!("{}\nq to quit", app.file().display());
    frame.render_widget(Text::raw(placeholder), frame.area());
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn given_app_when_rendering_then_file_path_and_quit_hint_are_shown() {
        let app = App::new(PathBuf::from("/var/log/app.log"));
        let mut terminal = Terminal::new(TestBackend::new(16, 2)).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["/var/log/app.log", "q to quit"]);
    }
}
