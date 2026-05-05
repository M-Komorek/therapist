use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::contract::Component;
use crate::action::Action;

pub struct StatusBar {
    file_path: String,
    current_line: usize,
    total_lines: usize,
    indexer_bytes: usize,
}

impl StatusBar {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            current_line: 0,
            total_lines: 0,
            indexer_bytes: 0,
        }
    }

    fn format_bytes(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = 1024 * KB;

        if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn left_segment(&self) -> String {
        self.file_path.clone()
    }

    fn right_segment(&self) -> String {
        let position = format!("{}/{}", self.current_line + 1, self.total_lines);
        let memory = Self::format_bytes(self.indexer_bytes);
        format!("{} | {}", position, memory)
    }
}

impl Component for StatusBar {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if let Action::StatusUpdate {
            current_line,
            total_lines,
            memory_consumption: indexer_bytes,
        } = action
        {
            self.current_line = current_line;
            self.total_lines = total_lines;
            self.indexer_bytes = indexer_bytes;
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let left = self.left_segment();
        let right = self.right_segment();

        let available_width = area.width as usize;
        let right_len = right.chars().count();
        let left_max = available_width.saturating_sub(right_len);

        let left_truncated: String = left.chars().take(left_max).collect();
        let padding = available_width
            .saturating_sub(left_truncated.chars().count())
            .saturating_sub(right_len);

        let bar_style = Style::default().bg(Color::DarkGray).fg(Color::White);

        let line = Line::from(vec![
            Span::styled(left_truncated, bar_style),
            Span::styled(" ".repeat(padding), bar_style),
            Span::styled(right, bar_style),
        ]);

        frame.render_widget(Paragraph::new(line).style(bar_style), area);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::StatusBar;

    #[test]
    fn given_bytes_below_kb_when_formatted_then_shows_bytes() {
        assert_eq!(StatusBar::format_bytes(512), "512 B");
    }

    #[test]
    fn given_bytes_at_kb_boundary_when_formatted_then_shows_kb() {
        assert_eq!(StatusBar::format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn given_bytes_above_mb_when_formatted_then_shows_mb() {
        assert_eq!(StatusBar::format_bytes(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn given_initial_state_when_left_segment_then_shows_file_path() {
        let bar = StatusBar::new("/var/log/app.log".to_string());
        assert_eq!(bar.left_segment(), "/var/log/app.log");
    }

    #[test]
    fn given_initial_state_when_right_segment_then_shows_one_of_zero() {
        let bar = StatusBar::new("/var/log/app.log".to_string());
        assert_eq!(bar.right_segment(), "1/0 | 0 B");
    }

    #[test]
    fn given_status_update_when_right_segment_then_shows_updated_position() {
        use crate::action::Action;
        use crate::ui::components::contract::Component;

        let mut bar = StatusBar::new("/var/log/app.log".to_string());
        bar.update(Action::StatusUpdate {
            current_line: 9,
            total_lines: 100,
            memory_consumption: 2048,
        })
        .unwrap();
        assert_eq!(bar.right_segment(), "10/100 | 2.0 KB");
    }
}
