use std::{fs::File, io, ops::ControlFlow, os::unix::fs::FileExt, path::PathBuf, sync::Arc};

use super::{LogFileError, bytes_left, read_chunks};

const SCAN_CHUNK_SIZE: usize = 1024 * 1024;
const SKIP_CHUNK_SIZE: usize = 64 * 1024;

/// Reads the file in chunks of whole lines, each with the number of its first line. A line cut by
/// a read waits for the next one, so nothing that looks at a chunk sees half a line. The one
/// exception is a line longer than the read buffer (1 MiB): its start comes alone as a chunk without
/// a newline, and the rest of it is skipped, so memory stays bounded.
pub struct LineScanner {
    path: PathBuf,
    file: Arc<File>,
    file_size: u64,
}

struct LineBuffer {
    bytes: Vec<u8>,
    filled: usize,
}

impl LineScanner {
    pub(super) fn new(path: PathBuf, file: Arc<File>, file_size: u64) -> Self {
        Self {
            path,
            file,
            file_size,
        }
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
            if line_buffer.is_full() {
                if on_chunk(line_buffer.filled_bytes(), chunk_first_line).is_break() {
                    return Ok(());
                }
                byte_offset = self.next_newline_offset(byte_offset)?;
                line_buffer.clear();
            }
            let bytes_read = line_buffer.read_more(&self.file, byte_offset, self.file_size)?;
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

    fn next_newline_offset(&self, byte_offset: u64) -> io::Result<u64> {
        let mut chunk_offset = byte_offset;
        let mut newline_offset = self.file_size;

        read_chunks(
            &self.file,
            byte_offset..self.file_size,
            SKIP_CHUNK_SIZE,
            |chunk| match memchr::memchr(b'\n', chunk) {
                Some(newline_position) => {
                    newline_offset = chunk_offset + newline_position as u64;
                    ControlFlow::Break(())
                }
                None => {
                    chunk_offset += chunk.len() as u64;
                    ControlFlow::Continue(())
                }
            },
        )?;
        Ok(newline_offset)
    }
}

impl LineBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            bytes: vec![0; capacity],
            filled: 0,
        }
    }

    fn is_full(&self) -> bool {
        self.filled == self.bytes.len()
    }

    fn clear(&mut self) {
        self.filled = 0;
    }

    fn read_more(&mut self, file: &File, byte_offset: u64, read_end: u64) -> io::Result<usize> {
        let room_left = self.bytes.len() - self.filled;
        let bytes_wanted = room_left.min(bytes_left(byte_offset, read_end));
        let read_buffer = &mut self.bytes[self.filled..self.filled + bytes_wanted];
        let bytes_read = file.read_at(read_buffer, byte_offset)?;
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
    use std::io::Write;

    use super::*;
    use crate::domain::log_file::{LogFile, tests::open_with};

    fn chunks_of(content: &[u8], chunk_size: usize) -> Vec<(String, usize)> {
        let (_temp_file, log_file) = open_with(content);
        scanned_chunks(&log_file, chunk_size)
    }

    fn scanned_chunks(log_file: &LogFile, chunk_size: usize) -> Vec<(String, usize)> {
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
    fn given_line_that_fits_the_buffer_only_after_a_read_when_scanning_then_it_comes_whole() {
        let chunks = chunks_of(b"a\nlong\nb\n", 6);

        assert_eq!(
            chunks,
            [
                ("a\n".to_owned(), 0),
                ("long\n".to_owned(), 1),
                ("b\n".to_owned(), 2)
            ]
        );
    }

    #[test]
    fn given_line_longer_than_the_buffer_when_scanning_then_only_its_start_comes_and_the_numbering_holds()
     {
        let chunks = chunks_of(b"a\nlong line here\nb\n", 4);

        assert_eq!(
            chunks,
            [
                ("a\n".to_owned(), 0),
                ("long".to_owned(), 1),
                ("\nb\n".to_owned(), 1)
            ]
        );
    }

    #[test]
    fn given_last_line_longer_than_the_buffer_without_newline_when_scanning_then_only_its_start_comes()
     {
        let chunks = chunks_of(b"a\nlong line here", 4);

        assert_eq!(chunks, [("a\n".to_owned(), 0), ("long".to_owned(), 1)]);
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
    fn given_file_grown_after_opening_when_scanning_then_only_the_opened_size_is_scanned() {
        let (mut temp_file, log_file) = open_with(b"a\nb");

        temp_file.write_all(b"c\nERROR\n").unwrap();

        assert_eq!(
            scanned_chunks(&log_file, 64),
            [("a\n".to_owned(), 0), ("b".to_owned(), 1)]
        );
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
