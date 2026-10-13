use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

use super::palette;
use crate::control::{
    COMMAND_DESCRIPTIONS, KEY_BINDING_GROUPS, KeyBinding, KeyBindingGroup, key_sequence_text,
};

const POPUP_MARGIN: Margin = Margin::new(2, 1);
const COLUMN_GAP: u16 = 4;
const ENTRY_INDENT: &str = " ";
const TYPED_TEXT_GAP: usize = 2;
const ELLIPSIS: char = '…';
const POPUP_STYLE: Style = Style::new()
    .fg(palette::FOREGROUND)
    .bg(palette::BACKGROUND_DARK);
const BORDER_STYLE: Style = Style::new().fg(palette::YELLOW);
const TITLE_STYLE: Style = Style::new()
    .fg(palette::YELLOW)
    .add_modifier(Modifier::BOLD);
const TYPED_TEXT_STYLE: Style = Style::new().fg(palette::PURPLE);
const CLOSING_HINT_STYLE: Style = Style::new().fg(palette::FOREGROUND_MUTED);

struct HelpGroup {
    title: &'static str,
    entries: Vec<HelpEntry>,
}

struct HelpEntry {
    typed_text: String,
    description: &'static str,
}

pub fn render(frame: &mut Frame, log_view_area: Rect) {
    let popup_area = log_view_area.inner(POPUP_MARGIN);
    let popup_block = popup_block();
    let content_area = popup_block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup_block, popup_area);

    let help_groups = help_groups();
    let (left_groups, right_groups) = help_groups.split_at(balanced_split_position(&help_groups));
    let [left_column, right_column] = Layout::horizontal([Constraint::Fill(1); 2])
        .spacing(COLUMN_GAP)
        .areas(content_area);
    render_column(frame, left_column, left_groups);
    render_column(frame, right_column, right_groups);
}

fn popup_block() -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(BORDER_STYLE)
        .style(POPUP_STYLE)
        .padding(Padding::horizontal(1))
        .title_top(Line::styled(" Keys ", TITLE_STYLE))
        .title_bottom(Line::styled(" Esc close ", CLOSING_HINT_STYLE).right_aligned())
}

fn help_groups() -> Vec<HelpGroup> {
    KEY_BINDING_GROUPS
        .iter()
        .map(key_binding_help_group)
        .chain([command_help_group()])
        .collect()
}

fn key_binding_help_group(key_binding_group: &KeyBindingGroup) -> HelpGroup {
    HelpGroup {
        title: key_binding_group.title,
        entries: key_binding_group
            .bindings
            .iter()
            .map(key_binding_help_entry)
            .collect(),
    }
}

fn key_binding_help_entry(key_binding: &KeyBinding) -> HelpEntry {
    let key_sequence_texts: Vec<String> = key_binding
        .key_sequences
        .iter()
        .map(|key_sequence| key_sequence_text(key_sequence))
        .collect();
    HelpEntry {
        typed_text: key_sequence_texts.join(" "),
        description: key_binding.description,
    }
}

fn command_help_group() -> HelpGroup {
    HelpGroup {
        title: "Commands",
        entries: COMMAND_DESCRIPTIONS
            .iter()
            .map(|&(command, description)| HelpEntry {
                typed_text: command.to_owned(),
                description,
            })
            .collect(),
    }
}

fn balanced_split_position(help_groups: &[HelpGroup]) -> usize {
    (0..=help_groups.len())
        .rev()
        .min_by_key(|&split_position| {
            let (left_groups, right_groups) = help_groups.split_at(split_position);
            column_row_count(left_groups).max(column_row_count(right_groups))
        })
        .unwrap_or(0)
}

fn column_row_count(help_groups: &[HelpGroup]) -> usize {
    let group_row_count: usize = help_groups
        .iter()
        .map(|help_group| 1 + help_group.entries.len())
        .sum();
    let blank_row_count = help_groups.len().saturating_sub(1);
    group_row_count + blank_row_count
}

fn render_column(frame: &mut Frame, column_area: Rect, help_groups: &[HelpGroup]) {
    let lines = column_lines(help_groups, usize::from(column_area.width));
    frame.render_widget(Paragraph::new(lines), column_area);
}

fn column_lines(help_groups: &[HelpGroup], column_width: usize) -> Vec<Line<'static>> {
    let typed_text_width = typed_text_width(help_groups);
    let description_width = column_width.saturating_sub(ENTRY_INDENT.len() + typed_text_width);
    let mut lines = Vec::new();

    for (position, help_group) in help_groups.iter().enumerate() {
        if position > 0 {
            lines.push(Line::default());
        }
        lines.push(Line::styled(help_group.title, TITLE_STYLE));
        lines.extend(
            help_group
                .entries
                .iter()
                .map(|help_entry| entry_line(help_entry, typed_text_width, description_width)),
        );
    }
    lines
}

