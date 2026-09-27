mod log_view;
mod status_bar;

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
    log_view::render(
        frame,
        screen_areas.log_view,
        &app.session().log_file,
        &app.session().tab,
    );
    status_bar::render(
        frame,
        screen_areas.status_bar,
        &app.session().log_file,
        &app.session().tab,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::session::Session;

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

    #[test]
    fn given_app_when_rendering_then_log_view_is_above_the_status_bar() {
        let app = App::new(Session::from_text("first\nsecond\n"));
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        assert_eq!(
            rendered_rows(terminal.backend().buffer()),
            [" 1 │ first", " 2 │ second", "app.log    1/2 (50%)"]
        );
    }
}
