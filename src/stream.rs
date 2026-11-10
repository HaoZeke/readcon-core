//! Frames parsed from a byte stream with about one frame in memory.

use crate::error::ParseError;
use crate::iterators::ConFrameIterator;
use crate::types::ConFrame;
use std::io::{self, Read};
use std::path::Path;

/// Error type of [`ConFrameReader`]: a [`ParseError`], or an input or
/// output error from the source.
pub type StreamError = Box<dyn std::error::Error + Send + Sync>;

/// Bytes requested from the source when the buffer has no whole frame.
pub const DEFAULT_READ_SIZE: usize = 1 << 20;

/// Parses frames from any [`Read`], such as a gzip or Zstandard decoder.
///
/// The buffer keeps the bytes not yet returned and room for one read.
/// Before each read it moves those bytes to its front, and it grows only
/// to fit them plus that read.
/// Each read asks for `read_size` bytes.
/// The request grows to an eighth of the unread bytes when that exceeds
/// `read_size`, so a large frame needs few reads.
/// The capacity therefore stays near the largest frame, with the lines
/// the parser looks at after it, plus one read.
/// For a frame longer than eight reads, the read term is up to an eighth
/// of that frame.
///
/// Frames and errors are those of [`ConFrameIterator::from_bytes`] over
/// the whole stream, up to and including the first error.
/// When the source fails, the reader returns the frames that `from_bytes`
/// returns over the bytes read before the failure, up to its first error,
/// and then the source error.
/// After an error the reader returns `None`.
pub struct ConFrameReader<R> {
    src: R,
    /// Initialized storage, with the unread bytes in `buf[start..end]`.
    buf: Vec<u8>,
    start: usize,
    end: usize,
    read_size: usize,
    eof: bool,
    failed: Option<io::Error>,
    done: bool,
}

impl ConFrameReader<Box<dyn Read + Send>> {
    /// A reader over `path`, decompressed by
    /// [`crate::compression::open_decoded`].
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(crate::compression::open_decoded(path)?))
    }
}

impl<R: Read> ConFrameReader<R> {
    /// A reader that asks `src` for [`DEFAULT_READ_SIZE`] bytes at a time.
    pub fn new(src: R) -> Self {
        Self::with_read_size(src, DEFAULT_READ_SIZE)
    }

    /// Like [`Self::new`], asking the source for `read_size` bytes (at
    /// least 1) whenever the buffer has no whole frame.
    pub fn with_read_size(src: R, read_size: usize) -> Self {
        Self {
            src,
            buf: Vec::new(),
            start: 0,
            end: 0,
            read_size: read_size.max(1),
            eof: false,
            failed: None,
            done: false,
        }
    }

    /// Skip one frame without parsing its atoms, as
    /// [`ConFrameIterator::forward`] does.
    pub fn forward(&mut self) -> Option<Result<(), StreamError>> {
        self.step(|it| it.forward().map(|r| r.map(|()| ((), it.byte_pos()))))
    }

