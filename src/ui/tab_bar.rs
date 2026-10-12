use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::{BAR_STYLE, palette, slanted_edge};
use crate::domain::tabs::Tabs;

const WORDMARK: &str = " therapist ";
const WORDMARK_STYLE: Style = Style::new()
    .fg(palette::BACKGROUND_DARK)
    .bg(palette::YELLOW)
    .add_modifier(Modifier::BOLD);
const ACTIVE_TAB_BRACKET_STYLE: Style = Style::new()
    .fg(palette::YELLOW)
    .bg(palette::BACKGROUND_HIGHLIGHT);
const ACTIVE_TAB_NAME_STYLE: Style = ACTIVE_TAB_BRACKET_STYLE.add_modifier(Modifier::BOLD);
const INACTIVE_TAB_NAME_STYLE: Style = Style::new().fg(palette::FOREGROUND_DARK);
const INACTIVE_TAB_BRACKET_STYLE: Style = Style::new().fg(palette::FOREGROUND_FAINT);

pub fn render(frame: &mut Frame, area: Rect, tabs: &Tabs) {
    frame.buffer_mut().set_style(area, BAR_STYLE);
    frame.render_widget(tab_bar_line(tabs), area);
}

fn tab_bar_line(tabs: &Tabs) -> Line<'static> {
    let mut tab_bar_line = Line::from(Span::styled(WORDMARK, WORDMARK_STYLE));
    let mut left_background = palette::YELLOW;
    for (position, tab_state) in tabs.tab_states().iter().enumerate() {
        let is_active = position == tabs.active_position();
        let tab_background = if is_active {
            palette::BACKGROUND_HIGHLIGHT
        } else {
            palette::BACKGROUND_DARK
        };
        tab_bar_line.extend(edge_between(left_background, tab_background));
        tab_bar_line.extend(tab_label(&tab_state.name(), is_active));
        left_background = tab_background;
    }

    tab_bar_line.extend(edge_between(left_background, palette::BACKGROUND_DARK));
    tab_bar_line
}

fn edge_between(left_background: Color, right_background: Color) -> Option<Span<'static>> {
    (left_background != right_background).then(|| slanted_edge(left_background, right_background))
}

fn tab_label(name: &str, is_active: bool) -> [Span<'static>; 3] {
    let (name_style, bracket_style) = if is_active {
        (ACTIVE_TAB_NAME_STYLE, ACTIVE_TAB_BRACKET_STYLE)
    } else {
        (INACTIVE_TAB_NAME_STYLE, INACTIVE_TAB_BRACKET_STYLE)
    };
    [
        Span::styled(" [", bracket_style),
        Span::styled(name.to_owned(), name_style),
        Span::styled("] ", bracket_style),
    ]
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn render_tab_bar(tabs: &Tabs, width: u16) -> Buffer {
        render_to_buffer(width, 1, |frame| render(frame, frame.area(), tabs))
    }

    #[test]
    fn given_only_the_all_tab_when_rendering_then_the_wordmark_leads_and_the_tab_sits_on_the_highlight_background()
     {
        let buffer = render_tab_bar(&Tabs::default(), 24);

        assert_eq!(
            rendered_rows(&buffer),
            [" therapist \u{1fb60}\u{1fb57} [all] \u{1fb60}\u{1fb57}"]
        );
        assert_eq!(buffer[(1, 0)].bg, palette::YELLOW);
        assert_eq!(buffer[(11, 0)].fg, palette::YELLOW);
        assert_eq!(buffer[(11, 0)].bg, palette::BACKGROUND_HIGHLIGHT);
        assert_eq!(buffer[(13, 0)].bg, palette::BACKGROUND_HIGHLIGHT);
        assert_eq!(buffer[(21, 0)].bg, palette::BACKGROUND_DARK);
    }

    #[test]
    fn given_grep_tab_active_when_rendering_then_it_is_yellow_on_the_highlight_and_the_inactive_tab_is_on_the_bar()
     {
        let mut tabs = Tabs::default();
        tabs.open_grep_tab(vec!["ERROR".to_owned()]);

        let buffer = render_tab_bar(&tabs, 40);

        assert_eq!(
            rendered_rows(&buffer),
            [" therapist \u{1fb60}\u{1fb57} [all] \u{1fb60}\u{1fb57} [ERROR] \u{1fb60}\u{1fb57}"]
        );
        assert_eq!(buffer[(14, 0)].fg, palette::FOREGROUND_FAINT);
        assert_eq!(buffer[(15, 0)].fg, palette::FOREGROUND_DARK);
        assert_eq!(buffer[(15, 0)].bg, palette::BACKGROUND_DARK);
        assert_eq!(buffer[(23, 0)].fg, palette::YELLOW);
        assert_eq!(buffer[(24, 0)].fg, palette::YELLOW);
        assert_eq!(buffer[(24, 0)].bg, palette::BACKGROUND_HIGHLIGHT);
        assert!(buffer[(24, 0)].modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn given_inactive_tabs_next_to_each_other_when_rendering_then_no_edge_separates_them() {
        let mut tabs = Tabs::default();
        tabs.open_grep_tab(vec!["ERROR".to_owned()]);
        tabs.open_grep_tab(vec!["WARN".to_owned()]);

        tabs.switch_to_all_lines_tab();

        let buffer = render_tab_bar(&tabs, 40);
        assert_eq!(
            rendered_rows(&buffer),
            [" therapist \u{1fb60}\u{1fb57} [all] \u{1fb60}\u{1fb57} [ERROR]  [WARN]"]
        );
    }

    #[test]
    fn given_terminal_narrower_than_the_tabs_when_rendering_then_they_are_cut_at_the_edge() {
        let buffer = render_tab_bar(&Tabs::default(), 3);

        assert_eq!(rendered_rows(&buffer), [" th"]);
    }
}
