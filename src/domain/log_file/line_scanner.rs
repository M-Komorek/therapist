use std::{fs::File, io, ops::ControlFlow, os::unix::fs::FileExt, path::PathBuf, sync::Arc};

use super::LogFileError;

const SCAN_CHUNK_SIZE: usize = 1024 * 1024;

/// Reads the file in chunks of whole lines, each with the number of its first line. A line cut by
/// a read waits for the next one, so nothing that looks at a chunk sees half a line.
pub struct LineScanner {
    path: PathBuf,
    file: Arc<File>,
}

struct LineBuffer {
    bytes: Vec<u8>,
    filled: usize,
}

impl LineScanner {
    pub(super) fn new(path: PathBuf, file: Arc<File>) -> Self {
        Self { path, file }
    }

    pub fn scan(
        &self,
        on_chunk: impl FnMut(&[u8], usize) -> ControlFlow<()>,
    ) -> Result<(), LogFileError> {
        self.scan_in_chunks(SCAN_CHUNK_SIZE, on_chunk)
            .map_err(|source| LogFileError::CannotRead {
                path: self.path.clone(),
                source,
            })
    }

    fn scan_in_chunks(
        &self,
        chunk_size: usize,
        mut on_chunk: impl FnMut(&[u8], usize) -> ControlFlow<()>,
    ) -> io::Result<()> {
        let mut line_buffer = LineBuffer::new(chunk_size);
        let mut byte_offset = 0;
        let mut chunk_first_line = 0;

        loop {
            let bytes_read = line_buffer.read_more(&self.file, byte_offset)?;
            if bytes_read == 0 {
                let last_line = line_buffer.filled_bytes();
                if !last_line.is_empty() {
                    let _ = on_chunk(last_line, chunk_first_line);
                }
                return Ok(());
            }
            byte_offset += bytes_read as u64;
            let Some(whole_lines_length) = line_buffer.whole_lines_length() else {
                continue;
            };
            let whole_lines = &line_buffer.filled_bytes()[..whole_lines_length];
            if on_chunk(whole_lines, chunk_first_line).is_break() {
                return Ok(());
            }
            chunk_first_line += count_newlines(whole_lines);
            line_buffer.remove_whole_lines(whole_lines_length);
        }
    }
}

impl LineBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            bytes: vec![0; capacity],
            filled: 0,
        }
    }

    fn read_more(&mut self, file: &File, byte_offset: u64) -> io::Result<usize> {
        if self.filled == self.bytes.len() {
            self.bytes.resize(self.bytes.len() * 2, 0);
        }
        let bytes_read = file.read_at(&mut self.bytes[self.filled..], byte_offset)?;
        self.filled += bytes_read;
        Ok(bytes_read)
    }

    fn whole_lines_length(&self) -> Option<usize> {
        memchr::memrchr(b'\n', self.filled_bytes()).map(|last_newline| last_newline + 1)
    }

    fn remove_whole_lines(&mut self, whole_lines_length: usize) {
        self.bytes.copy_within(whole_lines_length..self.filled, 0);
        self.filled -= whole_lines_length;
    }

    fn filled_bytes(&self) -> &[u8] {
        &self.bytes[..self.filled]
    }
}

fn count_newlines(text: &[u8]) -> usize {
    memchr::memchr_iter(b'\n', text).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::log_file::tests::open_with;

    fn chunks_of(content: &[u8], chunk_size: usize) -> Vec<(String, usize)> {
        let (_temp_file, log_file) = open_with(content);
        let mut chunks = Vec::new();
        log_file
            .line_scanner()
            .unwrap()
            .scan_in_chunks(chunk_size, |chunk, chunk_first_line| {
                chunks.push((
                    String::from_utf8_lossy(chunk).into_owned(),
                    chunk_first_line,
                ));
                ControlFlow::Continue(())
            })
            .unwrap();
        chunks
    }

    #[test]
    fn given_lines_split_by_reads_when_scanning_then_each_chunk_ends_after_a_newline() {
        let chunks = chunks_of(b"ab ERROR\nxx ERROR\nend\n", 12);

        assert_eq!(
            chunks,
            [
                ("ab ERROR\n".to_owned(), 0),
                ("xx ERROR\n".to_owned(), 1),
                ("end\n".to_owned(), 2)
            ]
        );
    }

    #[test]
    fn given_line_longer_than_a_read_when_scanning_then_it_comes_whole_in_one_chunk() {
        let chunks = chunks_of(b"a\nlong line here\nb\n", 4);

        let chunk_texts: Vec<&str> = chunks.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(chunk_texts.concat(), "a\nlong line here\nb\n");
        assert!(chunks.contains(&("long line here\n".to_owned(), 1)));
    }

    #[test]
    fn given_last_line_without_newline_when_scanning_then_it_comes_as_the_last_chunk() {
        let chunks = chunks_of(b"a\nb\nlast", 64);

        assert_eq!(chunks, [("a\nb\n".to_owned(), 0), ("last".to_owned(), 2)]);
    }

    #[test]
    fn given_empty_file_when_scanning_then_there_are_no_chunks() {
        assert!(chunks_of(b"", 8).is_empty());
    }

    #[test]
    fn given_scan_told_to_stop_when_scanning_then_no_more_chunks_come() {
        let (_temp_file, log_file) = open_with(b"a\nb\nc\n");
        let mut chunk_count = 0;

        log_file
            .line_scanner()
            .unwrap()
            .scan_in_chunks(2, |_chunk, _chunk_first_line| {
                chunk_count += 1;
                ControlFlow::Break(())
            })
            .unwrap();

        assert_eq!(chunk_count, 1);
    }
}
