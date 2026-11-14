use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};

use super::palette;
use crate::control::{KEY_BINDING_GROUPS, KeyBinding, KeyBindingGroup, key_sequence_text};

const POPUP_MARGIN: Margin = Margin::new(2, 1);
const POPUP_PADDING: Padding = Padding::symmetric(2, 1);
const BORDER_THICKNESS: u16 = 1;
const COLUMN_GAP: u16 = 5;
const ENTRY_INDENT: &str = " ";
const TYPED_TEXT_GAP: usize = 2;
const ELLIPSIS: char = '…';
const RULE_SYMBOL: &str = "─";
const COUNT_PLACEHOLDER: &str = "[count]";
const POPUP_TITLE: &str = " Keys ";
const POPUP_STYLE: Style = Style::new()
    .fg(palette::FOREGROUND)
    .bg(palette::BACKGROUND_DARK);
const BORDER_STYLE: Style = Style::new().fg(palette::FOREGROUND_FAINT);
const POPUP_TITLE_STYLE: Style = Style::new()
    .fg(palette::BACKGROUND_DARK)
    .bg(palette::YELLOW)
    .add_modifier(Modifier::BOLD);
const TITLE_STYLE: Style = Style::new()
    .fg(palette::YELLOW)
    .add_modifier(Modifier::BOLD);
const TYPED_TEXT_STYLE: Style = Style::new().fg(palette::PURPLE);
const KEY_NAME_BRACKET_STYLE: Style = Style::new().fg(palette::FOREGROUND_FAINT);
const COUNT_PLACEHOLDER_STYLE: Style = Style::new().fg(palette::FOREGROUND_MUTED);
const CLOSING_HINT_STYLE: Style = Style::new().fg(palette::FOREGROUND_MUTED);
const RULE_STYLE: Style = Style::new().fg(palette::FOREGROUND_FAINT);

struct HelpGroup {
    title: &'static str,
    entries: Vec<HelpEntry>,
}

struct HelpEntry {
    key_names: Vec<String>,
    description: &'static str,
}

impl HelpEntry {
    fn key_names_width(&self) -> usize {
        self.key_names
            .iter()
            .map(|key_name| key_name.chars().count())
            .sum()
    }
}

pub fn render(frame: &mut Frame, log_view_area: Rect) {
    let help_groups = help_groups();
    let (left_groups, right_groups) = help_groups.split_at(balanced_split_position(&help_groups));

    let popup_area = popup_area(log_view_area, left_groups, right_groups);
    let popup_block = popup_block();
    let content_area = popup_block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup_block, popup_area);

    render_key_columns(frame, content_area, left_groups, right_groups);
}

fn popup_area(log_view_area: Rect, left_groups: &[HelpGroup], right_groups: &[HelpGroup]) -> Rect {
    let column_width = column_width(left_groups).max(column_width(right_groups));
    let content_width = 2 * cell_count(column_width) + COLUMN_GAP;
    let content_height = cell_count(key_row_count(left_groups, right_groups));
    let popup_width =
        content_width + 2 * BORDER_THICKNESS + POPUP_PADDING.left + POPUP_PADDING.right;
    let popup_height =
        content_height + 2 * BORDER_THICKNESS + POPUP_PADDING.top + POPUP_PADDING.bottom;
    log_view_area.inner(POPUP_MARGIN).centered(
        Constraint::Length(popup_width),
        Constraint::Length(popup_height),
    )
}

fn cell_count(length: usize) -> u16 {
    u16::try_from(length).unwrap_or(u16::MAX)
}

fn popup_block() -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(BORDER_STYLE)
        .style(POPUP_STYLE)
        .padding(POPUP_PADDING)
        .title_top(Line::styled(POPUP_TITLE, POPUP_TITLE_STYLE))
        .title_bottom(Line::styled(" Esc close ", CLOSING_HINT_STYLE).right_aligned())
}

