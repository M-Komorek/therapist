pub const HIGHLIGHT_COLOR_COUNT: usize = 12;

#[derive(Debug, PartialEq)]
pub struct Highlight {
    pub phrase: String,
    pub color_index: usize,
}

/// The highlighted phrases, oldest first, each at most once. A new phrase takes the color the
/// fewest highlights use, the first one on a tie, so a color repeats only when every color is in
/// use.
#[derive(Default)]
pub struct Highlights {
    highlights_oldest_first: Vec<Highlight>,
}

impl Highlights {
    pub fn toggle(&mut self, phrase: String) {
        match self.position_of(&phrase) {
            Some(position) => {
                self.highlights_oldest_first.remove(position);
            }
            None => self.add(phrase),
        }
    }

    pub fn contains(&self, phrase: &str) -> bool {
        self.position_of(phrase).is_some()
    }

    pub fn remove_all(&mut self) {
        self.highlights_oldest_first.clear();
    }

    pub fn oldest_first(&self) -> std::slice::Iter<'_, Highlight> {
        self.highlights_oldest_first.iter()
    }

    fn add(&mut self, phrase: String) {
        let color_index = self.least_used_color_index();
        self.highlights_oldest_first.push(Highlight {
            phrase,
            color_index,
        });
    }

    fn position_of(&self, phrase: &str) -> Option<usize> {
        self.highlights_oldest_first
            .iter()
            .position(|highlight| highlight.phrase == phrase)
    }

    fn least_used_color_index(&self) -> usize {
        let mut highlights_per_color = [0; HIGHLIGHT_COLOR_COUNT];
        for highlight in &self.highlights_oldest_first {
            highlights_per_color[highlight.color_index] += 1;
        }

        (0..HIGHLIGHT_COLOR_COUNT)
            .min_by_key(|&color_index| highlights_per_color[color_index])
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_highlights(phrases: &[&str]) -> Highlights {
        let mut highlights = Highlights::default();
        for phrase in phrases {
            highlights.toggle((*phrase).to_owned());
        }
        highlights
    }

    fn colors_by_phrase(highlights: &Highlights) -> Vec<(&str, usize)> {
        highlights
            .oldest_first()
            .map(|highlight| (highlight.phrase.as_str(), highlight.color_index))
            .collect()
    }

    #[test]
    fn given_no_highlights_when_three_phrases_are_added_then_they_take_the_first_three_colors() {
        let highlights = make_highlights(&["ERROR", "WARN", "user=42"]);

        assert_eq!(
            colors_by_phrase(&highlights),
            [("ERROR", 0), ("WARN", 1), ("user=42", 2)]
        );
    }

    #[test]
    fn given_a_removed_highlight_when_a_phrase_is_added_then_it_takes_the_freed_color() {
        let mut highlights = make_highlights(&["ERROR", "WARN", "user=42"]);
        highlights.toggle("WARN".to_owned());

        highlights.toggle("took=4".to_owned());

        assert_eq!(
            colors_by_phrase(&highlights),
            [("ERROR", 0), ("user=42", 2), ("took=4", 1)]
        );
    }

    #[test]
    fn given_every_color_in_use_when_phrases_are_added_then_the_colors_start_again_from_the_first()
    {
        let mut highlights = Highlights::default();
        for phrase_number in 0..HIGHLIGHT_COLOR_COUNT {
            highlights.toggle(format!("phrase {phrase_number}"));
        }

        highlights.toggle("one more".to_owned());
        highlights.toggle("two more".to_owned());

        let colors: Vec<usize> = colors_by_phrase(&highlights)
            .into_iter()
            .map(|(_, color_index)| color_index)
            .collect();
        let every_color_then_the_first_two: Vec<usize> =
            (0..HIGHLIGHT_COLOR_COUNT).chain([0, 1]).collect();
        assert_eq!(colors, every_color_then_the_first_two);
    }

    #[test]
    fn given_one_highlight_when_checking_phrases_then_only_that_exact_phrase_is_contained() {
        let highlights = make_highlights(&["ERROR"]);

        assert!(highlights.contains("ERROR"));
        assert!(!highlights.contains("error"));
        assert!(!highlights.contains("ERR"));
    }

    #[test]
    fn given_highlighted_phrase_when_toggled_then_it_is_removed() {
        let mut highlights = make_highlights(&["ERROR", "WARN"]);

        highlights.toggle("ERROR".to_owned());

        assert_eq!(colors_by_phrase(&highlights), [("WARN", 1)]);
    }

    #[test]
    fn given_phrases_differing_only_in_case_when_added_then_both_are_highlighted() {
        let highlights = make_highlights(&["ERROR", "error"]);

        assert_eq!(colors_by_phrase(&highlights), [("ERROR", 0), ("error", 1)]);
    }

    #[test]
    fn given_three_highlights_when_the_middle_one_is_removed_then_the_others_keep_their_order_and_colors()
     {
        let mut highlights = make_highlights(&["ERROR", "WARN", "user=42"]);

        highlights.toggle("WARN".to_owned());

        assert_eq!(
            colors_by_phrase(&highlights),
            [("ERROR", 0), ("user=42", 2)]
        );
    }

    #[test]
    fn given_highlights_when_all_are_removed_then_the_next_phrase_takes_the_first_color() {
        let mut highlights = make_highlights(&["ERROR", "WARN"]);

        highlights.remove_all();
        highlights.toggle("user=42".to_owned());

        assert_eq!(colors_by_phrase(&highlights), [("user=42", 0)]);
    }
}
