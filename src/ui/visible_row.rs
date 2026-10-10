use std::ops::Range;

use ratatui::{buffer::CellWidth, style::Style, text::Span};

const TAB_STOP: usize = 8;

pub(super) fn visible_spans(text: &str, visible_columns: Range<usize>) -> Vec<Span<'static>> {
    let bytes_before_visible_end = &text.as_bytes()[..visible_columns.end.min(text.len())];
    if is_printable_ascii(bytes_before_visible_end) {
        return vec![Span::raw(visible_ascii_text(text, visible_columns))];
    }
    let mut row = VisibleRow::new(visible_columns);
    let mut plain_text_start = 0;
    let controls = text
        .char_indices()
        .filter(|(_, character)| character.is_control());
    for (control_offset, control) in controls {
        row.push_text(&text[plain_text_start..control_offset]);
        row.push_control(control);
        plain_text_start = control_offset + control.len_utf8();
    }
    row.push_text(&text[plain_text_start..]);
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
/// control characters show dimmed in caret notation (`^[`), and other control characters are
/// dropped. A wide character cut by the left edge becomes spaces, so the rest of the line keeps its
/// columns.
struct VisibleRow {
    visible_columns: Range<usize>,
    current_column: usize,
    spans: Vec<Span<'static>>,
}

impl VisibleRow {
    fn new(visible_columns: Range<usize>) -> Self {
        Self {
            visible_columns,
            current_column: 0,
            spans: Vec::new(),
        }
    }

    fn push_text(&mut self, text: &str) {
        let span = Span::raw(text);
        for grapheme in span.styled_graphemes(Style::default()) {
            if self.is_full() {
                return;
            }
            let grapheme_width = usize::from(grapheme.symbol.cell_width());
            self.push_symbol(grapheme.symbol, grapheme_width, Style::default());
        }
    }

    fn push_control(&mut self, control: char) {
        if control == '\t' {
            let spaces_to_tab_stop = TAB_STOP - self.current_column % TAB_STOP;
            let tab_spaces = " ".repeat(spaces_to_tab_stop);
            self.push_symbol(&tab_spaces, spaces_to_tab_stop, Style::default());
        } else if control.is_ascii_control() {
            self.push_symbol(&caret_notation(control), 2, Style::new().dim());
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

    #[test]
    fn given_controls_inside_and_past_the_visible_columns_when_cutting_then_only_those_columns_are_kept()
     {
        let visible = visible_spans("abc\x1bdefgh\x1b", 2..7);

        let span_texts: Vec<&str> = visible.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(span_texts, ["c", "^[", "de"]);
        assert_eq!(visible[1].style, Style::new().dim());
    }
}