fn help_groups() -> Vec<HelpGroup> {
    KEY_BINDING_GROUPS
        .iter()
        .map(key_binding_help_group)
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
    HelpEntry {
        key_names: key_binding
            .key_sequence
            .iter()
            .map(|key| key_sequence_text(std::slice::from_ref(key)))
            .collect(),
        description: key_binding.description,
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

fn column_width(help_groups: &[HelpGroup]) -> usize {
    let entry_prefix_width = ENTRY_INDENT.len() + typed_text_width(help_groups);
    let title_widths = help_groups
        .iter()
        .map(|help_group| help_group.title.chars().count());
    let entry_widths = help_groups
        .iter()
        .flat_map(|help_group| &help_group.entries)
        .map(|help_entry| entry_prefix_width + help_entry.description.chars().count());
    title_widths.chain(entry_widths).max().unwrap_or(0)
}

fn key_row_count(left_groups: &[HelpGroup], right_groups: &[HelpGroup]) -> usize {
    column_row_count(left_groups).max(column_row_count(right_groups))
}

fn render_key_columns(
    frame: &mut Frame,
    key_area: Rect,
    left_groups: &[HelpGroup],
    right_groups: &[HelpGroup],
) {
    let [left_column, right_column] = Layout::horizontal([Constraint::Fill(1); 2])
        .spacing(COLUMN_GAP)
        .areas(key_area);
    render_column(frame, left_column, left_groups);
    render_column(frame, right_column, right_groups);
    render_column_divider(
        frame,
        column_divider_area(key_area, left_column, left_groups, right_groups),
    );
}

fn column_divider_area(
    key_area: Rect,
    left_column: Rect,
    left_groups: &[HelpGroup],
    right_groups: &[HelpGroup],
) -> Rect {
    Rect {
        x: left_column.right().saturating_add(COLUMN_GAP / 2),
        width: 1,
        height: cell_count(key_row_count(left_groups, right_groups)),
        ..key_area
    }
    .intersection(key_area)
}

fn render_column_divider(frame: &mut Frame, divider_area: Rect) {
    let divider = Block::new().borders(Borders::LEFT).border_style(RULE_STYLE);
    frame.render_widget(divider, divider_area);
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
        lines.push(group_title_line(help_group.title, column_width));
        lines.extend(
            help_group
                .entries
                .iter()
                .map(|help_entry| entry_line(help_entry, typed_text_width, description_width)),
        );
    }
    lines
}

fn group_title_line(title: &'static str, line_width: usize) -> Line<'static> {
    let rule_width = line_width.saturating_sub(title.chars().count() + 1);
    Line::from(vec![
        Span::styled(title, TITLE_STYLE),
        Span::raw(" "),
        Span::styled(RULE_SYMBOL.repeat(rule_width), RULE_STYLE),
    ])
}

fn typed_text_width(help_groups: &[HelpGroup]) -> usize {
    let longest_typed_text_width = help_groups
        .iter()
        .flat_map(|help_group| &help_group.entries)
        .map(HelpEntry::key_names_width)
        .max()
        .unwrap_or(0);
    longest_typed_text_width + TYPED_TEXT_GAP
}

fn entry_line(
    help_entry: &HelpEntry,
    typed_text_width: usize,
    description_width: usize,
) -> Line<'static> {
    let padding_width = typed_text_width.saturating_sub(help_entry.key_names_width());
    let mut spans = vec![Span::raw(ENTRY_INDENT)];
    spans.extend(
        help_entry
            .key_names
            .iter()
            .flat_map(|key_name| key_name_spans(key_name)),
    );
    spans.push(Span::raw(" ".repeat(padding_width)));
    spans.extend(description_spans(&text_fitted_to_width(
        help_entry.description,
        description_width,
    )));
    Line::from(spans)
}

fn key_name_spans(key_name: &str) -> Vec<Span<'static>> {
    match key_name
        .strip_prefix('<')
        .and_then(|bracketed_name| bracketed_name.strip_suffix('>'))
    {
        Some(bare_key_name) => vec![
            Span::styled("<", KEY_NAME_BRACKET_STYLE),
            Span::styled(bare_key_name.to_owned(), TYPED_TEXT_STYLE),
            Span::styled(">", KEY_NAME_BRACKET_STYLE),
        ],
        None => vec![Span::styled(key_name.to_owned(), TYPED_TEXT_STYLE)],
    }
}