fn typed_text_width(help_groups: &[HelpGroup]) -> usize {
    let longest_typed_text_width = help_groups
        .iter()
        .flat_map(|help_group| &help_group.entries)
        .map(|help_entry| help_entry.typed_text.chars().count())
        .max()
        .unwrap_or(0);
    longest_typed_text_width + TYPED_TEXT_GAP
}

fn entry_line(
    help_entry: &HelpEntry,
    typed_text_width: usize,
    description_width: usize,
) -> Line<'static> {
    Line::from(vec![
        Span::raw(ENTRY_INDENT),
        Span::styled(
            format!("{:<typed_text_width$}", help_entry.typed_text),
            TYPED_TEXT_STYLE,
        ),
        Span::raw(text_fitted_to_width(
            help_entry.description,
            description_width,
        )),
    ])
}

fn text_fitted_to_width(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let mut fitted_text: String = text.chars().take(width.saturating_sub(1)).collect();
    if width > 0 {
        fitted_text.push(ELLIPSIS);
    }
    fitted_text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tests::{render_to_buffer, rendered_rows};

    fn make_help_group(entry_count: usize) -> HelpGroup {
        HelpGroup {
            title: "Group",
            entries: (0..entry_count)
                .map(|_| HelpEntry {
                    typed_text: "x".to_owned(),
                    description: "does x",
                })
                .collect(),
        }
    }

    fn make_help_groups(entry_counts: &[usize]) -> Vec<HelpGroup> {
        entry_counts
            .iter()
            .map(|&entry_count| make_help_group(entry_count))
            .collect()
    }

    fn render_help(width: u16, height: u16) -> Vec<String> {
        rendered_rows(&render_to_buffer(width, height, |frame| {
            render(frame, frame.area())
        }))
    }

    #[test]
    fn given_groups_of_different_sizes_when_splitting_then_the_taller_column_is_as_short_as_possible()
     {
        let help_groups = make_help_groups(&[8, 5, 5, 3, 5, 4, 4]);

        assert_eq!(balanced_split_position(&help_groups), 3);
    }

    #[test]
    fn given_one_group_when_splitting_then_it_goes_in_the_left_column() {
        assert_eq!(balanced_split_position(&make_help_groups(&[3])), 1);
    }

    #[test]
    fn given_no_groups_when_splitting_then_both_columns_are_empty() {
        assert_eq!(balanced_split_position(&[]), 0);
    }

    #[test]
    fn given_groups_when_counting_column_rows_then_a_blank_row_separates_each_pair() {
        assert_eq!(column_row_count(&make_help_groups(&[2, 3])), 3 + 1 + 4);
        assert_eq!(column_row_count(&[]), 0);
    }

    #[test]
    fn given_text_that_fits_when_fitting_then_it_is_unchanged() {
        assert_eq!(text_fitted_to_width("Quit", 10), "Quit");
        assert_eq!(text_fitted_to_width("Quit", 4), "Quit");
    }

    #[test]
    fn given_text_too_long_when_fitting_then_it_is_cut_and_ends_with_an_ellipsis() {
        assert_eq!(text_fitted_to_width("Scroll a page up", 8), "Scroll …");
        assert_eq!(text_fitted_to_width("Quit", 1), "…");
        assert_eq!(text_fitted_to_width("Quit", 0), "");
    }

    #[test]
    fn given_wide_area_when_rendering_then_the_groups_sit_in_two_columns() {
        let rows = render_help(100, 28);

        assert!(rows[1].starts_with("  ╭ Keys ─"));
        assert!(rows[2].contains("Moving") && rows[2].contains("Bookmarks"));
        assert!(rows[3].contains(" j <Down>   Move the cursor down"));
        assert!(rows[26].ends_with("─ Esc close ╯"));
    }

    #[test]
    fn given_narrow_area_when_rendering_then_long_descriptions_end_with_an_ellipsis() {
        let rows = render_help(60, 28);

        let bookmark_row = rows
            .iter()
            .find(|row| row.contains(" bb "))
            .expect("the bb row is shown");
        assert!(
            bookmark_row
                .trim_end_matches(['│', ' '])
                .ends_with(ELLIPSIS)
        );
    }

    #[test]
    fn given_tiny_area_when_rendering_then_it_does_not_panic() {
        for (width, height) in [(10, 3), (4, 2), (1, 1)] {
            render_help(width, height);
        }
    }
}
