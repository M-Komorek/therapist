use std::ops::{Range, RangeInclusive};

use ratatui::{buffer::CellWidth, style::Style, text::Span};

use super::palette;

const TAB_STOP: usize = 8;
const CONTROL_CHARACTER_STYLE: Style = Style::new().fg(palette::FOREGROUND_MUTED);
const CONTROL_SEQUENCE_INTRODUCER: &[u8] = b"\x1b[";
const CONTROL_SEQUENCE_PARAMETER_BYTES: RangeInclusive<u8> = 0x20..=0x3f;
const CONTROL_SEQUENCE_FINAL_BYTES: RangeInclusive<u8> = 0x40..=0x7e;

pub(super) struct MatchLayer {
    pub(super) match_ranges: Vec<Range<usize>>,
    pub(super) style: Style,
}

/// One log line as it appears in the visible columns. Tabs expand to the next tab stop, ANSI
/// escape sequences such as `ESC [31m` are hidden, other ASCII control characters show muted in
/// caret notation (`^[`), and other control characters are dropped. A wide character cut by the
/// left edge becomes spaces, so the rest of the line keeps its columns. Each match layer holds
/// byte ranges of the line in order. Text inside them takes the style of the first layer that
/// covers it. The line is cut on a side when some of its text is hidden past that edge.
pub(super) struct VisibleRow {
    pub(super) spans: Vec<Span<'static>>,
    pub(super) is_cut_on_the_left: bool,
    pub(super) is_cut_on_the_right: bool,
}

pub(super) fn visible_row(
    text: &str,
    visible_columns: Range<usize>,
    match_layers: &[MatchLayer],
) -> VisibleRow {
    let has_matches = match_layers
        .iter()
        .any(|match_layer| !match_layer.match_ranges.is_empty());
    if !has_matches && is_printable_ascii_through(text, visible_columns.end) {
        return ascii_visible_row(text, visible_columns);
    }
    let mut row_builder = VisibleRowBuilder::new(visible_columns, match_layers);
    let mut plain_text_start = 0;
    let controls = text
        .char_indices()
        .filter(|(_, character)| character.is_control());
    for (control_offset, control) in controls {
        row_builder.push_text(&text[plain_text_start..control_offset], plain_text_start);
        let control_length = match control_sequence_length(&text[control_offset..]) {
            Some(control_sequence_length) => control_sequence_length,
            None => {
                row_builder.push_control(control);
                control.len_utf8()
            }
        };
        plain_text_start = control_offset + control_length;
    }
    row_builder.push_text(&text[plain_text_start..], plain_text_start);
    row_builder.visible_row
}

fn is_printable_ascii_through(text: &str, last_byte_offset: usize) -> bool {
    let bytes_through_last = &text.as_bytes()[..last_byte_offset.saturating_add(1).min(text.len())];
    bytes_through_last
        .iter()
        .all(|byte| matches!(byte, b' '..=b'~'))
}

fn ascii_visible_row(text: &str, visible_columns: Range<usize>) -> VisibleRow {
    let visible_start = visible_columns.start.min(text.len());
    let visible_end = visible_columns.end.min(text.len());

    VisibleRow {
        spans: vec![Span::raw(text[visible_start..visible_end].to_owned())],
        is_cut_on_the_left: visible_columns.start > 0 && !text.is_empty(),
        is_cut_on_the_right: text.len() > visible_columns.end,
    }
}

fn control_sequence_length(text_from_control: &str) -> Option<usize> {
    let bytes_after_introducer = text_from_control
        .as_bytes()
        .strip_prefix(CONTROL_SEQUENCE_INTRODUCER)?;
    let final_byte_offset = bytes_after_introducer
        .iter()
        .position(|byte| !CONTROL_SEQUENCE_PARAMETER_BYTES.contains(byte))?;
    let final_byte = bytes_after_introducer[final_byte_offset];

    CONTROL_SEQUENCE_FINAL_BYTES
        .contains(&final_byte)
        .then_some(CONTROL_SEQUENCE_INTRODUCER.len() + final_byte_offset + 1)
}

struct VisibleRowBuilder<'a> {
    visible_columns: Range<usize>,
    match_layers: &'a [MatchLayer],
    match_ranges_ahead: Vec<&'a [Range<usize>]>,
    current_column: usize,
    visible_row: VisibleRow,
}

