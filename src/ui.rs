mod command_line;
mod log_view;
mod status_bar;
mod visible_row;

use ratatui::Frame;

use crate::{
    app::{Model, ScreenAreas},
    control::Mode,
};

pub fn render(frame: &mut Frame, model: &Model) {
    let screen_areas = ScreenAreas::new(frame.area());
    log_view::render(
        frame,
        screen_areas.log_view,
        &model.log_file,
        &model.log_view_state,
        model
            .search_state
            .query()
            .map(|search_query| search_query.phrase.as_str()),
        &model.bookmarks,
    );
    match model.input_controller.mode() {
        Mode::CommandLine(command_line) => command_line::render(
            frame,
            screen_areas.bottom_row,
            command_line.prefix(),
            command_line.text(),
            command_line.text_before_cursor(),
        ),
        Mode::Normal => status_bar::render(frame, screen_areas.bottom_row, model),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        crossterm::event::KeyCode,
        layout::{Position, Size},
        style::Color,
    };

    use super::*;
    use crate::domain::log_file::LogFile;

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

    fn make_model() -> Model {
        let mut model =
            Model::with_screen_size(LogFile::from_text("first\nsecond\n"), Size::new(50, 3));
        model.finish_scheduled_jobs();
        model
    }

    fn submit_search(model: &mut Model, typed: &str) {
        model.type_keys(typed);
        model.press(KeyCode::Enter);
    }

    fn render_model(model: &Model) -> Vec<String> {
        rendered_rows(&render_to_buffer(50, 3, |frame| render(frame, model)))
    }

    fn cursor_position_after_render(model: &Model) -> Position {
        let mut terminal = Terminal::new(TestBackend::new(50, 3)).unwrap();
        terminal.draw(|frame| render(frame, model)).unwrap();
        terminal.get_cursor_position().unwrap()
    }

    #[test]
    fn given_open_command_line_when_rendering_then_it_replaces_the_status_bar() {
        let mut model = make_model();

        model.type_keys(":12");

        assert_eq!(render_model(&model), ["  1 │ first", "  2 │ second", ":12"]);
    }

    #[test]
    fn given_open_search_line_when_rendering_then_it_shows_the_search_prefix() {
        let mut model = make_model();

        model.type_keys("/ERR");

        assert_eq!(render_model(&model)[2], "/ERR");
    }

    #[test]
    fn given_wide_characters_on_the_command_line_when_rendering_then_the_cursor_is_at_the_editing_position()
     {
        let mut model = make_model();
        model.type_keys(":漢x");

        model.press(KeyCode::Left);

        assert_eq!(cursor_position_after_render(&model), Position::new(3, 2));
    }

    #[test]
    fn given_command_failed_when_rendering_then_the_error_shows_until_the_next_key() {
        let mut model = make_model();
        model.type_keys(":abc");
        model.press(KeyCode::Enter);
        assert_eq!(
            render_model(&model)[2],
            " app.log    Not a command: abc │ 1/2 (50%) │ 13 B"
        );

        model.press(KeyCode::Esc);

        assert_eq!(
            render_model(&model)[2],
            " app.log                 <Esc> │ 1/2 (50%) │ 13 B"
        );
    }

    #[test]
    fn given_app_when_rendering_then_log_view_is_above_the_status_bar() {
        let model = make_model();

        assert_eq!(
            render_model(&model),
            [
                "  1 │ first",
                "  2 │ second",
                " app.log                         1/2 (50%) │ 13 B"
            ]
        );
    }

    #[test]
    fn given_search_scheduled_when_rendering_then_the_status_bar_says_searching() {
        let mut model = make_model();

        submit_search(&mut model, "/sec");

        assert_eq!(
            render_model(&model)[2],
            " app.log            searching… │ 1/2 (50%) │ 13 B"
        );
    }

    #[test]
    fn given_search_done_when_rendering_then_the_status_bar_shows_the_query_and_match_position() {
        let mut model = make_model();
        submit_search(&mut model, "/sec");

        model.finish_scheduled_jobs();

        assert_eq!(
            render_model(&model)[2],
            " app.log           /sec [1/1] │ 2/2 (100%) │ 13 B"
        );
    }

    #[test]
    fn given_search_without_matches_when_rendering_then_the_status_bar_says_pattern_not_found() {
        let mut model = make_model();
        submit_search(&mut model, "/zz");

        model.finish_scheduled_jobs();

        assert_eq!(
            render_model(&model)[2],
            " app.log Pattern not found: zz │ 1/2 (50%) │ 13 B"
        );
    }

    #[test]
    fn given_search_cleared_with_escape_when_rendering_then_the_segment_and_the_highlight_are_gone()
    {
        let mut model = make_model();
        submit_search(&mut model, "/sec");
        model.finish_scheduled_jobs();
        let is_highlighted = |model: &Model| {
            let buffer = render_to_buffer(50, 3, |frame| render(frame, model));
            buffer[(6, 1)].bg == Color::Yellow
        };
        assert!(is_highlighted(&model));

        model.press(KeyCode::Esc);

        assert_eq!(
            render_model(&model)[2],
            " app.log                <Esc> │ 2/2 (100%) │ 13 B"
        );
        assert!(!is_highlighted(&model));
    }

    #[test]
    fn given_count_typed_when_rendering_then_the_status_bar_shows_it_before_the_position() {
        let mut model = make_model();

        model.type_keys("5");

        assert_eq!(
            render_model(&model)[2],
            " app.log                     5 │ 1/2 (50%) │ 13 B"
        );
    }
}
