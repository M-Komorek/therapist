mod log_view;
mod status_bar;
mod visible_row;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::app::App;

pub struct ScreenAreas {
    pub log_view: Rect,
    pub status_bar: Rect,
}

impl ScreenAreas {
    pub fn new(screen: Rect) -> Self {
        let [log_view, status_bar] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(screen);
        Self {
            log_view,
            status_bar,
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
    status_bar::render(
        frame,
        screen_areas.status_bar,
        &session.log_file,
        &session.tab,
        &app.last_keys_text(),
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        layout::Size,
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
