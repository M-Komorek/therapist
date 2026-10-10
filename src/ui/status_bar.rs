use ratatui::{Frame, layout::Rect, style::Stylize, text::Line};

use crate::{
    app::{LogFileIndexingStatus, StatusMessage},
    domain::{log_file::LogFile, log_view_state::LogViewState},
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    log_file_indexing_status: LogFileIndexingStatus,
    log_view_state: &LogViewState,
    last_keys: &str,
    status_message: Option<&StatusMessage>,
) {
    let position_text = position_text(
        log_view_state.cursor_line(),
        log_file.line_count(),
        log_file_indexing_status,
    );
    let indexing_text = indexing_text(log_file_indexing_status, log_file.percent_indexed());
    let right_text = right_text(last_keys, position_text, indexing_text);
    frame.render_widget(left_line(log_file, status_message), area);
    frame.render_widget(Line::raw(right_text).right_aligned(), area);
}

fn left_line<'a>(log_file: &'a LogFile, status_message: Option<&'a StatusMessage>) -> Line<'a> {
    match status_message {
        Some(StatusMessage::Error(message_text)) => Line::from(message_text.as_str().red()),
        None => Line::raw(log_file.file_name()),
    }
}

fn right_text(last_keys: &str, position_text: String, indexing_text: Option<String>) -> String {
    let mut segments = Vec::new();
    if !last_keys.is_empty() {
        segments.push(last_keys.to_owned());
    }
    segments.push(position_text);
    segments.extend(indexing_text);
    segments.join(" | ")
}

fn position_text(
    cursor_line: usize,
    line_count: usize,
    log_file_indexing_status: LogFileIndexingStatus,
) -> String {
    let line_number = if line_count == 0 { 0 } else { cursor_line + 1 };
    let position = format!("{line_number}/{line_count}");
    match log_file_indexing_status {
        LogFileIndexingStatus::Running | LogFileIndexingStatus::Failed => format!("{position}+"),
        LogFileIndexingStatus::Done if line_count == 0 => position,
        LogFileIndexingStatus::Done => {
            format!("{position} ({}%)", line_number * 100 / line_count)
        }
    }
}

fn indexing_text(
    log_file_indexing_status: LogFileIndexingStatus,
    percent_indexed: u64,
) -> Option<String> {
    match log_file_indexing_status {
        LogFileIndexingStatus::Running => Some(format!("indexing {percent_indexed}%")),
        LogFileIndexingStatus::Failed => Some("indexing failed".to_owned()),
        LogFileIndexingStatus::Done => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, style::Color};

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_status(text: &str, cursor_line: usize, last_keys: &str) -> String {
        let log_file = LogFile::from_text(text);
        let mut log_view_state = LogViewState::default();
        log_view_state.move_cursor_down(cursor_line, log_file.line_count());
        rendered_rows(&render_status_buffer(
            &log_file,
            LogFileIndexingStatus::Done,
            &log_view_state,
            last_keys,
            None,
        ))
        .remove(0)
    }

    fn render_status_with_indexing(
        log_file: &LogFile,
        log_file_indexing_status: LogFileIndexingStatus,
    ) -> String {
        rendered_rows(&render_status_buffer(
            log_file,
            log_file_indexing_status,
            &LogViewState::default(),
            "",
            None,
        ))
        .remove(0)
    }

    fn render_status_buffer(
        log_file: &LogFile,
        log_file_indexing_status: LogFileIndexingStatus,
        log_view_state: &LogViewState,
        last_keys: &str,
        status_message: Option<&StatusMessage>,
    ) -> Buffer {
        render_to_buffer(30, 1, |frame| {
            render(
                frame,
                frame.area(),
                log_file,
                log_file_indexing_status,
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
            LogFileIndexingStatus::Done,
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
            position_text(1203, 88213, LogFileIndexingStatus::Done),
            "1204/88213 (1%)"
        );
    }

    #[test]
    fn given_file_still_indexing_when_rendering_then_total_has_a_plus_and_progress_follows() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");

        assert_eq!(
            render_status_with_indexing(&log_file, LogFileIndexingStatus::Running),
            "app.log     0/0+ | indexing 0%"
        );
    }

    #[test]
    fn given_indexing_failed_when_rendering_then_total_has_a_plus_and_failure_is_shown() {
        let log_file = LogFile::from_text("a\nb\nc\nd\n");

        assert_eq!(
            render_status_with_indexing(&log_file, LogFileIndexingStatus::Failed),
            "app.log 1/4+ | indexing failed"
        );
    }

    #[test]
    fn given_file_still_indexing_when_formatting_position_then_there_is_no_percent() {
        assert_eq!(
            position_text(1203, 88213, LogFileIndexingStatus::Running),
            "1204/88213+"
        );
    }

    #[test]
    fn given_indexing_running_when_formatting_progress_then_percent_is_shown() {
        assert_eq!(
            indexing_text(LogFileIndexingStatus::Running, 45),
            Some("indexing 45%".to_owned())
        );
    }

    #[test]
    fn given_indexing_done_when_formatting_progress_then_nothing_is_shown() {
        assert_eq!(indexing_text(LogFileIndexingStatus::Done, 100), None);
    }
}
