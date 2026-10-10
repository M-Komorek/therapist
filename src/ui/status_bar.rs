use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
};

use crate::{
    app::{LogFileIndexingStatus, StatusMessage},
    domain::{log_file::LogFile, log_view_state::LogViewState},
};

const BAR_STYLE: Style = Style::new().bg(Color::DarkGray);
const SEGMENT_SEPARATOR: &str = " │ ";
const FILE_SIZE_UNITS: [&str; 4] = ["kB", "MB", "GB", "TB"];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_file: &LogFile,
    log_file_indexing_status: LogFileIndexingStatus,
    log_view_state: &LogViewState,
    last_keys: &str,
    status_message: Option<&StatusMessage>,
) {
    let message_span = message_span(
        status_message,
        log_file_indexing_status,
        log_file.percent_indexed(),
    );
    let position_text = position_text(
        log_view_state.cursor_line(),
        log_file.line_count(),
        log_file_indexing_status,
    );
    let right_segments = right_segments(
        last_keys,
        message_span,
        position_text,
        file_size_text(log_file.file_size()),
    );

    frame.buffer_mut().set_style(area, BAR_STYLE);
    frame.render_widget(Line::from(format!(" {}", log_file.file_name())), area);
    frame.render_widget(right_line(right_segments), area);
}

fn right_segments<'a>(
    last_keys: &'a str,
    message_span: Option<Span<'a>>,
    position_text: String,
    file_size_text: String,
) -> Vec<Span<'a>> {
    let mut segments = Vec::new();
    if !last_keys.is_empty() {
        segments.push(Span::raw(last_keys));
    }
    segments.extend(message_span);
    segments.push(Span::raw(position_text));
    segments.push(Span::raw(file_size_text));
    segments
}

fn right_line(segments: Vec<Span<'_>>) -> Line<'_> {
    let mut right_line = Line::default();
    for (segment_index, segment) in segments.into_iter().enumerate() {
        if segment_index > 0 {
            right_line.push_span(SEGMENT_SEPARATOR.dim());
        }
        right_line.push_span(segment);
    }

    right_line.push_span(" ");
    right_line.right_aligned()
}

fn message_span(
    status_message: Option<&StatusMessage>,
    log_file_indexing_status: LogFileIndexingStatus,
    percent_indexed: u64,
) -> Option<Span<'_>> {
    match status_message {
        Some(StatusMessage::Error(message_text)) => Some(message_text.as_str().red()),
        None => indexing_span(log_file_indexing_status, percent_indexed),
    }
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

fn indexing_span(
    log_file_indexing_status: LogFileIndexingStatus,
    percent_indexed: u64,
) -> Option<Span<'static>> {
    match log_file_indexing_status {
        LogFileIndexingStatus::Running => Some(format!("indexing {percent_indexed}%").yellow()),
        LogFileIndexingStatus::Failed => Some("indexing failed".red()),
        LogFileIndexingStatus::Done => None,
    }
}

fn file_size_text(file_size: u64) -> String {
    if file_size < 1000 {
        return format!("{file_size} B");
    }

    let mut size_in_unit = file_size as f64 / 1000.0;
    let mut unit = FILE_SIZE_UNITS[0];
    for larger_unit in &FILE_SIZE_UNITS[1..] {
        if rounds_below_1000(size_in_unit) {
            break;
        }
        size_in_unit /= 1000.0;
        unit = larger_unit;
    }

    format!("{size_in_unit:.1} {unit}")
}

