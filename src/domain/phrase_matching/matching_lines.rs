use std::ops::Range;

use super::{
    MatchingLinesBatch,
    line_offsets::{LINES_PER_BLOCK, LineOffsets},
};

/// The lines a phrase scan has found so far, and how much of the file it has read. The lines are
/// counted from 0 and come in file order, because a scan reads the file from the start.
#[derive(Default)]
pub struct MatchingLines {
    line_blocks: Vec<LineBlock>,
    percent_scanned: u64,
}

struct LineBlock {
    first_line: usize,
    match_count_before: usize,
    line_offsets: LineOffsets,
}

impl MatchingLines {
    pub fn add_batch(&mut self, matching_lines_batch: MatchingLinesBatch) {
        for line in matching_lines_batch.matching_lines {
            self.add_line(line);
        }
        self.percent_scanned = matching_lines_batch.percent_scanned;
    }

    pub fn percent_scanned(&self) -> u64 {
        self.percent_scanned
    }

    pub fn count(&self) -> usize {
        self.line_blocks.last().map_or(0, |last_block| {
            last_block.match_count_before + last_block.line_offsets.count()
        })
    }

    pub fn count_above(&self, line: usize) -> usize {
        let Some(line_block) = self.last_block_starting_at_or_above(line) else {
            return 0;
        };
        let offset = line - line_block.first_line;
        line_block.match_count_before + line_block.line_offsets.count_before(offset)
    }

    pub fn count_at_or_above(&self, line: usize) -> usize {
        self.count_above(line.saturating_add(1))
    }

    pub fn contains(&self, line: usize) -> bool {
        self.last_block_starting_at_or_above(line)
            .is_some_and(|line_block| {
                line_block
                    .line_offsets
                    .contains(line - line_block.first_line)
            })
    }

    pub fn get(&self, position: usize) -> Option<usize> {
        let blocks_up_to_position = self
            .line_blocks
            .partition_point(|line_block| line_block.match_count_before <= position);
        let line_block = self
            .line_blocks
            .get(blocks_up_to_position.checked_sub(1)?)?;
        let offset = line_block
            .line_offsets
            .get(position - line_block.match_count_before)?;
        Some(line_block.first_line + offset)
    }

    pub fn last(&self) -> Option<usize> {
        self.get(self.count().checked_sub(1)?)
    }

    pub fn in_positions(&self, positions: Range<usize>) -> Vec<usize> {
        positions.map_while(|position| self.get(position)).collect()
    }

    fn add_line(&mut self, line: usize) {
        let block_first_line = line - line % LINES_PER_BLOCK;
        let starts_a_block = self
            .line_blocks
            .last()
            .is_none_or(|last_block| last_block.first_line != block_first_line);
        if starts_a_block {
            self.start_block(block_first_line);
        }
        if let Some(last_block) = self.line_blocks.last_mut() {
            last_block.line_offsets.insert(line - block_first_line);
        }
    }

    fn start_block(&mut self, first_line: usize) {
        let match_count_before = self.count();
        self.line_blocks.push(LineBlock {
            first_line,
            match_count_before,
            line_offsets: LineOffsets::default(),
        });
    }

