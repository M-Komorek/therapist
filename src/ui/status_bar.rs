use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
};

use super::{BAR_STYLE, palette, slanted_edge};
use crate::{
    app::{Model, StatusMessage},
    domain::{log_file::IndexingStatus, search::SearchState, tabs::GrepState},
    jobs::JobSlot,
};

const SEGMENT_SEPARATOR: &str = " · ";
const PROGRESS_METER_WIDTH: usize = 10;
const ACCENT_SEGMENT_STYLE: Style = Style::new()
    .fg(palette::BACKGROUND_DARK)
    .bg(palette::YELLOW)
    .add_modifier(Modifier::BOLD);
const FILE_SIZE_SEGMENT_STYLE: Style = Style::new()
    .fg(palette::YELLOW)
    .bg(palette::BACKGROUND_HIGHLIGHT);
const FILE_SIZE_UNITS: [&str; 4] = ["kB", "MB", "GB", "TB"];

pub fn render(frame: &mut Frame, area: Rect, model: &Model) {
    let log_file = &model.log_file;
    let line_count = log_file.line_count();
    let active_tab = model.tabs.active_tab();
    let cursor_row = active_tab.log_view_state().cursor_row();
    let cursor_line = active_tab.cursor_line(line_count).unwrap_or_default();
    let is_grepping = model
        .job_board
        .has_job(JobSlot::Grepping(active_tab.tab_id()));
    let is_row_count_final = if active_tab.shows_all_lines() {
        log_file.indexing_status() == IndexingStatus::Done
    } else {
        !is_grepping
    };
    let last_keys = model.input_controller.last_keys_text();
    let message_span = message_span(
        model.status_message.as_ref(),
        log_file.indexing_status(),
        log_file.percent_indexed(),
    );
    let search_span = search_span(
        active_tab.search_state(),
        cursor_line,
        model
            .job_board
            .has_job(JobSlot::Searching(active_tab.tab_id())),
    );
    let percent_grepped = active_tab.grep_state().map(GrepState::percent_grepped);
    let grepping_span = grepping_span(percent_grepped, is_grepping);
    let position_text = position_text(
        cursor_row,
        active_tab.row_count(line_count),
        is_row_count_final,
    );
    let right_segments = right_segments(&last_keys, message_span, search_span, grepping_span);
    let left_line = left_line(&log_file.file_name(), file_size_text(log_file.file_size()));

    frame.buffer_mut().set_style(area, BAR_STYLE);
    frame.render_widget(left_line, area);
    frame.render_widget(right_line(right_segments, position_text), area);
}

fn left_line(file_name: &str, file_size_text: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {file_name} "), ACCENT_SEGMENT_STYLE),
        slanted_edge(palette::YELLOW, palette::BACKGROUND_HIGHLIGHT),
        Span::styled(format!(" {file_size_text} "), FILE_SIZE_SEGMENT_STYLE),
        slanted_edge(palette::BACKGROUND_HIGHLIGHT, palette::BACKGROUND_DARK),
    ])
}

fn right_segments<'a>(
    last_keys: &'a str,
    message_span: Option<Span<'a>>,
    search_span: Option<Span<'a>>,
    grepping_span: Option<Span<'a>>,
) -> Vec<Span<'a>> {
    let mut segments = Vec::new();
    if !last_keys.is_empty() {
        segments.push(last_keys.fg(palette::PURPLE));
    }
    segments.extend(message_span);
    segments.extend(search_span);
    segments.extend(grepping_span);
    segments
}

fn right_line(segments: Vec<Span<'_>>, position_text: String) -> Line<'_> {
    let mut right_line = Line::default();
    for (segment_index, segment) in segments.into_iter().enumerate() {
        if segment_index > 0 {
            right_line.push_span(SEGMENT_SEPARATOR.fg(palette::FOREGROUND_FAINT));
        }
        right_line.push_span(segment);
    }

    right_line.push_span(" ");
    right_line.push_span(slanted_edge(palette::BACKGROUND_DARK, palette::YELLOW));
    right_line.push_span(Span::styled(
        format!(" {position_text} "),
        ACCENT_SEGMENT_STYLE,
    ));
    right_line.right_aligned()
}

fn message_span(
    status_message: Option<&StatusMessage>,
    log_file_indexing_status: IndexingStatus,
    percent_indexed: u64,
) -> Option<Span<'_>> {
    match status_message {
        Some(StatusMessage::Info(message_text)) => Some(Span::raw(message_text.as_str())),
        Some(StatusMessage::Error(message_text)) => Some(message_text.as_str().fg(palette::RED)),
        None => indexing_span(log_file_indexing_status, percent_indexed),
    }
}