fn description_spans(description: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (position, description_part) in description.split(COUNT_PLACEHOLDER).enumerate() {
        if position > 0 {
            spans.push(Span::styled(COUNT_PLACEHOLDER, COUNT_PLACEHOLDER_STYLE));
        }
        spans.push(Span::raw(description_part.to_owned()));
    }
    spans
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
                    key_names: vec!["x".to_owned()],
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
    fn given_groups_when_measuring_the_column_width_then_it_fits_the_widest_entry() {
        assert_eq!(column_width(&make_help_groups(&[2, 1])), 1 + (1 + 2) + 6);
    }

    #[test]
    fn given_groups_without_entries_when_measuring_the_column_width_then_it_fits_the_title() {
        assert_eq!(column_width(&make_help_groups(&[0])), "Group".len());
        assert_eq!(column_width(&[]), 0);
    }

    #[test]
    fn given_wide_area_when_rendering_then_the_groups_sit_in_two_columns() {
        let rows = render_help(100, 28);

        assert!(rows[1].starts_with("  ╭ Keys ─"));
        assert_eq!(rows[2].trim_matches(['│', ' ']), "");
        assert!(rows[3].contains("Moving") && rows[3].contains("Bookmarks"));
        assert!(rows[4].contains(" j         Move the cursor down"));
        assert!(rows[26].ends_with("─ Esc close ╯"));
    }

    #[test]
    fn given_area_bigger_than_the_content_when_rendering_then_the_popup_fits_the_content_and_is_centered()
     {
        let rows = render_help(160, 40);

        let popup_rows = 7..=33;
        let popup_left_edge = " ".repeat(22);
        assert!(rows[7].starts_with(&format!("{popup_left_edge}╭ Keys ─")));
        assert!(rows[33].ends_with("─ Esc close ╯"));
        assert_eq!(rows[33].chars().count(), 22 + 117);
        for (row_index, row) in rows.iter().enumerate() {
            assert_eq!(row.is_empty(), !popup_rows.contains(&row_index));
        }
    }

    #[test]
    fn given_narrow_area_when_rendering_then_long_descriptions_end_with_an_ellipsis() {
        let rows = render_help(60, 28);

        let bookmark_row = rows
            .iter()
            .find(|row| row.contains(" <Space>b "))
            .expect("the <Space>b row is shown");
        assert!(
            bookmark_row
                .trim_end_matches(['│', ' '])
                .ends_with(ELLIPSIS)
        );
    }

    #[test]
    fn given_wide_area_when_rendering_then_a_divider_runs_between_the_columns_beside_the_keys_only()
    {
        let rows = render_help(100, 28);

        let divider_column = rows[3]
            .char_indices()
            .filter(|&(_, character)| character == '│')
            .map(|(byte_position, _)| rows[3][..byte_position].chars().count())
            .nth(1)
            .expect("the divider is shown");
        let character_at_divider = |row: &String| row.chars().nth(divider_column);
        assert!(
            rows[3..=24]
                .iter()
                .all(|row| character_at_divider(row) == Some('│'))
        );
        assert_ne!(character_at_divider(&rows[25]), Some('│'));
    }

    #[test]
    fn given_title_shorter_than_the_line_when_building_the_title_line_then_a_rule_fills_the_rest() {
        assert_eq!(group_title_line("Tabs", 10).to_string(), "Tabs ─────");
        assert_eq!(group_title_line("Tabs", 5).to_string(), "Tabs ");
        assert_eq!(group_title_line("Tabs", 2).to_string(), "Tabs ");
    }

    #[test]
    fn given_named_key_when_building_its_spans_then_the_brackets_are_dimmed() {
        let spans = key_name_spans("<Ctrl-d>");

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].style, KEY_NAME_BRACKET_STYLE);
        assert_eq!(spans[1].content, "Ctrl-d");
        assert_eq!(spans[1].style, TYPED_TEXT_STYLE);
        assert_eq!(spans[2].style, KEY_NAME_BRACKET_STYLE);
    }

    #[test]
    fn given_character_key_when_building_its_spans_then_it_is_one_key_colored_span() {
        for key_name in ["gg", "<", ">"] {
            let spans = key_name_spans(key_name);

            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content, key_name);
            assert_eq!(spans[0].style, TYPED_TEXT_STYLE);
        }
    }

    #[test]
    fn given_description_with_a_count_when_building_its_spans_then_only_the_count_is_muted() {
        let spans = description_spans("Move down [count] lines");

        let muted_spans: Vec<_> = spans
            .iter()
            .filter(|span| span.style == COUNT_PLACEHOLDER_STYLE)
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(muted_spans, [COUNT_PLACEHOLDER]);
        assert_eq!(Line::from(spans).to_string(), "Move down [count] lines");
    }

    #[test]
    fn given_description_without_a_count_when_building_its_spans_then_it_is_plain() {
        let spans = description_spans("Quit");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style, Style::default());
    }

    #[test]
    fn given_tiny_area_when_rendering_then_it_does_not_panic() {
        for (width, height) in [(10, 3), (4, 2), (1, 1)] {
            render_help(width, height);
        }
    }
}