    fn last_block_starting_at_or_above(&self, line: usize) -> Option<&LineBlock> {
        let blocks_up_to_line = self
            .line_blocks
            .partition_point(|line_block| line_block.first_line <= line);
        self.line_blocks.get(blocks_up_to_line.checked_sub(1)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_matching_lines(lines: &[usize]) -> MatchingLines {
        let mut matching_lines = MatchingLines::default();
        matching_lines.add_batch(MatchingLinesBatch {
            matching_lines: lines.to_vec(),
            percent_scanned: 100,
        });
        matching_lines
    }

    #[test]
    fn given_no_batches_when_asking_for_matching_lines_then_there_are_none_and_nothing_is_scanned()
    {
        let matching_lines = MatchingLines::default();

        assert_eq!(matching_lines.count(), 0);
        assert_eq!(matching_lines.percent_scanned(), 0);
        assert_eq!(matching_lines.get(0), None);
        assert_eq!(matching_lines.last(), None);
        assert_eq!(matching_lines.count_above(10), 0);
        assert!(!matching_lines.contains(0));
        assert!(matching_lines.in_positions(0..10).is_empty());
    }

    #[test]
    fn given_two_batches_when_they_are_added_then_their_lines_are_one_list_and_the_percent_is_the_last_one_s()
     {
        let mut matching_lines = make_matching_lines(&[3, 7]);

        matching_lines.add_batch(MatchingLinesBatch {
            matching_lines: vec![12],
            percent_scanned: 62,
        });

        assert_eq!(matching_lines.count(), 3);
        assert_eq!(matching_lines.get(2), Some(12));
        assert_eq!(matching_lines.get(3), None);
        assert_eq!(matching_lines.last(), Some(12));
        assert_eq!(matching_lines.percent_scanned(), 62);
    }

    #[test]
    fn given_matching_lines_when_counting_those_above_a_line_then_a_match_on_the_line_counts_only_at_or_above()
     {
        let matching_lines = make_matching_lines(&[3, 7, 12]);

        assert_eq!(matching_lines.count_above(0), 0);
        assert_eq!(matching_lines.count_above(3), 0);
        assert_eq!(matching_lines.count_above(8), 2);
        assert_eq!(matching_lines.count_above(99), 3);
        assert_eq!(matching_lines.count_at_or_above(0), 0);
        assert_eq!(matching_lines.count_at_or_above(3), 1);
        assert_eq!(matching_lines.count_at_or_above(12), 3);
    }

    #[test]
    fn given_matching_lines_when_asking_whether_lines_match_then_only_those_lines_do() {
        let matching_lines = make_matching_lines(&[3, 7]);

        assert!(matching_lines.contains(3) && matching_lines.contains(7));
        assert!(!matching_lines.contains(4));
    }

    #[test]
    fn given_positions_past_the_last_match_when_taking_lines_in_them_then_they_stop_at_the_last() {
        let matching_lines = make_matching_lines(&[3, 7, 12]);

        assert_eq!(matching_lines.in_positions(1..10), [7, 12]);
        assert!(matching_lines.in_positions(5..10).is_empty());
    }

    #[test]
    fn given_lines_on_both_sides_of_a_block_edge_when_queried_then_they_read_as_one_list() {
        let block_edge = LINES_PER_BLOCK;
        let matching_lines = make_matching_lines(&[block_edge - 1, block_edge, 3 * block_edge]);

        assert_eq!(matching_lines.count(), 3);
        assert_eq!(
            matching_lines.in_positions(0..3),
            [block_edge - 1, block_edge, 3 * block_edge]
        );
        assert_eq!(matching_lines.count_above(block_edge), 1);
        assert_eq!(matching_lines.count_above(2 * block_edge), 2);
        assert!(matching_lines.contains(block_edge));
        assert!(!matching_lines.contains(2 * block_edge));
        assert_eq!(matching_lines.last(), Some(3 * block_edge));
    }

    #[test]
    fn given_a_batch_that_ends_inside_a_block_when_the_next_one_continues_it_then_the_lines_stay_in_order()
     {
        let mut matching_lines = make_matching_lines(&[10, 20]);

        matching_lines.add_batch(MatchingLinesBatch {
            matching_lines: vec![30, LINES_PER_BLOCK + 5],
            percent_scanned: 100,
        });

        assert_eq!(
            matching_lines.in_positions(0..4),
            [10, 20, 30, LINES_PER_BLOCK + 5]
        );
    }

    #[test]
    fn given_every_line_of_several_blocks_when_queried_then_each_position_is_its_own_line() {
        let line_count = 3 * LINES_PER_BLOCK + 100;
        let every_line: Vec<usize> = (0..line_count).collect();

        let matching_lines = make_matching_lines(&every_line);

        assert_eq!(matching_lines.count(), line_count);
        assert_eq!(matching_lines.get(150_000), Some(150_000));
        assert_eq!(matching_lines.count_above(70_000), 70_000);
        assert_eq!(matching_lines.count_at_or_above(70_000), 70_001);
        assert!(matching_lines.contains(line_count - 1));
        assert!(!matching_lines.contains(line_count));
        assert_eq!(matching_lines.last(), Some(line_count - 1));
        assert_eq!(
            matching_lines.in_positions(line_count - 2..line_count + 5),
            [line_count - 2, line_count - 1]
        );
    }

    #[test]
    fn given_sparse_and_dense_blocks_when_queried_then_the_answers_match_a_plain_list() {
        let sparse_lines = (0..LINES_PER_BLOCK).step_by(50);
        let dense_lines = (LINES_PER_BLOCK..3 * LINES_PER_BLOCK).step_by(3);
        let lines: Vec<usize> = sparse_lines.chain(dense_lines).collect();

        let matching_lines = make_matching_lines(&lines);

        assert_eq!(matching_lines.count(), lines.len());
        for position in (0..lines.len()).step_by(97) {
            assert_eq!(matching_lines.get(position), Some(lines[position]));
        }
        for asked_line in (0..3 * LINES_PER_BLOCK + 10).step_by(101) {
            let lines_above = lines.partition_point(|&line| line < asked_line);
            assert_eq!(matching_lines.count_above(asked_line), lines_above);
            assert_eq!(
                matching_lines.contains(asked_line),
                lines.binary_search(&asked_line).is_ok()
            );
        }
    }
}
