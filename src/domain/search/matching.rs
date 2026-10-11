use memchr::memmem;

pub fn find_matching_lines(chunk: &[u8], chunk_first_line: usize, phrase: &str) -> Vec<usize> {
    if phrase.is_empty() {
        return Vec::new();
    }

    let mut matching_lines = Vec::new();
    let mut current_line = chunk_first_line;
    let mut newlines_counted_up_to = 0;

    for match_offset in memmem::find_iter(chunk, phrase.as_bytes()) {
        current_line += count_newlines(&chunk[newlines_counted_up_to..match_offset]);
        newlines_counted_up_to = match_offset;
        if matching_lines.last() != Some(&current_line) {
            matching_lines.push(current_line);
        }
    }
    matching_lines
}

fn count_newlines(text: &[u8]) -> usize {
    memchr::memchr_iter(b'\n', text).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_matches_at_the_very_start_and_end_when_searching_then_both_lines_are_found() {
        assert_eq!(
            find_matching_lines(b"ERROR first\nsecond\nlast ERROR", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_several_matches_on_one_line_when_searching_then_the_line_is_found_once() {
        assert_eq!(
            find_matching_lines(b"ERROR ERROR\nok\nERRORERROR\n", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_crlf_line_endings_when_searching_then_lines_are_counted_by_newlines() {
        assert_eq!(
            find_matching_lines(b"ok\r\nERROR\r\nok\r\nERROR\r\n", 0, "ERROR"),
            [1, 3]
        );
    }

    #[test]
    fn given_empty_phrase_when_searching_then_no_line_matches() {
        assert!(find_matching_lines(b"a\nb\n", 0, "").is_empty());
    }

    #[test]
    fn given_phrase_not_in_the_chunk_when_searching_then_no_line_matches() {
        assert!(find_matching_lines(b"a\nb\n", 0, "ERROR").is_empty());
    }

    #[test]
    fn given_chunk_starting_at_line_100_when_searching_then_lines_are_counted_from_there() {
        assert_eq!(find_matching_lines(b"ok\nERROR\nok\n", 100, "ERROR"), [101]);
    }
}