fn search_span(
    search_state: &SearchState,
    cursor_line: usize,
    is_searching: bool,
) -> Option<Span<'static>> {
    let search_query = search_state.query()?;
    if is_searching {
        return Some(
            progress_text("searching", search_state.percent_searched()).fg(palette::ORANGE),
        );
    }
    if search_state.match_count() == 0 {
        return Some(Span::raw(format!(
            "Pattern not found: {}",
            search_query.phrase
        )));
    }

    Some(Span::raw(format!(
        "{}{} [{}/{}]",
        search_query.direction.prefix(),
        search_query.phrase,
        search_state.match_count_at_or_above(cursor_line),
        search_state.match_count()
    )))
}

fn grepping_span(percent_grepped: Option<u64>, is_grepping: bool) -> Option<Span<'static>> {
    let percent_grepped = percent_grepped.filter(|_| is_grepping)?;
    Some(progress_text("grepping", percent_grepped).fg(palette::ORANGE))
}

fn progress_text(activity: &str, percent: u64) -> String {
    let filled_cells = percent.min(100) as usize * PROGRESS_METER_WIDTH / 100;
    let empty_cells = PROGRESS_METER_WIDTH - filled_cells;
    format!(
        "{activity} {}{} {percent}%",
        "━".repeat(filled_cells),
        "─".repeat(empty_cells)
    )
}

fn position_text(cursor_row: usize, row_count: usize, is_row_count_final: bool) -> String {
    let row_number = if row_count == 0 { 0 } else { cursor_row + 1 };
    let position = format!("{row_number}/{row_count}");
    match (is_row_count_final, row_count) {
        (false, _) => format!("{position}+"),
        (true, 0) => position,
        (true, _) => format!("{position} ({}%)", row_number * 100 / row_count),
    }
}

