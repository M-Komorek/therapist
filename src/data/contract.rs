pub trait LogSource: Send + Sync {
    #[allow(dead_code)]
    fn get_line(&self, line_idx: usize) -> color_eyre::Result<String>;
    fn get_lines(&self, start: usize, end: usize) -> Vec<String>;

    fn data(&self) -> &[u8];
    fn line_count(&self) -> usize;
    fn memory_usage(&self) -> usize;
}