impl<'a> VisibleRowBuilder<'a> {
    fn new(visible_columns: Range<usize>, match_layers: &'a [MatchLayer]) -> Self {
        Self {
            visible_columns,
            match_layers,
            match_ranges_ahead: match_layers
                .iter()
                .map(|match_layer| match_layer.match_ranges.as_slice())
                .collect(),
            current_column: 0,
            visible_row: VisibleRow {
                spans: Vec::new(),
                is_cut_on_the_left: false,
                is_cut_on_the_right: false,
            },
        }
    }

    fn push_text(&mut self, text: &str, text_byte_offset: usize) {
        let span = Span::raw(text);
        let mut grapheme_byte_offset = text_byte_offset;

        for grapheme in span.styled_graphemes(Style::default()) {
            if self.is_full() {
                self.visible_row.is_cut_on_the_right = true;
                return;
            }
            let grapheme_width = usize::from(grapheme.symbol.cell_width());
            let style = self.style_at(grapheme_byte_offset);
            self.push_symbol(grapheme.symbol, grapheme_width, style);
            grapheme_byte_offset += grapheme.symbol.len();
        }
    }

    fn style_at(&mut self, byte_offset: usize) -> Style {
        let layers = self.match_layers.iter().zip(&mut self.match_ranges_ahead);

        for (match_layer, match_ranges_ahead) in layers {
            drop_passed_ranges(match_ranges_ahead, byte_offset);
            let next_range = match_ranges_ahead.first();
            if next_range.is_some_and(|match_range| match_range.contains(&byte_offset)) {
                return match_layer.style;
            }
        }
        Style::default()
    }

    fn push_control(&mut self, control: char) {
        if control == '\t' {
            let spaces_to_tab_stop = TAB_STOP - self.current_column % TAB_STOP;
            let tab_spaces = " ".repeat(spaces_to_tab_stop);
            self.push_symbol(&tab_spaces, spaces_to_tab_stop, Style::default());
        } else if control.is_ascii_control() {
            self.push_symbol(&caret_notation(control), 2, CONTROL_CHARACTER_STYLE);
        }
    }

    fn push_symbol(&mut self, symbol: &str, symbol_width: usize, style: Style) {
        if self.is_full() {
            self.visible_row.is_cut_on_the_right = true;
            return;
        }
        let next_column = self.current_column + symbol_width;
        if self.current_column < self.visible_columns.start {
            self.visible_row.is_cut_on_the_left = true;
        }
        if next_column > self.visible_columns.end {
            self.visible_row.is_cut_on_the_right = true;
        }

        if self.current_column >= self.visible_columns.start {
            self.append(symbol, style);
        } else if next_column > self.visible_columns.start {
            let visible_width = next_column - self.visible_columns.start;
            self.append(&" ".repeat(visible_width), Style::default());
        }
        self.current_column = next_column;
    }

    fn append(&mut self, text: &str, style: Style) {
        let spans = &mut self.visible_row.spans;
        match spans.last_mut() {
            Some(last_span) if last_span.style == style => {
                last_span.content.to_mut().push_str(text)
            }
            _ => spans.push(Span::styled(text.to_owned(), style)),
        }
    }

    fn is_full(&self) -> bool {
        self.current_column >= self.visible_columns.end
    }
}

fn drop_passed_ranges(match_ranges: &mut &[Range<usize>], byte_offset: usize) {
    while let Some((passed_range, later_ranges)) = match_ranges.split_first()
        && passed_range.end <= byte_offset
    {
        *match_ranges = later_ranges;
    }
}

