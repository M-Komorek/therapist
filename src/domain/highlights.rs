pub const HIGHLIGHT_COLOR_COUNT: usize = 12;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum HighlightError {
    #[error("Already highlighted: {0}")]
    AlreadyHighlighted(String),
    #[error("Not highlighted: {0}")]
    NotHighlighted(String),
}

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
    pub fn add(&mut self, phrase: String) -> Result<(), HighlightError> {
        if self.position_of(&phrase).is_some() {
            return Err(HighlightError::AlreadyHighlighted(phrase));
        }

        let color_index = self.least_used_color_index();
        self.highlights_oldest_first.push(Highlight {
            phrase,
            color_index,
        });
        Ok(())
    }

    pub fn remove(&mut self, phrase: &str) -> Result<(), HighlightError> {
        let position = self
            .position_of(phrase)
            .ok_or_else(|| HighlightError::NotHighlighted(phrase.to_owned()))?;

        self.highlights_oldest_first.remove(position);
        Ok(())
    }

    pub fn remove_all(&mut self) {
        self.highlights_oldest_first.clear();
    }

    pub fn oldest_first(&self) -> std::slice::Iter<'_, Highlight> {
        self.highlights_oldest_first.iter()
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
            highlights.add((*phrase).to_owned()).unwrap();
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
        highlights.remove("WARN").unwrap();

        highlights.add("took=4".to_owned()).unwrap();

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
            highlights.add(format!("phrase {phrase_number}")).unwrap();
        }

        highlights.add("one more".to_owned()).unwrap();
        highlights.add("two more".to_owned()).unwrap();

        let colors: Vec<usize> = colors_by_phrase(&highlights)
            .into_iter()
            .map(|(_, color_index)| color_index)
            .collect();
        let every_color_then_the_first_two: Vec<usize> =
            (0..HIGHLIGHT_COLOR_COUNT).chain([0, 1]).collect();
        assert_eq!(colors, every_color_then_the_first_two);
    }

    #[test]
    fn given_highlighted_phrase_when_it_is_added_again_then_it_is_refused_and_kept_once() {
        let mut highlights = make_highlights(&["ERROR"]);

        let result = highlights.add("ERROR".to_owned());

        assert_eq!(
            result,
            Err(HighlightError::AlreadyHighlighted("ERROR".to_owned()))
        );
        assert_eq!(colors_by_phrase(&highlights), [("ERROR", 0)]);
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

        highlights.remove("WARN").unwrap();

        assert_eq!(
            colors_by_phrase(&highlights),
            [("ERROR", 0), ("user=42", 2)]
        );
    }

    #[test]
    fn given_phrase_that_is_not_highlighted_when_removing_then_it_is_refused() {
        let mut highlights = make_highlights(&["ERROR"]);

        let result = highlights.remove("WARN");

        assert_eq!(
            result,
            Err(HighlightError::NotHighlighted("WARN".to_owned()))
        );
        assert_eq!(colors_by_phrase(&highlights), [("ERROR", 0)]);
    }

    #[test]
    fn given_highlights_when_all_are_removed_then_the_next_phrase_takes_the_first_color() {
        let mut highlights = make_highlights(&["ERROR", "WARN"]);

        highlights.remove_all();
        highlights.add("user=42".to_owned()).unwrap();

        assert_eq!(colors_by_phrase(&highlights), [("user=42", 0)]);
    }

    #[test]
    fn given_highlight_errors_when_displayed_then_they_name_the_phrase() {
        assert_eq!(
            HighlightError::AlreadyHighlighted("ERROR".to_owned()).to_string(),
            "Already highlighted: ERROR"
        );
        assert_eq!(
            HighlightError::NotHighlighted("WARN".to_owned()).to_string(),
            "Not highlighted: WARN"
        );
    }
}