fn indexing_span(
    log_file_indexing_status: IndexingStatus,
    percent_indexed: u64,
) -> Option<Span<'static>> {
    match log_file_indexing_status {
        IndexingStatus::Running => {
            Some(progress_text("indexing", percent_indexed).fg(palette::ORANGE))
        }
        IndexingStatus::Failed => Some("indexing failed".fg(palette::RED)),
        IndexingStatus::Done => None,
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
    use ratatui::{buffer::Buffer, crossterm::event::KeyCode};

    use ratatui::layout::Size;

    use super::*;
    use crate::{
        domain::{
            log_file::LogFile,
            phrase_matching::MatchingLinesBatch,
            search::{SearchDirection, SearchQuery},
        },
        ui::{
            SLANTED_EDGE,
            tests::{render_to_buffer, rendered_rows},
        },
    };

    fn make_model(text: &str) -> Model {
        let mut model = make_unindexed_model(text);
        model.finish_scheduled_jobs();
        model
    }

    fn make_unindexed_model(text: &str) -> Model {
        Model::with_screen_size(LogFile::unindexed_from_text(text), Size::new(50, 3))
    }

    fn render_status_buffer(model: &Model) -> Buffer {
        render_to_buffer(50, 1, |frame| render(frame, frame.area(), model))
    }

    fn render_status(model: &Model) -> String {
        rendered_rows(&render_status_buffer(model)).remove(0)
    }

    fn make_search_state(
        phrase: &str,
        direction: SearchDirection,
        matching_lines: &[usize],
    ) -> SearchState {
        let mut search_state = SearchState::default();
        search_state.replace_query(SearchQuery {
            phrase: phrase.to_owned(),
            direction,
        });
        search_state.add_batch(MatchingLinesBatch {
            matching_lines: matching_lines.to_vec(),
            percent_scanned: 40,
        });
        search_state
    }

    #[test]
    fn given_no_search_when_formatting_the_search_then_nothing_is_shown() {
        assert_eq!(search_span(&SearchState::default(), 0, false), None);
    }

    #[test]
    fn given_search_running_when_formatting_the_search_then_its_percent_is_shown_in_orange() {
        let search_state = make_search_state("timeout", SearchDirection::Forward, &[2]);

        assert_eq!(
            search_span(&search_state, 0, true),
            Some("searching ━━━━────── 40%".fg(palette::ORANGE))
        );
    }

    #[test]
    fn given_search_without_matches_when_formatting_the_search_then_pattern_not_found_is_shown() {
        let search_state = make_search_state("zzzz", SearchDirection::Forward, &[]);

        assert_eq!(
            search_span(&search_state, 0, false),
            Some(Span::raw("Pattern not found: zzzz"))
        );
    }

    #[test]
    fn given_cursor_on_second_of_three_matches_when_formatting_the_search_then_its_position_is_shown()
     {
        let search_state = make_search_state("timeout", SearchDirection::Forward, &[2, 5, 9]);

        assert_eq!(
            search_span(&search_state, 5, false),
            Some(Span::raw("/timeout [2/3]"))
        );
    }

    #[test]
    fn given_backward_search_and_cursor_above_every_match_when_formatting_the_search_then_position_is_zero()
     {
        let search_state = make_search_state("WARN", SearchDirection::Backward, &[2, 5]);

        assert_eq!(
            search_span(&search_state, 0, false),
            Some(Span::raw("?WARN [0/2]"))
        );
    }

    #[test]
    fn given_error_message_when_rendering_then_it_takes_the_message_slot_in_red() {
        let mut model = make_model("a\nb\n");
        model.status_message = Some(StatusMessage::Error("Not a command: x".to_owned()));

        let buffer = render_status_buffer(&model);

        assert_eq!(
            rendered_rows(&buffer),
            [
                " app.log \u{1fb60}\u{1fb57} 4 B \u{1fb60}\u{1fb57}  Not a command: x \u{1fb60}\u{1fb57} 1/2 (50%)"
            ]
        );
        assert_eq!(buffer[(21, 0)].fg, palette::RED);
        assert_ne!(buffer[(1, 0)].fg, palette::RED);
        assert_ne!(buffer[(48, 0)].fg, palette::RED);
    }

    #[test]
    fn given_info_message_when_rendering_then_it_takes_the_message_slot_in_the_bar_color() {
        let mut model = make_model("a\nb\n");
        model.status_message = Some(StatusMessage::Info("search hit BOTTOM".to_owned()));

        let buffer = render_status_buffer(&model);

        assert_eq!(
            rendered_rows(&buffer),
            [
                " app.log \u{1fb60}\u{1fb57} 4 B \u{1fb60}\u{1fb57} search hit BOTTOM \u{1fb60}\u{1fb57} 1/2 (50%)"
            ]
        );
        assert_eq!(buffer[(20, 0)].fg, palette::FOREGROUND_DARK);
    }

    #[test]
    fn given_error_message_while_indexing_when_rendering_then_it_replaces_the_indexing_progress() {
        let mut model = make_unindexed_model("a\nb\n");
        model.status_message = Some(StatusMessage::Error("Not a command: x".to_owned()));

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 4 B \u{1fb60}\u{1fb57}       Not a command: x \u{1fb60}\u{1fb57} 0/0+"
        );
    }

    #[test]
    fn given_cursor_on_second_of_four_lines_when_rendering_then_name_and_size_are_left_and_position_is_right()
     {
        let mut model = make_model("a\nb\nc\nd\n");
        model
            .tabs
            .active_tab_mut()
            .log_view_state_mut()
            .move_cursor_down(1, 4);

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 8 B \u{1fb60}\u{1fb57}                   \u{1fb60}\u{1fb57} 2/4 (50%)"
        );
    }

    #[test]
    fn given_status_bar_when_rendering_then_file_name_and_position_are_accent_segments_on_the_dark_bar()
     {
        let buffer = render_status_buffer(&make_model("a\nb\nc\nd\n"));

        let background_at = |column| buffer[(column, 0)].bg;
        let edge_at =
            |column| buffer[(column, 0)].symbol().to_owned() + buffer[(column + 1, 0)].symbol();
        assert_eq!(background_at(1), palette::YELLOW);
        assert!(buffer[(1, 0)].modifier.contains(Modifier::BOLD));
        assert_eq!(edge_at(9), SLANTED_EDGE);
        assert_eq!(background_at(11), palette::BACKGROUND_HIGHLIGHT);
        assert_eq!(background_at(20), palette::BACKGROUND_DARK);
        assert_eq!(edge_at(37), SLANTED_EDGE);
        assert_eq!(background_at(40), palette::YELLOW);
        assert_eq!(background_at(49), palette::YELLOW);
    }

    #[test]
    fn given_last_keys_and_a_message_when_rendering_then_the_keys_are_purple_and_a_faint_separator_follows()
     {
        let mut model = make_model("a\nb\nc\nd\n");
        model.type_keys("2j");
        model.status_message = Some(StatusMessage::Info("hello".to_owned()));

        let buffer = render_status_buffer(&model);

        assert_eq!(
            rendered_rows(&buffer),
            [
                " app.log \u{1fb60}\u{1fb57} 8 B \u{1fb60}\u{1fb57}        2j · hello \u{1fb60}\u{1fb57} 3/4 (75%)"
            ]
        );
        assert_eq!(buffer[(27, 0)].fg, palette::PURPLE);
        assert_eq!(buffer[(30, 0)].fg, palette::FOREGROUND_FAINT);
        assert_eq!(buffer[(32, 0)].fg, palette::FOREGROUND_DARK);
    }

    #[test]
    fn given_empty_file_when_rendering_then_position_is_zero_of_zero() {
        assert_eq!(
            render_status(&make_model("")),
            " app.log \u{1fb60}\u{1fb57} 0 B \u{1fb60}\u{1fb57}                         \u{1fb60}\u{1fb57} 0/0"
        );
    }

    #[test]
    fn given_last_keys_when_rendering_then_a_separator_sets_them_apart_from_the_position() {
        let mut model = make_model("a\nb\nc\nd\n");

        model.type_keys("2j");

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 8 B \u{1fb60}\u{1fb57}                2j \u{1fb60}\u{1fb57} 3/4 (75%)"
        );
    }

    #[test]
    fn given_large_file_when_formatting_position_then_percent_is_rounded_down() {
        assert_eq!(position_text(1203, 88213, true), "1204/88213 (1%)");
    }

    #[test]
    fn given_file_still_indexing_when_rendering_then_progress_comes_before_the_total_with_a_plus() {
        assert_eq!(
            render_status(&make_unindexed_model("a\nb\n")),
            " app.log \u{1fb60}\u{1fb57} 4 B \u{1fb60}\u{1fb57} indexing ────────── 0% \u{1fb60}\u{1fb57} 0/0+"
        );
    }

    #[test]
    fn given_indexing_failed_when_rendering_then_failure_is_shown_and_total_has_a_plus() {
        let mut model = make_model("a\nb\nc\nd\n");
        model.log_file.mark_indexing_failed();

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 8 B \u{1fb60}\u{1fb57}        indexing failed \u{1fb60}\u{1fb57} 1/4+"
        );
    }

    #[test]
    fn given_row_count_not_final_when_formatting_position_then_there_is_no_percent() {
        assert_eq!(position_text(1203, 88213, false), "1204/88213+");
    }

    #[test]
    fn given_grep_running_when_building_its_span_then_its_percent_is_shown_in_orange() {
        assert_eq!(
            grepping_span(Some(62), true),
            Some("grepping ━━━━━━──── 62%".fg(palette::ORANGE))
        );
    }

    #[test]
    fn given_percents_when_building_progress_text_then_every_full_ten_percent_fills_one_cell() {
        assert_eq!(progress_text("grepping", 0), "grepping ────────── 0%");
        assert_eq!(progress_text("grepping", 9), "grepping ────────── 9%");
        assert_eq!(progress_text("grepping", 62), "grepping ━━━━━━──── 62%");
        assert_eq!(progress_text("grepping", 100), "grepping ━━━━━━━━━━ 100%");
    }

    #[test]
    fn given_percent_above_100_when_building_progress_text_then_the_meter_stays_full() {
        assert_eq!(progress_text("grepping", 150), "grepping ━━━━━━━━━━ 150%");
    }

    #[test]
    fn given_grep_done_or_all_lines_tab_when_building_the_grep_span_then_nothing_is_shown() {
        assert_eq!(grepping_span(Some(100), false), None);
        assert_eq!(grepping_span(None, false), None);
    }

    #[test]
    fn given_running_grep_tab_when_rendering_then_the_progress_and_its_rows_so_far_are_shown() {
        let mut model = make_model("ok\nERROR a\nok\nERROR b\n");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);
        let mut job_notifications = model.job_board.run_scheduled_jobs();
        let _done_notification = job_notifications.pop();
        model.handle_job_notifications(job_notifications);

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 22 B grepping ━━━━━━━━━━ 100% \u{1fb60}\u{1fb57} 1/2+"
        );
    }

    #[test]
    fn given_finished_grep_tab_when_rendering_then_the_position_counts_its_rows() {
        let mut model = make_model("ok\nERROR a\nok\nERROR b\n");
        model.type_keys("&ERROR");
        model.press(KeyCode::Enter);
        model.finish_scheduled_jobs();
        model.type_keys("j");

        assert_eq!(
            render_status(&model),
            " app.log \u{1fb60}\u{1fb57} 22 B \u{1fb60}\u{1fb57}               j \u{1fb60}\u{1fb57} 2/2 (100%)"
        );
    }

    #[test]
    fn given_indexing_running_when_building_progress_then_percent_is_shown_in_orange() {
        assert_eq!(
            indexing_span(IndexingStatus::Running, 45),
            Some("indexing ━━━━────── 45%".fg(palette::ORANGE))
        );
    }

    #[test]
    fn given_indexing_failed_when_building_progress_then_failure_is_shown_in_red() {
        assert_eq!(
            indexing_span(IndexingStatus::Failed, 45),
            Some("indexing failed".fg(palette::RED))
        );
    }

    #[test]
    fn given_indexing_done_when_building_progress_then_nothing_is_shown() {
        assert_eq!(indexing_span(IndexingStatus::Done, 100), None);
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