fn rounds_below_1000(size_in_unit: f64) -> bool {
    (size_in_unit * 10.0).round() < 10_000.0
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, style::Modifier};

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_status(text: &str, cursor_line: usize, last_keys: &str) -> String {
        rendered_rows(&render_indexed_status_buffer(text, cursor_line, last_keys)).remove(0)
    }

    fn render_indexed_status_buffer(text: &str, cursor_line: usize, last_keys: &str) -> Buffer {
        let log_file = LogFile::from_text(text);
        let mut log_view_state = LogViewState::default();
        log_view_state.move_cursor_down(cursor_line, log_file.line_count());
        render_status_buffer(
            &log_file,
            LogFileIndexingStatus::Done,
            &log_view_state,
            last_keys,
            None,
        )
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
        render_to_buffer(50, 1, |frame| {
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
    fn given_error_message_when_rendering_then_it_takes_the_message_slot_in_red() {
        let log_file = LogFile::from_text("a\nb\n");
        let error_message = StatusMessage::Error("Not a command: x".to_owned());

        let buffer = render_status_buffer(
            &log_file,
            LogFileIndexingStatus::Done,
            &LogViewState::default(),
            "",
            Some(&error_message),
        );

        assert_eq!(
            rendered_rows(&buffer),
            [" app.log       Not a command: x │ 1/2 (50%) │ 4 B"]
        );
        assert_eq!(buffer[(15, 0)].fg, Color::Red);
        assert_ne!(buffer[(1, 0)].fg, Color::Red);
        assert_ne!(buffer[(48, 0)].fg, Color::Red);
    }

    #[test]
    fn given_error_message_while_indexing_when_rendering_then_it_replaces_the_indexing_progress() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");
        let error_message = StatusMessage::Error("Not a command: x".to_owned());

        let buffer = render_status_buffer(
            &log_file,
            LogFileIndexingStatus::Running,
            &LogViewState::default(),
            "",
            Some(&error_message),
        );

        assert_eq!(
            rendered_rows(&buffer),
            [" app.log            Not a command: x │ 0/0+ │ 4 B"]
        );
    }

    #[test]
    fn given_cursor_on_second_of_four_lines_when_rendering_then_name_is_left_and_position_and_size_are_right()
     {
        assert_eq!(
            render_status("a\nb\nc\nd\n", 1, ""),
            " app.log                          2/4 (50%) │ 8 B"
        );
    }

    #[test]
    fn given_status_bar_when_rendering_then_the_row_is_tinted_and_separators_dim() {
        let buffer = render_indexed_status_buffer("a\nb\nc\nd\n", 1, "");

        assert_eq!(buffer[(0, 0)].bg, Color::DarkGray);
        assert_eq!(buffer[(20, 0)].bg, Color::DarkGray);
        assert_eq!(buffer[(49, 0)].bg, Color::DarkGray);
        assert_eq!(buffer[(44, 0)].symbol(), "│");
        assert!(buffer[(44, 0)].modifier.contains(Modifier::DIM));
    }

    #[test]
    fn given_empty_file_when_rendering_then_position_is_zero_of_zero() {
        assert_eq!(
            render_status("", 0, ""),
            " app.log                                0/0 │ 0 B"
        );
    }

    #[test]
    fn given_last_keys_when_rendering_then_a_separator_sets_them_apart_from_the_position() {
        assert_eq!(
            render_status("a\nb\nc\nd\n", 1, "5j"),
            " app.log                     5j │ 2/4 (50%) │ 8 B"
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
    fn given_file_still_indexing_when_rendering_then_progress_comes_before_the_total_with_a_plus() {
        let log_file = LogFile::unindexed_from_text("a\nb\n");

        assert_eq!(
            render_status_with_indexing(&log_file, LogFileIndexingStatus::Running),
            " app.log                 indexing 0% │ 0/0+ │ 4 B"
        );
    }

    #[test]
    fn given_indexing_failed_when_rendering_then_failure_is_shown_and_total_has_a_plus() {
        let log_file = LogFile::from_text("a\nb\nc\nd\n");

        assert_eq!(
            render_status_with_indexing(&log_file, LogFileIndexingStatus::Failed),
            " app.log             indexing failed │ 1/4+ │ 8 B"
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
    fn given_indexing_running_when_building_progress_then_percent_is_shown_in_yellow() {
        assert_eq!(
            indexing_span(LogFileIndexingStatus::Running, 45),
            Some("indexing 45%".yellow())
        );
    }

    #[test]
    fn given_indexing_failed_when_building_progress_then_failure_is_shown_in_red() {
        assert_eq!(
            indexing_span(LogFileIndexingStatus::Failed, 45),
            Some("indexing failed".red())
        );
    }

    #[test]
    fn given_indexing_done_when_building_progress_then_nothing_is_shown() {
        assert_eq!(indexing_span(LogFileIndexingStatus::Done, 100), None);
    }

    #[test]
    fn given_size_below_1000_bytes_when_formatting_size_then_it_is_whole_bytes() {
        assert_eq!(file_size_text(0), "0 B");
        assert_eq!(file_size_text(999), "999 B");
    }

    #[test]
    fn given_size_from_1000_bytes_when_formatting_size_then_it_has_one_decimal_in_a_decimal_unit() {
        assert_eq!(file_size_text(1000), "1.0 kB");
        assert_eq!(file_size_text(20_061), "20.1 kB");
        assert_eq!(file_size_text(317_837_192), "317.8 MB");
        assert_eq!(file_size_text(3_178_371_920), "3.2 GB");
    }

    #[test]
    fn given_size_that_rounds_up_to_1000_when_formatting_size_then_the_next_unit_is_used() {
        assert_eq!(file_size_text(999_949), "999.9 kB");
        assert_eq!(file_size_text(999_999), "1.0 MB");
    }

    #[test]
    fn given_size_beyond_the_largest_unit_when_formatting_size_then_it_stays_in_that_unit() {
        assert_eq!(file_size_text(5_000_000_000_000_000), "5000.0 TB");
    }
}