fn caret_notation(control: char) -> String {
    format!("^{}", char::from(control as u8 ^ 0x40))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP_STYLE: Style = Style::new().bg(palette::YELLOW);
    const LOWER_STYLE: Style = Style::new().bg(palette::PURPLE);

    fn visible_spans(
        text: &str,
        visible_columns: Range<usize>,
        match_layers: &[MatchLayer],
    ) -> Vec<Span<'static>> {
        visible_row(text, visible_columns, match_layers).spans
    }

    fn cut_sides(text: &str, visible_columns: Range<usize>) -> (bool, bool) {
        let visible_row = visible_row(text, visible_columns, &[]);
        (
            visible_row.is_cut_on_the_left,
            visible_row.is_cut_on_the_right,
        )
    }

    #[test]
    fn given_line_longer_than_the_visible_columns_when_cutting_then_it_is_cut_on_the_right() {
        assert_eq!(cut_sides("abcdef", 0..4), (false, true));
        assert_eq!(cut_sides("ab\tcdef", 0..4), (false, true));
    }

    #[test]
    fn given_line_that_ends_at_the_right_edge_when_cutting_then_it_is_not_cut() {
        assert_eq!(cut_sides("abcd", 0..4), (false, false));
        assert_eq!(cut_sides("ab\tc", 0..10), (false, false));
    }

    #[test]
    fn given_view_scrolled_right_when_cutting_then_lines_with_text_left_of_it_are_cut_on_the_left()
    {
        assert_eq!(cut_sides("abcdef", 2..6), (true, false));
        assert_eq!(cut_sides("ab", 4..8), (true, false));
        assert_eq!(cut_sides("\x1bXY", 1..8), (true, false));
    }

    #[test]
    fn given_empty_line_or_only_escape_sequences_when_scrolled_then_nothing_is_cut() {
        assert_eq!(cut_sides("", 2..6), (false, false));
        assert_eq!(cut_sides("\x1b[0m", 2..6), (false, false));
    }

    #[test]
    fn given_color_reset_just_past_the_right_edge_when_cutting_then_it_is_not_cut() {
        assert_eq!(cut_sides("abcd\x1b[0m", 0..4), (false, false));
    }

    #[test]
    fn given_wide_character_across_the_right_edge_when_cutting_then_it_is_cut_on_the_right() {
        assert_eq!(cut_sides("a漢", 0..2), (false, true));
    }

    #[test]
    fn given_match_layers_when_cutting_then_the_cut_sides_are_the_same_as_without_them() {
        let text = "abcdef";

        let visible_row = visible_row(text, 2..4, &top_layer(text, "c"));

        assert!(visible_row.is_cut_on_the_left && visible_row.is_cut_on_the_right);
    }

    #[test]
    fn given_nul_and_del_when_converting_then_they_use_standard_caret_notation() {
        assert_eq!(caret_notation('\0'), "^@");
        assert_eq!(caret_notation('\x7f'), "^?");
    }

    fn match_layer(text: &str, phrase: &str, style: Style) -> MatchLayer {
        let match_ranges = text
            .match_indices(phrase)
            .map(|(match_start, _)| match_start..match_start + phrase.len())
            .collect();
        MatchLayer {
            match_ranges,
            style,
        }
    }

    fn top_layer(text: &str, phrase: &str) -> [MatchLayer; 1] {
        [match_layer(text, phrase, TOP_STYLE)]
    }

    fn span_texts(spans: &[Span]) -> Vec<String> {
        spans.iter().map(|span| span.content.to_string()).collect()
    }

    fn texts_in_style(spans: &[Span], style: Style) -> Vec<String> {
        spans
            .iter()
            .filter(|span| span.style == style)
            .map(|span| span.content.to_string())
            .collect()
    }

    #[test]
    fn given_ascii_line_with_two_matches_when_cutting_then_each_match_is_its_own_highlighted_span()
    {
        let text = "an ERROR, ERROR";

        let visible = visible_spans(text, 0..20, &top_layer(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["an ", "ERROR", ", ", "ERROR"]);
        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["ERROR", "ERROR"]);
    }

    #[test]
    fn given_match_cut_by_the_left_edge_when_scrolled_then_only_its_visible_part_is_highlighted() {
        let text = "an ERROR here";

        let visible = visible_spans(text, 5..13, &top_layer(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["ROR", " here"]);
        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["ROR"]);
    }

    #[test]
    fn given_match_past_the_right_edge_when_cutting_then_nothing_is_highlighted() {
        let text = "abc ERROR";

        let visible = visible_spans(text, 0..3, &top_layer(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["abc"]);
        assert!(texts_in_style(&visible, TOP_STYLE).is_empty());
    }

    #[test]
    fn given_match_after_a_tab_and_wide_characters_when_cutting_then_the_match_itself_is_highlighted()
     {
        let text = "漢\tżółw x";

        let visible = visible_spans(text, 0..30, &top_layer(text, "żółw"));

        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["żółw"]);
    }

    #[test]
    fn given_match_after_caret_notation_when_scrolled_then_the_highlight_keeps_its_columns() {
        let text = "\x1bab ERROR";

        let visible = visible_spans(text, 2..20, &top_layer(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["ab ", "ERROR"]);
        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["ERROR"]);
    }

    #[test]
    fn given_many_matches_on_a_line_when_cutting_then_each_one_is_highlighted() {
        let text = "x".repeat(1000);

        let visible = visible_spans(&text, 990..1000, &top_layer(&text, "x"));

        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["x".repeat(10)]);
    }

    #[test]
    fn given_overlapping_layers_when_cutting_then_the_first_layer_wins_where_they_overlap() {
        let text = "took=442ms";
        let match_layers = [
            match_layer(text, "442", TOP_STYLE),
            match_layer(text, "took=4", LOWER_STYLE),
        ];

        let visible = visible_spans(text, 0..20, &match_layers);

        assert_eq!(span_texts(&visible), ["took=", "442", "ms"]);
        assert_eq!(texts_in_style(&visible, LOWER_STYLE), ["took="]);
        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["442"]);
    }

    #[test]
    fn given_lower_layer_match_around_a_top_layer_match_when_cutting_then_it_shows_on_both_sides() {
        let text = "a user=42 b";
        let match_layers = [
            match_layer(text, "er=4", TOP_STYLE),
            match_layer(text, "user=42", LOWER_STYLE),
        ];

        let visible = visible_spans(text, 0..20, &match_layers);

        assert_eq!(span_texts(&visible), ["a ", "us", "er=4", "2", " b"]);
        assert_eq!(texts_in_style(&visible, LOWER_STYLE), ["us", "2"]);
    }

    #[test]
    fn given_overlap_cut_by_the_left_edge_when_scrolled_then_each_layer_keeps_its_visible_part() {
        let text = "took=442ms";
        let match_layers = [
            match_layer(text, "442", TOP_STYLE),
            match_layer(text, "took=4", LOWER_STYLE),
        ];

        let visible = visible_spans(text, 3..8, &match_layers);

        assert_eq!(span_texts(&visible), ["k=", "442"]);
        assert_eq!(texts_in_style(&visible, LOWER_STYLE), ["k="]);
    }

    #[test]
    fn given_layers_without_matches_on_the_line_when_cutting_then_the_text_is_plain() {
        let text = "plain line";
        let match_layers = [
            match_layer(text, "ERROR", TOP_STYLE),
            match_layer(text, "WARN", LOWER_STYLE),
        ];

        let visible = visible_spans(text, 0..20, &match_layers);

        assert_eq!(visible, [Span::raw("plain line")]);
    }

    #[test]
    fn given_controls_inside_and_past_the_visible_columns_when_cutting_then_only_those_columns_are_kept()
     {
        let visible = visible_spans("abc\x1bdefgh\x1b", 2..7, &[]);

        let span_texts: Vec<&str> = visible.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(span_texts, ["c", "^[", "de"]);
        assert_eq!(visible[1].style, CONTROL_CHARACTER_STYLE);
    }

    #[test]
    fn given_color_escape_sequences_when_cutting_then_they_are_hidden() {
        let visible = visible_spans("a\x1b[31mred\x1b[0m b", 0..20, &[]);

        assert_eq!(span_texts(&visible), ["ared b"]);
    }

    #[test]
    fn given_escape_sequence_before_the_visible_columns_when_scrolled_then_it_takes_no_columns() {
        let visible = visible_spans("\x1b[1;31mabcdef", 2..4, &[]);

        assert_eq!(span_texts(&visible), ["cd"]);
    }

    #[test]
    fn given_match_between_escape_sequences_when_cutting_then_the_match_is_highlighted() {
        let text = "\x1b[1mERROR\x1b[0m here";

        let visible = visible_spans(text, 0..20, &top_layer(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["ERROR", " here"]);
        assert_eq!(texts_in_style(&visible, TOP_STYLE), ["ERROR"]);
    }

    #[test]
    fn given_unterminated_escape_sequence_when_cutting_then_it_shows_in_caret_notation() {
        let visible = visible_spans("a\x1b[31", 0..20, &[]);

        assert_eq!(span_texts(&visible), ["a", "^[", "[31"]);
    }

    #[test]
    fn given_escape_not_followed_by_a_bracket_when_cutting_then_it_shows_in_caret_notation() {
        let visible = visible_spans("\x1b(B ok", 0..20, &[]);

        assert_eq!(span_texts(&visible), ["^[", "(B ok"]);
    }
}
