use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
};

use super::BAR_STYLE;
use crate::domain::tabs::Tabs;

pub fn render(frame: &mut Frame, area: Rect, tabs: &Tabs) {
    let mut tab_bar_line = Line::default();
    for (position, tab_state) in tabs.tab_states().iter().enumerate() {
        tab_bar_line.push_span(" ");
        tab_bar_line.push_span(tab_label(
            tab_state.name(),
            position == tabs.active_position(),
        ));
    }

    frame.buffer_mut().set_style(area, BAR_STYLE);
    frame.render_widget(tab_bar_line, area);
}

fn tab_label(name: &str, is_active: bool) -> Span<'static> {
    let tab_label = Span::raw(format!("[{name}]"));
    if is_active {
        tab_label.reversed()
    } else {
        tab_label
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        style::{Color, Modifier},
    };

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_tab_bar(tabs: &Tabs, width: u16) -> Buffer {
        render_to_buffer(width, 1, |frame| render(frame, frame.area(), tabs))
    }

    #[test]
    fn given_only_the_all_tab_when_rendering_then_it_shows_reversed_on_the_tinted_bar() {
        let buffer = render_tab_bar(&Tabs::default(), 20);

        assert_eq!(rendered_rows(&buffer), [" [all]"]);
        let is_reversed = |column| buffer[(column, 0)].modifier.contains(Modifier::REVERSED);
        assert!(is_reversed(1) && is_reversed(5));
        assert!(!is_reversed(0) && !is_reversed(6));
        assert_eq!(buffer[(19, 0)].bg, Color::DarkGray);
    }

    #[test]
    fn given_terminal_narrower_than_the_tabs_when_rendering_then_they_are_cut_at_the_edge() {
        let buffer = render_tab_bar(&Tabs::default(), 3);

        assert_eq!(rendered_rows(&buffer), [" [a"]);
    }
}
