use ratatui::{Frame, layout::Rect, text::Line};

use crate::domain::{
    log_file::{Indexing, LogFile},
    tab::Tab,
};

pub fn render(frame: &mut Frame, area: Rect, log_file: &LogFile, tab: &Tab, last_keys: &str) {
    let indexing = log_file.indexing();
    let position = position_text(tab.cursor(), log_file.line_count(), indexing);
    let indexing_progress = indexing_text(indexing, log_file.percent_indexed());
    let right_text = right_text(last_keys, position, indexing_progress);
    frame.render_widget(Line::raw(log_file.file_name()), area);
    frame.render_widget(Line::raw(right_text).right_aligned(), area);
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
    use super::*;
    use crate::{
        domain::session::Session,
        ui::tests::{render_to_buffer, rendered_rows},
    };

    fn render_status(text: &str, cursor: usize, last_keys: &str) -> String {
        let mut session = Session::from_text(text);
        session.tab.move_down(cursor, session.log_file.line_count());
        render_session_status(&session, last_keys)
    }

    fn render_session_status(session: &Session, last_keys: &str) -> String {
        let buffer = render_to_buffer(30, 1, |frame| {
            render(
                frame,
                frame.area(),
                &session.log_file,
                &session.tab,
                last_keys,
            );
        });
        rendered_rows(&buffer).remove(0)
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
        let session = Session::unindexed_from_text("a\nb\n");

        assert_eq!(
            render_session_status(&session, ""),
            "app.log     0/0+ | indexing 0%"
        );
    }

    #[test]
    fn given_indexing_failed_when_rendering_then_total_has_a_plus_and_failure_is_shown() {
        let mut session = Session::from_text("a\nb\nc\nd\n");
        session.log_file.set_indexing(Indexing::Failed);

        assert_eq!(
            render_session_status(&session, ""),
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
