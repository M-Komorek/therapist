use ratatui::{Frame, layout::Rect, style::Stylize, text::Line};

use crate::{
    app::StatusMessage,
    domain::{
        log_file::{Indexing, LogFile},
        log_view_state::LogViewState,
    },
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    log_view_state: &LogViewState,
    last_keys: &str,
    status_message: Option<&StatusMessage>,
) {
    let indexing = log_file.indexing();
    let position = position_text(log_view_state.cursor(), log_file.line_count(), indexing);
    let indexing_progress = indexing_text(indexing, log_file.percent_indexed());
    let right_text = right_text(last_keys, position, indexing_progress);
    frame.render_widget(left_line(log_file, status_message), area);
    frame.render_widget(Line::raw(right_text).right_aligned(), area);
}

fn left_line<'a>(log_file: &'a LogFile, status_message: Option<&'a StatusMessage>) -> Line<'a> {
    match status_message {
        Some(StatusMessage::Error(message_text)) => Line::from(message_text.as_str().red()),
        None => Line::raw(log_file.file_name()),
    }
}

fn right_text(last_keys: &str, position: String, indexing_progress: Option<String>) -> String {
    let mut segments = Vec::new();
    if !last_keys.is_empty() {
        segments.push(last_keys.to_owned());
    }
    segments.push(position);
    segments.extend(indexing_progress);
    segments.join(" | ")
}

fn position_text(cursor: usize, line_count: usize, indexing: Indexing) -> String {
    let line_number = if line_count == 0 { 0 } else { cursor + 1 };
    let position = format!("{line_number}/{line_count}");
    match indexing {
        Indexing::Running | Indexing::Failed => format!("{position}+"),
        Indexing::Done if line_count == 0 => position,
        Indexing::Done => format!("{position} ({}%)", line_number * 100 / line_count),
    }
}

fn indexing_text(indexing: Indexing, percent_indexed: u64) -> Option<String> {
    match indexing {
        Indexing::Running => Some(format!("indexing {percent_indexed}%")),
        Indexing::Failed => Some("indexing failed".to_owned()),
        Indexing::Done => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, style::Color};

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_status(text: &str, cursor: usize, last_keys: &str) -> String {
        let log_file = LogFile::from_text(text);
        let mut log_view_state = LogViewState::default();
        log_view_state.move_down(cursor, log_file.line_count());
        rendered_rows(&render_status_buffer(
            &log_file,
            &log_view_state,
            last_keys,
            None,
        ))
        .remove(0)
    }

    fn render_log_file_status(log_file: &LogFile) -> String {
        rendered_rows(&render_status_buffer(
            log_file,
            &LogViewState::default(),
            "",
            None,
        ))
        .remove(0)
    }

    fn render_status_buffer(
        log_file: &LogFile,
        log_view_state: &LogViewState,
        last_keys: &str,
        status_message: Option<&StatusMessage>,
    ) -> Buffer {
        render_to_buffer(30, 1, |frame| {
            render(
                frame,
                frame.area(),
                log_file,
                log_view_state,
                last_keys,
                status_message,
            );
        })
    }

    #[test]
    fn given_error_message_when_rendering_then_it_replaces_the_file_name_in_red() {
        let log_file = LogFile::from_text("a\nb\n");
        let error_message = StatusMessage::Error("Not a command: x".to_owned());

        let buffer = render_status_buffer(
            &log_file,
            &LogViewState::default(),
            "",
            Some(&error_message),
        );

        assert_eq!(rendered_rows(&buffer), ["Not a command: x     1/2 (50%)"]);
        assert_eq!(buffer[(0, 0)].fg, Color::Red);
        assert_ne!(buffer[(29, 0)].fg, Color::Red);
    }

    #[test]
    fn given_cursor_on_second_of_four_lines_when_rendering_then_name_is_left_and_position_is_right()
    {
        assert_eq!(
            render_status("a\nb\nc\nd\n", 1, ""),
            "app.log              2/4 (50%)"
        );
    }

    #[test]
    fn given_empty_file_when_rendering_then_position_is_zero_of_zero() {
        assert_eq!(render_status("", 0, ""), "app.log                    0/0");
    }

    #[test]
    fn given_last_keys_when_rendering_then_a_separator_sets_them_apart_from_the_position() {
        assert_eq!(
            render_status("a\nb\nc\nd\n", 1, "5j"),
            "app.log         5j | 2/4 (50%)"
        );
    }

    #[test]
    fn given_large_file_when_formatting_position_then_percent_is_rounded_down() {
        assert_eq!(
            position_text(1203, 88213, Indexing::Done),
            "1204/88213 (1%)"
        );
    }

    #[test]
    fn given_file_still_indexing_when_rendering_then_total_has_a_plus_and_progress_follows() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");

        assert_eq!(
            render_log_file_status(&log_file),
            "app.log     0/0+ | indexing 0%"
        );
    }

    #[test]
    fn given_indexing_failed_when_rendering_then_total_has_a_plus_and_failure_is_shown() {
        let mut log_file = LogFile::from_text("a\nb\nc\nd\n");
        log_file.set_indexing(Indexing::Failed);

        assert_eq!(
            render_log_file_status(&log_file),
            "app.log 1/4+ | indexing failed"
        );
    }

    #[test]
    fn given_file_still_indexing_when_formatting_position_then_there_is_no_percent() {
        assert_eq!(position_text(1203, 88213, Indexing::Running), "1204/88213+");
    }

    #[test]
    fn given_indexing_running_when_formatting_progress_then_percent_is_shown() {
        assert_eq!(
            indexing_text(Indexing::Running, 45),
            Some("indexing 45%".to_owned())
        );
    }

    #[test]
    fn given_indexing_done_when_formatting_progress_then_nothing_is_shown() {
        assert_eq!(indexing_text(Indexing::Done, 100), None);
    }
}
