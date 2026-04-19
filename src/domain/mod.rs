pub mod file;
pub mod indexer;

pub trait LogSource: Send + Sync {
    fn line_count(&self) -> usize;

    #[allow(dead_code)]
    fn get_line(&self, line_idx: usize) -> color_eyre::Result<String>;

    fn get_lines(&self, start: usize, end: usize) -> Vec<String>;
}
