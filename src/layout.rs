use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutSlot {
    Unassigned,
    Main,
    Footer,
}

pub struct LayoutManager {
    slots: HashMap<LayoutSlot, Rect>,
}

impl LayoutManager {
    pub fn new(size: Size) -> Self {
        let mut mgr = Self {
            slots: HashMap::new(),
        };
        mgr.compute(size);
        mgr
    }

    pub fn compute(&mut self, size: Size) {
        let area = Rect::new(0, 0, size.width, size.height);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main
                Constraint::Length(1), // Footer
            ])
            .split(area);

        self.slots.insert(LayoutSlot::Unassigned, area);
        self.slots.insert(LayoutSlot::Main, rows[0]);
        self.slots.insert(LayoutSlot::Footer, rows[1]);
    }

    pub fn get_area(&self, slot: LayoutSlot) -> Rect {
        self.slots
            .get(&slot)
            .copied()
            .expect("Unhandled LayoutSlot!")
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Size;

    use super::{LayoutManager, LayoutSlot};

    const TERMINAL_SIZE: Size = Size::new(80, 40);

    #[test]
    fn unassigned_slot_fills_whole_terminal() {
        let mgr = LayoutManager::new(TERMINAL_SIZE);
        let unassigned = mgr.get_area(LayoutSlot::Unassigned);
        assert_eq!(unassigned.height, TERMINAL_SIZE.height);
        assert_eq!(unassigned.width, TERMINAL_SIZE.width);
    }

    #[test]
    fn main_slot_fills_all_rows_except_footer() {
        let mgr = LayoutManager::new(TERMINAL_SIZE);
        let main = mgr.get_area(LayoutSlot::Main);
        assert_eq!(main.height, TERMINAL_SIZE.height - 1);
        assert_eq!(main.width, TERMINAL_SIZE.width);
    }

    #[test]
    fn footer_slot_is_one_row_tall() {
        let mgr = LayoutManager::new(TERMINAL_SIZE);
        let footer = mgr.get_area(LayoutSlot::Footer);
        assert_eq!(footer.height, 1);
        assert_eq!(footer.width, TERMINAL_SIZE.width);
    }

    #[test]
    fn footer_slot_is_at_bottom() {
        let mgr = LayoutManager::new(TERMINAL_SIZE);
        let footer = mgr.get_area(LayoutSlot::Footer);
        assert_eq!(footer.y, TERMINAL_SIZE.height - 1);
    }

    #[test]
    fn compute_updates_slots_on_resize() {
        let mut mgr = LayoutManager::new(TERMINAL_SIZE);
        mgr.compute(Size::new(100, 40));
        let unassigned = mgr.get_area(LayoutSlot::Unassigned);
        assert_eq!(unassigned.height, 40);
        assert_eq!(unassigned.width, 100);
    }
}