    /// Run `op` on a fresh iterator over the buffered whole lines.
    /// Its result is final once the stream has ended, or once the lines
    /// after its cursor fix every look-ahead decision.
    /// Otherwise read more.
    ///
    /// After a source error no more bytes arrive, so the buffer is parsed
    /// as a whole input.
    /// A frame from it is returned, and an error or the end of the buffer
    /// gives way to the source error.
    fn step<T>(
        &mut self,
        mut op: impl FnMut(&mut ConFrameIterator<'_>) -> Option<Result<(T, usize), ParseError>>,
    ) -> Option<Result<T, StreamError>> {
        let mut cut = false;
        while !self.done {
            let ended = self.eof || self.failed.is_some();
            let rest = &self.buf[self.start..self.end];
            let text = if ended {
                rest
            } else {
                memchr::memrchr(b'\n', rest).map_or(&rest[..0], |i| &rest[..=i])
            };
            // Once a frame was cut by the buffer end, a skip walk checks
            // each longer buffer first, so a frame larger than one read is
            // not parsed in full once per read.
            if ended || !cut || {
                let mut probe = ConFrameIterator::from_bytes(text);
                if self.start > 0 {
                    probe.allow_mid_file_bom();
                }
                probe.forward();
                settled(text, probe.byte_pos())
            } {
                let mut it = if ended {
                    ConFrameIterator::from_bytes(text)
                } else {
                    ConFrameIterator::from_prefix(text)
                };
                if self.start > 0 {
                    it.allow_mid_file_bom();
                }
                let result = op(&mut it);
                // A legacy velocity block that is cut between its label and
                // its last row is not finished. More bytes can still attach
                // it to this frame. The error is final only at end of input,
                // where the same bytes are parsed as a whole file. A failed
                // read sets `ended`, so the source error is the result below
                // once that whole-file parse does not yield a frame.
                if !ended && incomplete_prefix(&result) {
                    cut = true;
                    self.fill();
                    continue;
                }
                if ended || settled(text, it.byte_pos()) {
                    match result {
                        Some(Ok((value, used))) => {
                            self.start += used;
                            return Some(Ok(value));
                        }
                        other if self.failed.is_none() => {
                            self.done = true;
                            return other.map(|r| r.map(|(value, _)| value).map_err(Into::into));
                        }
                        _ => {}
                    }
                }
            }
            cut = true;
            if let Some(e) = self.failed.take() {
                self.done = true;
                return Some(Err(e.into()));
            }
            self.fill();
        }
        None
    }

    /// Move the unread bytes to the front, and grow the buffer to fit them
    /// plus one read.
    /// Then read until that read is full, the source ends, or it fails.
    /// The bytes read before a failure are kept.
    fn fill(&mut self) {
        self.buf.copy_within(self.start..self.end, 0);
        self.end -= self.start;
        self.start = 0;
        let target = self.end + self.read_size.max(self.end / 8);
        if target > self.buf.len() {
            self.buf.reserve_exact(target - self.buf.len());
            self.buf.resize(target, 0);
        }
        while self.end < target {
            match self.src.read(&mut self.buf[self.end..target]) {
                Ok(0) => {
                    self.eof = true;
                    return;
                }
                Ok(n) => self.end += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    self.failed = Some(e);
                    return;
                }
            }
        }
    }
}

/// An incomplete frame or section on a prefix. More bytes may complete it.
fn incomplete_prefix<T>(
    result: &Option<Result<(T, usize), ParseError>>,
) -> bool {
    matches!(
        result,
        Some(Err(
            ParseError::IncompleteSection(_)
                | ParseError::IncompleteFrame
                | ParseError::IncompleteHeader
        ))
    )
}

/// Whether `text` has, after `pos`, any whitespace-only lines, then a
/// line with text and one more line.
/// The parser reads no further past its cursor.
/// A scan for trailing whitespace stops at the line with text.
/// A section probe after a peeked blank reads two lines.
/// It stops early when the first of them is whitespace.
/// `text` ends in a newline, so every piece is a whole line.
fn settled(text: &[u8], pos: usize) -> bool {
    let mut lines = text[pos.min(text.len())..].split_inclusive(|&b| b == b'\n');
    lines.any(|line| !line.iter().all(u8::is_ascii_whitespace)) && lines.next().is_some()
}

