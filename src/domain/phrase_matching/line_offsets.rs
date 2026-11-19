pub(super) const LINES_PER_BLOCK: usize = 65_536;
const BITS_PER_WORD: usize = 64;
const WORDS_PER_BITMAP: usize = LINES_PER_BLOCK / BITS_PER_WORD;
const LONGEST_LIST: usize = 4_096;

pub(super) enum LineOffsets {
    List(Vec<u16>),
    Bitmap(Box<[u64; WORDS_PER_BITMAP]>),
}

impl Default for LineOffsets {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl LineOffsets {
    pub(super) fn insert(&mut self, offset: usize) {
        match self {
            Self::List(listed_offsets) => {
                listed_offsets.push(offset as u16);
                if listed_offsets.len() > LONGEST_LIST {
                    *self = Self::Bitmap(bitmap_of(listed_offsets));
                }
            }
            Self::Bitmap(bitmap_words) => mark(bitmap_words, offset),
        }
    }

    pub(super) fn count(&self) -> usize {
        match self {
            Self::List(listed_offsets) => listed_offsets.len(),
            Self::Bitmap(bitmap_words) => bitmap_words.iter().map(|&word| marked_count(word)).sum(),
        }
    }

    pub(super) fn count_before(&self, offset: usize) -> usize {
        match self {
            Self::List(listed_offsets) => {
                listed_offsets.partition_point(|&listed_offset| usize::from(listed_offset) < offset)
            }
            Self::Bitmap(bitmap_words) => count_marked_before(bitmap_words, offset),
        }
    }

    pub(super) fn contains(&self, offset: usize) -> bool {
        match self {
            Self::List(listed_offsets) => u16::try_from(offset)
                .is_ok_and(|offset| listed_offsets.binary_search(&offset).is_ok()),
            Self::Bitmap(bitmap_words) => is_marked(bitmap_words, offset),
        }
    }

