mod command_line;
mod log_view;
mod status_bar;
mod visible_row;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

pub use status_bar::StatusMessage;

use crate::{app::App, control::Mode};

pub struct ScreenAreas {
    pub log_view: Rect,
    pub bottom_row: Rect,
}

impl ScreenAreas {
    pub fn new(screen: Rect) -> Self {
        let [log_view, bottom_row] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(screen);
        Self {
            log_view,
            bottom_row,
        }
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let screen_areas = ScreenAreas::new(frame.area());
    let session = app.session();
    log_view::render(
        frame,
        screen_areas.log_view,
        &session.log_file,
        &session.tab,
    );
    match app.control().mode() {
        Mode::CommandLine(command_line) => command_line::render(
            frame,
            screen_areas.bottom_row,
            ':',
            command_line.text(),
            command_line.cursor_column(),
        ),
        Mode::Normal => status_bar::render(
            frame,
            screen_areas.bottom_row,
            &session.log_file,
            &session.tab,
            &app.control().last_keys_text(),
            app.status_message(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        layout::{Position, Size},
    };

    use super::*;
    use crate::{domain::session::Session, infra::terminal::Event};

    pub(super) fn render_to_buffer(
        width: u16,
        height: u16,
        render: impl FnOnce(&mut Frame),
    ) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(render).unwrap();
        terminal.backend().buffer().clone()
    }

    pub(super) fn rendered_rows(buffer: &Buffer) -> Vec<String> {
        buffer
            .content
            .chunks(usize::from(buffer.area.width))
            .map(|row| {
                let text: String = row.iter().map(|cell| cell.symbol()).collect();
                text.trim_end().to_owned()
            })
            .collect()
    }

    fn make_app() -> App {
        App::new(Session::from_text("first\nsecond\n"), Size::new(30, 3))
    }

    fn render_app(app: &App) -> Vec<String> {
        rendered_rows(&render_to_buffer(30, 3, |frame| render(frame, app)))
    }

    fn type_keys(app: &mut App, typed: &str) {
        for character in typed.chars() {
            app.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::NONE,
            )));
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(Event::Key(code.into()));
    }

    fn cursor_position_after_render(app: &App) -> Position {
        let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal.get_cursor_position().unwrap()
    }

    #[test]
    fn given_open_command_line_when_rendering_then_it_replaces_the_status_bar() {
        let mut app = make_app();

        type_keys(&mut app, ":12");

        assert_eq!(render_app(&app), [" 1 │ first", " 2 │ second", ":12"]);
    }

    #[test]
    fn given_wide_characters_on_the_command_line_when_rendering_then_the_cursor_is_at_the_editing_position()
     {
        let mut app = make_app();
        type_keys(&mut app, ":漢x");

        press(&mut app, KeyCode::Left);

        assert_eq!(cursor_position_after_render(&app), Position::new(3, 2));
    }

    #[test]
    fn given_command_failed_when_rendering_then_the_error_shows_until_the_next_key() {
        let mut app = make_app();
        type_keys(&mut app, ":abc");
        press(&mut app, KeyCode::Enter);
        assert_eq!(render_app(&app)[2], "Not a command: abc   1/2 (50%)");

        press(&mut app, KeyCode::Esc);

        assert_eq!(render_app(&app)[2], "app.log              1/2 (50%)");
    }

    #[test]
    fn given_app_when_rendering_then_log_view_is_above_the_status_bar() {
        let app = make_app();

        assert_eq!(
            render_app(&app),
            [
                " 1 │ first",
                " 2 │ second",
                "app.log              1/2 (50%)"
            ]
        );
    }

    #[test]
    fn given_count_typed_when_rendering_then_the_status_bar_shows_it_before_the_position() {
        let mut app = make_app();

        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('5'),
            KeyModifiers::NONE,
        )));

        assert_eq!(render_app(&app)[2], "app.log          5 | 1/2 (50%)");
    }
}