impl<R: Read> Iterator for ConFrameReader<R> {
    type Item = Result<ConFrame, StreamError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.step(|it| it.next_with_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zero-atom frame of `len` bytes, padded by its user line.
    fn zero_atom_frame(len: usize) -> Vec<u8> {
        let tail = "\n{\"con_spec_version\":2}\n10.000000 10.000000 10.000000\n\
                    90.000000 90.000000 90.000000\n\n\n0\n\n\n";
        let mut frame = vec![b'U'; len - tail.len()];
        frame.extend_from_slice(tail.as_bytes());
        frame
    }

    /// The buffer after the first frame of `frame` then `after`.
    fn after_first(frame: &[u8], after: &[u8]) -> (usize, usize) {
        let text = [frame, after].concat();
        let mut reader = ConFrameReader::new(&text[..]);
        let want = ConFrameIterator::from_bytes(frame).next().unwrap().unwrap();
        assert_eq!(reader.next().unwrap().unwrap(), want);
        assert_eq!(reader.start, frame.len());
        (reader.buf.len(), reader.buf.capacity())
    }

    #[test]
    fn a_frame_over_one_read_grows_the_buffer_to_about_one_frame_and_one_read() {
        let small = zero_atom_frame(100).repeat(2);
        for len in [(1 << 20) + 57, (1 << 21) + 57] {
            let frame = zero_atom_frame(len);
            for after in [&small[..], &[][..]] {
                let (filled, capacity) = after_first(&frame, after);
                assert!(filled <= capacity);
                let message = format!(
                    "frame {len}, {} bytes after it: capacity {capacity}",
                    after.len()
                );
                assert!(capacity <= len + DEFAULT_READ_SIZE, "{message}");
            }
        }
    }

    /// Reproducer for the one-byte stream disagreeing with the string reader.
    ///
    /// `resources/test/tiny_cuh2.convel` is one frame. After the coordinate
    /// block a blank line opens a legacy velocity block: a symbol, then
    /// `Velocities of Component N`, then five-field rows. Mutation 0 of the
    /// differential appends a form-feed line (`\n\u{000c}\n`), which is
    /// trailing whitespace and does not add a frame (CON-frame.1).
    ///
    /// A one-byte read used to end the frame at the blank while the velocity
    /// rows were still outside the buffer. The next chunk then started a
    /// frame at `Velocities` and rejected it as a float. The block belongs
    /// to the frame when its labels and rows are present. A chunk boundary
    /// inside those rows waits. A row that is present and not five fields
    /// is not a block (CON-frame.7) and stays a new frame.
    #[test]
    fn one_byte_stream_keeps_the_legacy_velocity_block() {
        let mut bytes = std::fs::read("resources/test/tiny_cuh2.convel").unwrap();
        bytes.extend_from_slice(b"\n\x0c\n");
        let expect: Vec<_> = crate::iterators::ConFrameIterator::from_bytes(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(expect.len(), 1);
        assert!(expect[0].atom_data.iter().all(|atom| atom.velocity().is_some()));
        for size in [1usize, 3, 61, DEFAULT_READ_SIZE] {
            let got: Vec<_> = ConFrameReader::with_read_size(std::io::Cursor::new(&bytes), size)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(got, expect, "read size {size}");
        }
    }

    /// One `Interrupted`, then the inner reader.
    struct InterruptOnce<R> {
        inner: R,
        tripped: bool,
    }

    impl<R: std::io::Read> std::io::Read for InterruptOnce<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.tripped {
                self.tripped = true;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "again",
                ));
            }
            self.inner.read(buf)
        }
    }

    #[test]
    fn an_interrupted_read_is_retried_and_a_later_frame_is_probed() {
        let first = zero_atom_frame(200);
        let second = zero_atom_frame(4000);
        let bytes = [first.clone(), second].concat();
        let expect: Vec<_> = ConFrameIterator::from_bytes(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(expect.len(), 2);
        let src = InterruptOnce {
            inner: std::io::Cursor::new(bytes),
            tripped: false,
        };
        let got: Vec<_> = ConFrameReader::with_read_size(src, 64)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(got, expect);
    }

    #[test]
    fn a_frame_of_many_reads_grows_the_buffer_by_an_eighth_at_most() {
        let read_size = 4096;
        let len = 64 * read_size + 57;
        let text = [zero_atom_frame(len), zero_atom_frame(100)]
            .concat()
            .repeat(2);
        let mut reader = ConFrameReader::with_read_size(&text[..], read_size);
        for _ in 0..4 {
            assert!(reader.next().unwrap().is_ok());
            assert!(reader.buf.capacity() <= len + len / 8 + read_size);
        }
        assert!(reader.next().is_none());
    }

    /// A source that yields `data`, then fails.
    struct FailAfter {
        data: Vec<u8>,
        pos: usize,
    }

    impl Read for FailAfter {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Err(io::Error::new(io::ErrorKind::Other, "source failed"));
            }
            let n = (self.data.len() - self.pos).min(buf.len());
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    /// A read that fails inside a legacy velocity block is the source error.
    /// The bytes already read are not a finished file, so they are not
    /// reported as IncompleteFrame.
    #[test]
    fn a_failed_read_inside_a_velocity_block_is_the_source_error() {
        let file = std::fs::read("resources/test/tiny_cuh2.convel").unwrap();
        let label = file
            .windows(b"Velocities of Component".len())
            .position(|w| w == b"Velocities of Component")
            .expect("velocity label");
        let prefix = file[..label + b"Velocities of Component 1\n".len()].to_vec();
        let err = ConFrameReader::with_read_size(FailAfter { data: prefix, pos: 0 }, 32)
            .next()
            .expect("a result")
            .expect_err("source error");
        assert!(err.to_string().contains("source failed"), "{err}");
    }

    /// The second frame of a small read is probed after the first frame
    /// has been consumed, including when its velocity block is still cut.
    #[test]
    fn a_later_frame_allows_a_mid_file_bom_after_a_cut_block() {
        let first = zero_atom_frame(200);
        let second = std::fs::read("resources/test/tiny_cuh2.convel").unwrap();
        let bytes = [first.clone(), second].concat();
        let expect: Vec<_> = ConFrameIterator::from_bytes(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(expect.len(), 2);
        let got: Vec<_> = ConFrameReader::with_read_size(std::io::Cursor::new(bytes), 48)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(got, expect);
    }
}
