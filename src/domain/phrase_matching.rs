use memchr::memmem;

#[derive(Debug, PartialEq)]
pub struct MatchingLinesBatch {
    pub matching_lines: Vec<usize>,
    pub percent_scanned: u64,
}

pub fn find_matching_lines(
    chunk: &[u8],
    chunk_first_line: usize,
    phrases: &[String],
) -> Vec<usize> {
    let Some((first_phrase, other_phrases)) = phrases.split_first() else {
        return Vec::new();
    };

    let mut matching_lines = find_lines_containing(chunk, chunk_first_line, first_phrase);
    for phrase in other_phrases {
        let lines_with_phrase = find_lines_containing(chunk, chunk_first_line, phrase);
        matching_lines.retain(|line| lines_with_phrase.binary_search(line).is_ok());
    }
    matching_lines
}

fn find_lines_containing(chunk: &[u8], chunk_first_line: usize, phrase: &str) -> Vec<usize> {
    if phrase.is_empty() {
        return Vec::new();
    }

    let mut lines_with_phrase = Vec::new();
    let mut current_line = chunk_first_line;
    let mut newlines_counted_up_to = 0;

    for match_offset in memmem::find_iter(chunk, phrase.as_bytes()) {
        current_line += count_newlines(&chunk[newlines_counted_up_to..match_offset]);
        newlines_counted_up_to = match_offset;
        if lines_with_phrase.last() != Some(&current_line) {
            lines_with_phrase.push(current_line);
        }
    }
    lines_with_phrase
}

fn count_newlines(text: &[u8]) -> usize {
    memchr::memchr_iter(b'\n', text).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phrases(phrase_texts: &[&str]) -> Vec<String> {
        phrase_texts
            .iter()
            .map(|phrase| (*phrase).to_owned())
            .collect()
    }

    #[test]
    fn given_matches_at_the_very_start_and_end_when_searching_then_both_lines_are_found() {
        assert_eq!(
            find_lines_containing(b"ERROR first\nsecond\nlast ERROR", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_several_matches_on_one_line_when_searching_then_the_line_is_found_once() {
        assert_eq!(
            find_lines_containing(b"ERROR ERROR\nok\nERRORERROR\n", 0, "ERROR"),
            [0, 2]
        );
    }

    #[test]
    fn given_crlf_line_endings_when_searching_then_lines_are_counted_by_newlines() {
        assert_eq!(
            find_lines_containing(b"ok\r\nERROR\r\nok\r\nERROR\r\n", 0, "ERROR"),
            [1, 3]
        );
    }

    #[test]
    fn given_empty_phrase_when_searching_then_no_line_matches() {
        assert!(find_lines_containing(b"a\nb\n", 0, "").is_empty());
    }

    #[test]
    fn given_phrase_not_in_the_chunk_when_searching_then_no_line_matches() {
        assert!(find_lines_containing(b"a\nb\n", 0, "ERROR").is_empty());
    }

    #[test]
    fn given_chunk_starting_at_line_100_when_searching_then_lines_are_counted_from_there() {
        assert_eq!(
            find_lines_containing(b"ok\nERROR\nok\n", 100, "ERROR"),
            [101]
        );
    }

    #[test]
    fn given_two_phrases_when_finding_matching_lines_then_only_lines_with_both_match() {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=42\nuser=42 ERROR\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &phrases(&["ERROR", "user=42"])),
            [0, 3]
        );
    }

    #[test]
    fn given_two_phrases_in_either_order_when_finding_matching_lines_then_the_lines_are_the_same() {
        let chunk = b"ERROR user=42\nERROR user=7\nINFO user=42\n";

        assert_eq!(
            find_matching_lines(chunk, 0, &phrases(&["user=42", "ERROR"])),
            find_matching_lines(chunk, 0, &phrases(&["ERROR", "user=42"]))
        );
    }

    #[test]
    fn given_phrases_on_different_lines_only_when_finding_matching_lines_then_no_line_matches() {
        assert!(
            find_matching_lines(b"ERROR\nuser=42\n", 0, &phrases(&["ERROR", "user=42"])).is_empty()
        );
    }

    #[test]
    fn given_no_phrases_when_finding_matching_lines_then_no_line_matches() {
        assert!(find_matching_lines(b"a\nb\n", 0, &[]).is_empty());
    }

    #[test]
    fn given_chunk_starting_at_line_100_when_finding_lines_with_two_phrases_then_lines_are_counted_from_there()
     {
        assert_eq!(
            find_matching_lines(
                b"ERROR\nERROR user=42\n",
                100,
                &phrases(&["ERROR", "user=42"])
            ),
            [101]
        );
    }
}
