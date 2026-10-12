use std::ops::Range;

use ratatui::{buffer::CellWidth, style::Style, text::Span};

use super::palette;

const TAB_STOP: usize = 8;
const HIGHLIGHT_STYLE: Style = Style::new().fg(palette::BACKGROUND).bg(palette::YELLOW);
const CONTROL_CHARACTER_STYLE: Style = Style::new().fg(palette::FOREGROUND_MUTED);

pub(super) fn visible_spans(
    text: &str,
    visible_columns: Range<usize>,
    highlight_ranges: &[Range<usize>],
) -> Vec<Span<'static>> {
    let bytes_before_visible_end = &text.as_bytes()[..visible_columns.end.min(text.len())];
    if highlight_ranges.is_empty() && is_printable_ascii(bytes_before_visible_end) {
        return vec![Span::raw(visible_ascii_text(text, visible_columns))];
    }
    let mut row = VisibleRow::new(visible_columns, highlight_ranges);
    let mut plain_text_start = 0;
    let controls = text
        .char_indices()
        .filter(|(_, character)| character.is_control());
    for (control_offset, control) in controls {
        row.push_text(&text[plain_text_start..control_offset], plain_text_start);
        row.push_control(control);
        plain_text_start = control_offset + control.len_utf8();
    }
    row.push_text(&text[plain_text_start..], plain_text_start);
    row.spans
}

fn is_printable_ascii(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| matches!(byte, b' '..=b'~'))
}

fn visible_ascii_text(text: &str, visible_columns: Range<usize>) -> String {
    let visible_start = visible_columns.start.min(text.len());
    let visible_end = visible_columns.end.min(text.len());
    text[visible_start..visible_end].to_owned()
}

/// One log line as it appears in the visible columns. Tabs expand to the next tab stop, ASCII
/// control characters show muted in caret notation (`^[`), and other control characters are
/// dropped. A wide character cut by the left edge becomes spaces, so the rest of the line keeps its
/// columns. Text inside the highlight ranges, which are byte ranges of the line in order, is
/// highlighted.
struct VisibleRow<'a> {
    visible_columns: Range<usize>,
    highlight_ranges: &'a [Range<usize>],
    current_column: usize,
    spans: Vec<Span<'static>>,
}

impl<'a> VisibleRow<'a> {
    fn new(visible_columns: Range<usize>, highlight_ranges: &'a [Range<usize>]) -> Self {
        Self {
            visible_columns,
            highlight_ranges,
            current_column: 0,
            spans: Vec::new(),
        }
    }

    fn push_text(&mut self, text: &str, text_byte_offset: usize) {
        let span = Span::raw(text);
        let mut grapheme_byte_offset = text_byte_offset;

        for grapheme in span.styled_graphemes(Style::default()) {
            if self.is_full() {
                return;
            }
            let grapheme_width = usize::from(grapheme.symbol.cell_width());
            let style = self.style_at(grapheme_byte_offset);
            self.push_symbol(grapheme.symbol, grapheme_width, style);
            grapheme_byte_offset += grapheme.symbol.len();
        }
    }

    fn style_at(&mut self, byte_offset: usize) -> Style {
        while let Some((passed_range, later_ranges)) = self.highlight_ranges.split_first()
            && passed_range.end <= byte_offset
        {
            self.highlight_ranges = later_ranges;
        }

        match self.highlight_ranges.first() {
            Some(highlight_range) if highlight_range.contains(&byte_offset) => HIGHLIGHT_STYLE,
            _ => Style::default(),
        }
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
            return;
        }
        let next_column = self.current_column + symbol_width;
        if self.current_column >= self.visible_columns.start {
            self.append(symbol, style);
        } else if next_column > self.visible_columns.start {
            let visible_width = next_column - self.visible_columns.start;
            self.append(&" ".repeat(visible_width), Style::default());
        }
        self.current_column = next_column;
    }

    fn append(&mut self, text: &str, style: Style) {
        match self.spans.last_mut() {
            Some(last_span) if last_span.style == style => {
                last_span.content.to_mut().push_str(text)
            }
            _ => self.spans.push(Span::styled(text.to_owned(), style)),
        }
    }

    fn is_full(&self) -> bool {
        self.current_column >= self.visible_columns.end
    }
}

fn caret_notation(control: char) -> String {
    format!("^{}", char::from(control as u8 ^ 0x40))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_nul_and_del_when_converting_then_they_use_standard_caret_notation() {
        assert_eq!(caret_notation('\0'), "^@");
        assert_eq!(caret_notation('\x7f'), "^?");
    }

    fn match_ranges(text: &str, phrase: &str) -> Vec<Range<usize>> {
        text.match_indices(phrase)
            .map(|(match_start, _)| match_start..match_start + phrase.len())
            .collect()
    }

    fn span_texts(spans: &[Span]) -> Vec<String> {
        spans.iter().map(|span| span.content.to_string()).collect()
    }

    fn highlighted_texts(spans: &[Span]) -> Vec<String> {
        spans
            .iter()
            .filter(|span| span.style == HIGHLIGHT_STYLE)
            .map(|span| span.content.to_string())
            .collect()
    }

    #[test]
    fn given_ascii_line_with_two_matches_when_cutting_then_each_match_is_its_own_highlighted_span()
    {
        let text = "an ERROR, ERROR";

        let visible = visible_spans(text, 0..20, &match_ranges(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["an ", "ERROR", ", ", "ERROR"]);
        assert_eq!(highlighted_texts(&visible), ["ERROR", "ERROR"]);
    }

    #[test]
    fn given_match_cut_by_the_left_edge_when_scrolled_then_only_its_visible_part_is_highlighted() {
        let text = "an ERROR here";

        let visible = visible_spans(text, 5..13, &match_ranges(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["ROR", " here"]);
        assert_eq!(highlighted_texts(&visible), ["ROR"]);
    }

    #[test]
    fn given_match_past_the_right_edge_when_cutting_then_nothing_is_highlighted() {
        let text = "abc ERROR";

        let visible = visible_spans(text, 0..3, &match_ranges(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["abc"]);
        assert!(highlighted_texts(&visible).is_empty());
    }

    #[test]
    fn given_match_after_a_tab_and_wide_characters_when_cutting_then_the_match_itself_is_highlighted()
     {
        let text = "漢\tżółw x";

        let visible = visible_spans(text, 0..30, &match_ranges(text, "żółw"));

        assert_eq!(highlighted_texts(&visible), ["żółw"]);
    }

    #[test]
    fn given_match_after_caret_notation_when_scrolled_then_the_highlight_keeps_its_columns() {
        let text = "\x1bab ERROR";

        let visible = visible_spans(text, 2..20, &match_ranges(text, "ERROR"));

        assert_eq!(span_texts(&visible), ["ab ", "ERROR"]);
        assert_eq!(highlighted_texts(&visible), ["ERROR"]);
    }

    #[test]
    fn given_many_matches_on_a_line_when_cutting_then_each_one_is_highlighted() {
        let text = "x".repeat(1000);

        let visible = visible_spans(&text, 990..1000, &match_ranges(&text, "x"));

        assert_eq!(highlighted_texts(&visible), ["x".repeat(10)]);
    }

    #[test]
    fn given_controls_inside_and_past_the_visible_columns_when_cutting_then_only_those_columns_are_kept()
     {
        let visible = visible_spans("abc\x1bdefgh\x1b", 2..7, &[]);

        let span_texts: Vec<&str> = visible.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(span_texts, ["c", "^[", "de"]);
        assert_eq!(visible[1].style, CONTROL_CHARACTER_STYLE);
    }
}