    pub(super) fn get(&self, position: usize) -> Option<usize> {
        match self {
            Self::List(listed_offsets) => listed_offsets.get(position).copied().map(usize::from),
            Self::Bitmap(bitmap_words) => nth_marked_offset(bitmap_words, position),
        }
    }
}

fn bitmap_of(listed_offsets: &[u16]) -> Box<[u64; WORDS_PER_BITMAP]> {
    let mut bitmap_words = Box::new([0; WORDS_PER_BITMAP]);
    for &listed_offset in listed_offsets {
        mark(&mut bitmap_words, usize::from(listed_offset));
    }
    bitmap_words
}

fn mark(bitmap_words: &mut [u64; WORDS_PER_BITMAP], offset: usize) {
    bitmap_words[offset / BITS_PER_WORD] |= 1 << (offset % BITS_PER_WORD);
}

fn is_marked(bitmap_words: &[u64; WORDS_PER_BITMAP], offset: usize) -> bool {
    bitmap_words
        .get(offset / BITS_PER_WORD)
        .is_some_and(|word| word & (1 << (offset % BITS_PER_WORD)) != 0)
}

fn count_marked_before(bitmap_words: &[u64; WORDS_PER_BITMAP], offset: usize) -> usize {
    let offset = offset.min(LINES_PER_BLOCK);
    let (word_index, bit_index) = (offset / BITS_PER_WORD, offset % BITS_PER_WORD);
    let marked_in_whole_words: usize = bitmap_words[..word_index]
        .iter()
        .map(|&word| marked_count(word))
        .sum();
    let marked_in_partial_word = bitmap_words
        .get(word_index)
        .map_or(0, |&word| marked_count(word & lowest_bits_mask(bit_index)));
    marked_in_whole_words + marked_in_partial_word
}

fn nth_marked_offset(bitmap_words: &[u64; WORDS_PER_BITMAP], position: usize) -> Option<usize> {
    let mut marks_to_skip = position;
    for (word_index, &word) in bitmap_words.iter().enumerate() {
        let marks_in_word = marked_count(word);
        if marks_to_skip < marks_in_word {
            return Some(word_index * BITS_PER_WORD + nth_set_bit(word, marks_to_skip));
        }
        marks_to_skip -= marks_in_word;
    }
    None
}

fn nth_set_bit(word: u64, position: usize) -> usize {
    let word_from_nth_set_bit = (0..position).fold(word, |word, _| without_lowest_set_bit(word));
    word_from_nth_set_bit.trailing_zeros() as usize
}

fn without_lowest_set_bit(word: u64) -> u64 {
    word & (word - 1)
}

fn lowest_bits_mask(bit_count: usize) -> u64 {
    (1 << bit_count) - 1
}

fn marked_count(word: u64) -> usize {
    word.count_ones() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORD_EDGE_OFFSETS: [usize; 6] = [0, 1, 63, 64, 1000, LINES_PER_BLOCK - 1];

    fn make_inserted_offsets(offsets: &[usize]) -> LineOffsets {
        let mut line_offsets = LineOffsets::default();
        for &offset in offsets {
            line_offsets.insert(offset);
        }
        line_offsets
    }

    fn make_bitmap(offsets: &[usize]) -> LineOffsets {
        let listed_offsets: Vec<u16> = offsets.iter().map(|&offset| offset as u16).collect();
        LineOffsets::Bitmap(bitmap_of(&listed_offsets))
    }

    fn every_nth_offset(step: usize) -> Vec<usize> {
        (0..LINES_PER_BLOCK).step_by(step).collect()
    }

    #[test]
    fn given_as_many_offsets_as_the_longest_list_when_inserted_then_they_stay_a_list() {
        let line_offsets = make_inserted_offsets(&(0..LONGEST_LIST).collect::<Vec<_>>());

        assert!(matches!(line_offsets, LineOffsets::List(_)));
        assert_eq!(line_offsets.count(), LONGEST_LIST);
    }

    #[test]
    fn given_one_offset_more_than_the_longest_list_when_inserted_then_they_become_a_bitmap() {
        let offsets: Vec<usize> = (0..=LONGEST_LIST).map(|offset| offset * 3).collect();

        let line_offsets = make_inserted_offsets(&offsets);

        assert!(matches!(line_offsets, LineOffsets::Bitmap(_)));
        assert_eq!(line_offsets.count(), LONGEST_LIST + 1);
        assert!(offsets.iter().all(|&offset| line_offsets.contains(offset)));
        assert!(!line_offsets.contains(1));
    }

    #[test]
    fn given_bitmap_when_more_offsets_are_inserted_then_they_are_found_in_order() {
        let mut line_offsets = make_inserted_offsets(&every_nth_offset(8));

        line_offsets.insert(LINES_PER_BLOCK - 1);

        assert!(matches!(line_offsets, LineOffsets::Bitmap(_)));
        assert_eq!(line_offsets.count(), LINES_PER_BLOCK / 8 + 1);
        assert_eq!(
            line_offsets.get(LINES_PER_BLOCK / 8),
            Some(LINES_PER_BLOCK - 1)
        );
    }

    #[test]
    fn given_no_offsets_when_queried_then_there_are_none() {
        for line_offsets in [make_inserted_offsets(&[]), make_bitmap(&[])] {
            assert_eq!(line_offsets.count(), 0);
            assert_eq!(line_offsets.count_before(LINES_PER_BLOCK), 0);
            assert_eq!(line_offsets.get(0), None);
            assert!(!line_offsets.contains(0));
        }
    }

    #[test]
    fn given_offsets_on_word_edges_when_counted_then_a_list_and_a_bitmap_count_them_all() {
        for line_offsets in [
            make_inserted_offsets(&WORD_EDGE_OFFSETS),
            make_bitmap(&WORD_EDGE_OFFSETS),
        ] {
            assert_eq!(line_offsets.count(), WORD_EDGE_OFFSETS.len());
        }
    }

    #[test]
    fn given_offsets_on_word_edges_when_counting_those_before_then_an_offset_never_counts_itself() {
        for line_offsets in [
            make_inserted_offsets(&WORD_EDGE_OFFSETS),
            make_bitmap(&WORD_EDGE_OFFSETS),
        ] {
            assert_eq!(line_offsets.count_before(0), 0);
            assert_eq!(line_offsets.count_before(1), 1);
            assert_eq!(line_offsets.count_before(63), 2);
            assert_eq!(line_offsets.count_before(64), 3);
            assert_eq!(line_offsets.count_before(65), 4);
            assert_eq!(line_offsets.count_before(LINES_PER_BLOCK - 1), 5);
            assert_eq!(line_offsets.count_before(LINES_PER_BLOCK), 6);
            assert_eq!(line_offsets.count_before(LINES_PER_BLOCK * 2), 6);
        }
    }

    #[test]
    fn given_offsets_on_word_edges_when_getting_each_position_then_they_come_in_order() {
        for line_offsets in [
            make_inserted_offsets(&WORD_EDGE_OFFSETS),
            make_bitmap(&WORD_EDGE_OFFSETS),
        ] {
            let offsets: Vec<Option<usize>> = (0..=WORD_EDGE_OFFSETS.len())
                .map(|position| line_offsets.get(position))
                .collect();

            assert_eq!(
                offsets,
                [
                    Some(0),
                    Some(1),
                    Some(63),
                    Some(64),
                    Some(1000),
                    Some(LINES_PER_BLOCK - 1),
                    None
                ]
            );
        }
    }

    #[test]
    fn given_offsets_on_word_edges_when_asking_whether_offsets_are_in_then_only_those_are() {
        for line_offsets in [
            make_inserted_offsets(&WORD_EDGE_OFFSETS),
            make_bitmap(&WORD_EDGE_OFFSETS),
        ] {
            assert!(
                WORD_EDGE_OFFSETS
                    .iter()
                    .all(|&offset| line_offsets.contains(offset))
            );
            assert!(!line_offsets.contains(2));
            assert!(!line_offsets.contains(62));
            assert!(!line_offsets.contains(65));
            assert!(!line_offsets.contains(LINES_PER_BLOCK));
        }
    }

    #[test]
    fn given_the_same_offsets_as_a_list_and_a_bitmap_when_queried_then_they_answer_the_same() {
        let offsets = every_nth_offset(37);
        let line_offsets_list = make_inserted_offsets(&offsets);
        let line_offsets_bitmap = make_bitmap(&offsets);

        for asked_offset in (0..=LINES_PER_BLOCK).step_by(11) {
            assert_eq!(
                line_offsets_list.count_before(asked_offset),
                line_offsets_bitmap.count_before(asked_offset)
            );
            assert_eq!(
                line_offsets_list.contains(asked_offset),
                line_offsets_bitmap.contains(asked_offset)
            );
        }
        for position in 0..=offsets.len() {
            assert_eq!(
                line_offsets_list.get(position),
                line_offsets_bitmap.get(position)
            );
        }
    }

    #[test]
    fn given_every_offset_in_a_bitmap_when_queried_then_each_position_is_its_own_offset() {
        let line_offsets = make_bitmap(&every_nth_offset(1));

        assert_eq!(line_offsets.count(), LINES_PER_BLOCK);
        assert_eq!(line_offsets.get(0), Some(0));
        assert_eq!(line_offsets.get(40_000), Some(40_000));
        assert_eq!(
            line_offsets.get(LINES_PER_BLOCK - 1),
            Some(LINES_PER_BLOCK - 1)
        );
        assert_eq!(line_offsets.get(LINES_PER_BLOCK), None);
    }
}
