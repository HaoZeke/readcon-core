//=============================================================================
// The Public API - A clean iterator for users of our library
//=============================================================================

use crate::parser::{LineStream, parse_declared_sections};
use crate::{error, types};
use std::path::Path;

/// memchr-backed line cursor for the full parse path (not only frame skip).
///
/// Profile note: `str::Lines` + `Peekable` showed up under
/// `ConFrameIterator::next` on multi-atom multi-frame workloads. One cursor
/// serves `next` / `peek` / `forward_fast` so skip and full parse share the
/// same O(1) newline scan rather than two desynchronized views of the buffer.
pub struct MemchrLines<'a> {
    bytes: &'a [u8],
    pos: usize,
    peeked: Option<&'a str>,
    /// When the buffer was not already a `&str`, header lines are checked
    /// as they are tokenized. Coordinate rows stay on the byte parser.
    check_utf8: bool,
    utf8_bad: bool,
    /// False while a stream may still append bytes after this buffer.
    input_complete: bool,
    /// Absolute offsets of every `\n`. Set only for a frame-start scan.
    /// `None` keeps the per-line `memchr` path used by a full parse.
    newlines: Option<Vec<usize>>,
    /// Index of the line that starts at `pos` when `newlines` is in use.
    line_no: usize,
}

impl<'a> MemchrLines<'a> {
    pub fn new(text: &'a str) -> Self {
        Self::from_parts(text.as_bytes(), false)
    }

    /// Cursor over bytes that are not yet known to be UTF-8.
    ///
    /// ASCII coordinate rows are accepted by the numeric scan. Text lines
    /// are validated when they are materialized, so there is no separate
    /// whole-buffer UTF-8 pass.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self::from_parts(bytes, true)
    }

    fn from_parts(bytes: &'a [u8], check_utf8: bool) -> Self {
        Self {
            bytes,
            pos: 0,
            peeked: None,
            check_utf8,
            utf8_bad: false,
            input_complete: true,
            newlines: None,
            line_no: 0,
        }
    }

    #[inline]
    fn read_one(&mut self) -> Option<&'a str> {
        if self.newlines.is_some() {
            return self.read_one_indexed();
        }
        if self.utf8_bad || self.pos >= self.bytes.len() {
            return None;
        }
        let rest = &self.bytes[self.pos..];
        let (line_bytes, advance) = match memchr::memchr(b'\n', rest) {
            Some(i) => (&rest[..i], i + 1),
            None => (rest, rest.len()),
        };
        let trimmed = if line_bytes.last() == Some(&b'\r') {
            &line_bytes[..line_bytes.len() - 1]
        } else {
            line_bytes
        };
        if self.check_utf8 {
            match std::str::from_utf8(trimmed) {
                Ok(s) => {
                    self.pos += advance;
                    self.line_no += 1;
                    Some(s)
                }
                Err(_) => {
                    // Stay on the rejected line. Moving past it makes a
                    // whitespace tail look like a clean end of file.
                    self.utf8_bad = true;
                    self.line_no = self.bytes[..self.pos]
                        .iter()
                        .filter(|byte| **byte == b'\n')
                        .count()
                        + 1;
                    None
                }
            }
        } else {
            self.pos += advance;
            self.line_no += 1;
            // SAFETY: source was `&str`; line is a UTF-8 prefix cut on ASCII `\n`/`\r`.
            Some(unsafe { std::str::from_utf8_unchecked(trimmed) })
        }
    }

    #[inline]
    pub fn next_line(&mut self) -> Option<&'a str> {
        if let Some(p) = self.peeked.take() {
            return Some(p);
        }
        self.read_one()
    }

    #[inline]
    pub fn peek_line(&mut self) -> Option<&'a str> {
        if self.peeked.is_none() {
            self.peeked = self.read_one();
        }
        self.peeked
    }

    /// Drop any peek buffer (required before bulk cursor advances).
    fn clear_peek(&mut self) {
        if let Some(p) = self.peeked.take() {
            // Rewind pos to the start of the peeked line.
            let start = p.as_ptr() as usize - self.bytes.as_ptr() as usize;
            self.pos = start;
            if let Some(nl) = self.newlines.as_deref() {
                self.line_no = line_no_for_pos(nl, self.bytes.len(), start);
            }
        }
    }

    /// One line from the precomputed newline index.
    ///
    /// The cursor must sit on a line start (or at the end). The bytes of
    /// the line are the same slice `memchr` would return, including a line
    /// whose body crosses a chunk boundary: only the `\n` is indexed.
    fn read_one_indexed(&mut self) -> Option<&'a str> {
        if self.utf8_bad || self.pos >= self.bytes.len() {
            return None;
        }
        let len = self.bytes.len();
        let line_no = {
            let nl = self.newlines.as_deref().unwrap();
            if self.pos == line_start_pos(nl, len, self.line_no) {
                self.line_no
            } else {
                line_no_for_pos(nl, len, self.pos)
            }
        };
        let span = indexed_span(self.newlines.as_deref().unwrap(), self.bytes, line_no)?;
        let trimmed = &self.bytes[span.start..span.content_end];
        if self.check_utf8 {
            match std::str::from_utf8(trimmed) {
                Ok(s) => {
                    self.pos = span.next_pos;
                    self.line_no = line_no + 1;
                    Some(s)
                }
                Err(_) => {
                    self.utf8_bad = true;
                    self.line_no = line_no;
                    None
                }
            }
        } else {
            self.pos = span.next_pos;
            self.line_no = line_no + 1;
            // SAFETY: source was `&str`; the span is a UTF-8 line cut on ASCII `\n`/`\r`.
            Some(unsafe { std::str::from_utf8_unchecked(trimmed) })
        }
    }

    fn utf8_error(&self) -> Option<error::ParseError> {
        self.utf8_bad.then(invalid_utf8)
    }

    /// Keep `err` unless this cursor already rejected a line as UTF-8.
    fn prefer_utf8(&self, err: error::ParseError) -> error::ParseError {
        if self.utf8_bad { invalid_utf8() } else { err }
    }
}

fn invalid_utf8() -> error::ParseError {
    error::ParseError::ValidationError("invalid UTF-8".into())
}

impl<'a> Iterator for MemchrLines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.next_line()
    }
}

impl<'a> LineStream<'a> for MemchrLines<'a> {
    #[inline]
    fn next_line(&mut self) -> Option<&'a str> {
        MemchrLines::next_line(self)
    }
    #[inline]
    fn peek_line(&mut self) -> Option<&'a str> {
        MemchrLines::peek_line(self)
    }
    #[inline]
    fn next_coord_row(
        &mut self,
        spec_version: u32,
        default_id: u64,
    ) -> Result<([f64; 3], [bool; 3], u64), crate::error::ParseError> {
        if let Some(p) = self.peeked.take() {
            self.line_no += 1;
            return crate::parser::parse_coord_row_at(p.as_bytes(), 0, spec_version, default_id)
                .map(|(row, _)| row);
        }
        let rest = &self.bytes[self.pos..];
        let nl = memchr::memchr(b'\n', rest);
        let mut end = nl.unwrap_or(rest.len());
        if end > 0 && rest[end - 1] == b'\r' {
            end -= 1;
        }
        let line = &rest[..end];
        if rest.is_empty() {
            return Err(crate::error::ParseError::IncompleteFrame);
        }
        // A line that is present but holds no token is a row of the wrong
        // length (CON-num.9). End of input, with no line, is IncompleteFrame.
        if line.iter().all(|b| *b == b' ' || *b == b'\t') {
            self.line_no += 1;
            self.pos += nl.map(|i| i + 1).unwrap_or(rest.len());
            return Err(crate::error::ParseError::InvalidVectorLength {
                expected: 5,
                found: 0,
            });
        }
        let parsed =
            crate::parser::parse_coord_fields(line, 0, spec_version, Some(default_id));
        self.line_no += 1;
        // `parse_coord_fields` consumes trailing spaces and tabs, and it
        // returns `Err` when any other byte remains. `Ok` is a finished row.
        match parsed {
            Ok((row, _)) => {
                self.pos += nl.map(|i| i + 1).unwrap_or(rest.len());
                Ok(row)
            }
            Err(err) => Err(err),
        }
    }
    fn bytes_remaining(&mut self) -> Option<usize> {
        if !self.input_complete {
            return None;
        }
        let peeked = self.peeked.as_ref().map(|line| line.len()).unwrap_or(0);
        Some(self.bytes.len().saturating_sub(self.pos).saturating_add(peeked))
    }
    fn remainder_is_whitespace(&mut self) -> bool {
        if self.utf8_bad {
            return false;
        }
        if let Some(line) = self.peek_line()
            && !line.trim().is_empty()
        {
            return false;
        }
        // `peek_line` may have rejected a line. That is a decode error,
        // including when every byte after it is ASCII whitespace.
        if self.utf8_bad {
            return false;
        }
        self.bytes[self.pos..]
            .iter()
            .all(|b| b.is_ascii_whitespace())
    }
    fn blank_line_opens_section(&mut self) -> bool {
        // `peek_line` has already moved `pos` to the line after the peek.
        let Some(peeked) = self.peeked else {
            return false;
        };
        if !peeked.trim().is_empty() {
            return false;
        }
        match probe_section(&self.bytes[self.pos..]) {
            SectionProbe::Opens => true,
            SectionProbe::Closed => false,
            // Leave the rejected bytes on the cursor. `prefer_utf8` then
            // reports the decode error instead of a missing section.
            SectionProbe::InvalidUtf8 => {
                self.utf8_bad = true;
                false
            }
        }
    }

    fn blank_opens_velocity_block(&mut self) -> bool {
        match self.peek_line() {
            Some(line) if line.trim().is_empty() => self.blank_line_opens_section(),
            _ => true,
        }
    }

    fn legacy_velocity_rows_complete(&mut self, counts: &[usize]) -> bool {
        // `peek_line` has already moved `pos` past the blank separator.
        let mut rest = &self.bytes[self.pos..];
        for (type_idx, &count) in counts.iter().enumerate() {
            let Some((_, after_symbol)) = split_raw_line(rest) else {
                return false;
            };
            let Some((raw_label, after_label)) = split_raw_line(after_symbol) else {
                return false;
            };
            rest = after_label;
            let Ok(label) = std::str::from_utf8(strip_cr(raw_label)) else {
                return false;
            };
            if !crate::parser::line_has_component_label(label, (type_idx + 1) as u32) {
                return false;
            }
            for _ in 0..count {
                let Some((raw_row, after_row)) = split_raw_line(rest) else {
                    return false;
                };
                rest = after_row;
                let Ok(row) = std::str::from_utf8(strip_cr(raw_row)) else {
                    return false;
                };
                if crate::parser::token_count(row) != 5 {
                    return false;
                }
            }
        }
        true
    }
}

fn split_raw_line(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    if bytes.is_empty() {
        return None;
    }
    match memchr::memchr(b'\n', bytes) {
        Some(i) => Some((&bytes[..i], &bytes[i + 1..])),
        None => Some((bytes, &[])),
    }
}

fn strip_cr(line: &[u8]) -> &[u8] {
    if line.last() == Some(&b'\r') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

enum SectionProbe {
    Opens,
    Closed,
    InvalidUtf8,
}

/// A blank line opens a section when the next line is a symbol and the
/// line after that is a component label (CON-file.5, CON-header.4).
fn probe_section(after_blank: &[u8]) -> SectionProbe {
    let Some((raw1, after1)) = split_raw_line(after_blank) else {
        return SectionProbe::Closed;
    };
    let line1 = strip_cr(raw1);
    if line1.iter().all(|b| matches!(b, b' ' | b'\t')) {
        // A blank symbol line still opens the block when the next line is
        // the component label.
        let Some((raw_label, _)) = split_raw_line(after1) else {
            return SectionProbe::Closed;
        };
        let Ok(label) = std::str::from_utf8(strip_cr(raw_label)) else {
            return SectionProbe::InvalidUtf8;
        };
        return if crate::parser::line_has_component_label(label, 1) {
            SectionProbe::Opens
        } else {
            SectionProbe::Closed
        };
    }
    let Ok(text1) = std::str::from_utf8(line1) else {
        return SectionProbe::InvalidUtf8;
    };
    if text1.trim_start().starts_with('{') {
        return SectionProbe::Closed;
    }
    let Some((raw2, _)) = split_raw_line(after1) else {
        return SectionProbe::Closed;
    };
    let Ok(text2) = std::str::from_utf8(strip_cr(raw2)) else {
        return SectionProbe::InvalidUtf8;
    };
    if crate::parser::line_has_component_label(text2, 1) {
        SectionProbe::Opens
    } else {
        SectionProbe::Closed
    }
}

/// How far a legacy velocity block has got in this buffer.
///
/// `after_blank` begins at the first line after the blank separator.
enum LegacyVelocity {
    /// A present line shows this is not a velocity block (CON-frame.7).
    NotABlock,
    /// Nothing present contradicts a block, and a line is still missing.
    /// On a prefix those bytes may arrive. At EOF only an opened block,
    /// one whose component label was seen, is an error.
    NeedsMore { opened: bool },
    Complete,
}

/// A line that is present and is not a velocity label, or a row that is
/// present and does not have five fields, cannot become a block when more
/// bytes arrive (CON-frame.7). A missing line can.
fn legacy_velocity_prefix(after_blank: &[u8], counts: &[usize]) -> LegacyVelocity {
    let mut rest = after_blank;
    let mut opened = false;
    for (type_idx, &count) in counts.iter().enumerate() {
        let Some((raw_symbol, after_symbol)) = take_full_line(rest) else {
            return LegacyVelocity::NeedsMore { opened };
        };
        rest = after_symbol;
        let symbol = strip_cr(raw_symbol);
        if symbol.iter().all(|b| b.is_ascii_whitespace()) || symbol.first() == Some(&b'{') {
            return LegacyVelocity::NotABlock;
        }
        let Some((raw_label, after_label)) = take_full_line(rest) else {
            return LegacyVelocity::NeedsMore { opened };
        };
        rest = after_label;
        let Ok(label) = std::str::from_utf8(strip_cr(raw_label)) else {
            return LegacyVelocity::NotABlock;
        };
        if !crate::parser::line_has_component_label(label, (type_idx + 1) as u32) {
            return LegacyVelocity::NotABlock;
        }
        opened = true;
        for _ in 0..count {
            let Some((raw_row, after_row)) = take_full_line(rest) else {
                return LegacyVelocity::NeedsMore { opened };
            };
            rest = after_row;
            let Ok(row) = std::str::from_utf8(strip_cr(raw_row)) else {
                return LegacyVelocity::NotABlock;
            };
            if crate::parser::token_count(row) != 5 {
                return LegacyVelocity::NotABlock;
            }
        }
    }
    LegacyVelocity::Complete
}

/// A whole line, including its terminating newline. A partial line at the
/// end of a prefix is not whole: more bytes can still extend it.
fn take_full_line(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    if bytes.is_empty() {
        return None;
    }
    let i = memchr::memchr(b'\n', bytes)?;
    Some((&bytes[..i], &bytes[i + 1..]))
}

/// Every component of a legacy velocity block has a `velocities` label and
/// five fields on each row (CON-frame.5).
fn legacy_rows_complete(after_blank: &[u8], counts: &[usize]) -> bool {
    let mut rest = after_blank;
    for (type_idx, &count) in counts.iter().enumerate() {
        let Some((_, after_symbol)) = split_raw_line(rest) else {
            return false;
        };
        let Some((raw_label, after_label)) = split_raw_line(after_symbol) else {
            return false;
        };
        rest = after_label;
        let Ok(label) = std::str::from_utf8(strip_cr(raw_label)) else {
            return false;
        };
        if !crate::parser::line_has_component_label(label, (type_idx + 1) as u32) {
            return false;
        }
        for _ in 0..count {
            let Some((raw_row, after_row)) = split_raw_line(rest) else {
                return false;
            };
            rest = after_row;
            let Ok(row) = std::str::from_utf8(strip_cr(raw_row)) else {
                return false;
            };
            if crate::parser::token_count(row) != 5 {
                return false;
            }
        }
    }
    true
}

/// `None` is the legacy file: line 2 does not declare `sections`.
/// `Some` is that declaration, including an empty array.
fn declared_sections(line2: &str) -> Option<Vec<String>> {
    let trimmed = line2.trim_matches([' ', '\t']);
    if !trimmed.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;
    if !obj.contains_key("sections") {
        return None;
    }
    let names = obj
        .get("sections")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(names)
}

/// An iterator that lazily parses simulation frames from a `.con` or `.convel`
/// file's contents.
///
/// This struct wraps a memchr line cursor over the file buffer and, upon each
/// iteration, attempts to parse a complete `ConFrame`. A blank line after the
/// coordinates is a velocity section only when it begins a complete velocity
/// block. Otherwise that line is line 1 of the next frame, and it may be empty.
///
/// The iterator yields items of type `Result<ConFrame, ParseError>`, allowing for
/// robust error handling for each frame.
pub struct ConFrameIterator<'a> {
    pub(crate) lines: MemchrLines<'a>,
    check_bom: bool,
}

impl<'a> ConFrameIterator<'a> {
    /// 1-based line of the read position after the last line this iterator consumed.
    pub fn cursor_line(&self) -> u32 {
        self.lines.line_no as u32
    }

    /// Same value as [`Self::cursor_line`].
    pub fn lines_read(&self) -> u32 {
        self.cursor_line()
    }

    /// A slice that is not the start of the file may begin with U+FEFF.
    /// That byte is text of the line, not a file-level byte-order mark.
    pub(crate) fn allow_mid_file_bom(&mut self) {
        self.check_bom = false;
    }

    /// Creates a new `ConFrameIterator` from a string slice of the entire file.
    ///
    /// # Arguments
    ///
    /// * `file_contents` - A string slice containing the text of one or more `.con` frames.
    pub fn new(file_contents: &'a str) -> Self {
        ConFrameIterator {
            lines: MemchrLines::new(file_contents),
            check_bom: true,
        }
    }

    /// Like [`Self::new`], but reading from byte `pos` of `file_contents`.
    ///
    /// `pos` must be 0 or follow a `\n`. The iterator keeps no state from
    /// earlier frames, so skipping from `pos` sees what a scan sees there.
    pub(crate) fn starting_at(file_contents: &'a str, pos: usize) -> Self {
        debug_assert!(pos == 0 || file_contents.as_bytes()[pos - 1] == b'\n');
        let mut it = Self::new(file_contents);
        it.lines.pos = pos;
        it
    }

    /// Like [`Self::new`] for a buffer that has not been UTF-8 validated.
    ///
    /// Text lines are checked as they are tokenized. Numeric rows reject a
    /// non-ASCII byte during the fused parse, which is the UTF-8 check for
    /// those lines.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        ConFrameIterator {
            lines: MemchrLines::from_bytes(bytes),
            check_bom: true,
        }
    }

    /// A buffer that is only a prefix of the input. Atom-count checks wait
    /// until the stream has ended, so a short read is not `InvalidCount`.
    pub fn from_prefix(bytes: &'a [u8]) -> Self {
        let mut lines = MemchrLines::from_bytes(bytes);
        lines.input_complete = false;
        ConFrameIterator {
            lines,
            check_bom: true,
        }
    }

    /// Bulk-skips `n` lines from the shared memchr cursor.
    ///
    /// A final line with no trailing newline still counts when it is the
    /// last line this call needs and its bytes are present. The full
    /// parser accepts that line. Further lines after it are missing.
    fn advance_lines(&mut self, n: usize) -> Result<(), error::ParseError> {
        self.lines.clear_peek();
        if self.lines.newlines.is_some() {
            return self.advance_lines_indexed(n);
        }
        for i in 0..n {
            let rest = &self.lines.bytes[self.lines.pos..];
            match memchr::memchr(b'\n', rest) {
                Some(pos) => self.lines.pos += pos + 1,
                None => {
                    if rest.is_empty() {
                        return Err(error::ParseError::IncompleteFrame);
                    }
                    self.lines.pos = self.lines.bytes.len();
                    if i + 1 == n {
                        return Ok(());
                    }
                    return Err(error::ParseError::IncompleteFrame);
                }
            }
        }
        Ok(())
    }

    /// [`Self::advance_lines`] for `n` lines expected to end at `end`.
    ///
    /// When `end` follows a `\n` and exactly `n` newlines lie before it,
    /// the per-line search would stop at `end` too, so one vectorized
    /// count replaces it. Any other case takes the per-line search.
    fn advance_lines_to(&mut self, n: usize, end: Option<usize>) -> Result<(), error::ParseError> {
        // The newline index jumps by line count. `end` does not change
        // where that jump lands; it only shortcuts the per-line search.
        if self.lines.newlines.is_some() {
            return self.advance_lines(n);
        }
        self.lines.clear_peek();
        let pos = self.lines.pos;
        let bytes = self.lines.bytes;
        if let Some(end) = end
            && pos < end
            && end <= bytes.len()
            && bytes[end - 1] == b'\n'
            && memchr::memchr_iter(b'\n', &bytes[pos..end]).count() == n
        {
            self.lines.pos = end;
            return Ok(());
        }
        self.advance_lines(n)
    }

    /// [`Self::advance_lines`] using the stitched newline index.
    ///
    /// A missing line and a final line with no `\n` land on the same byte
    /// as the `memchr` loop. The index is not consulted for a full parse.
    fn advance_lines_indexed(&mut self, n: usize) -> Result<(), error::ParseError> {
        let len = self.lines.bytes.len();
        let step = {
            let nl = self.lines.newlines.as_deref().unwrap();
            let nlines = line_count(nl, len);
            let line_no = if self.lines.pos == line_start_pos(nl, len, self.lines.line_no) {
                self.lines.line_no
            } else {
                line_no_for_pos(nl, len, self.lines.pos)
            };
            if n == 0 {
                IndexStep::Stay(line_no)
            } else if line_no >= nlines {
                IndexStep::Missing(line_no)
            } else {
                match line_no.checked_add(n) {
                    Some(target) if target <= nlines => {
                        let pos = if target == nlines {
                            len
                        } else {
                            line_start_pos(nl, len, target)
                        };
                        IndexStep::Landed(target, pos)
                    }
                    _ => IndexStep::Short(nlines, len),
                }
            }
        };
        match step {
            IndexStep::Stay(line_no) => {
                self.lines.line_no = line_no;
                Ok(())
            }
            IndexStep::Missing(line_no) => {
                self.lines.line_no = line_no;
                Err(error::ParseError::IncompleteFrame)
            }
            IndexStep::Landed(line_no, pos) => {
                self.lines.line_no = line_no;
                self.lines.pos = pos;
                Ok(())
            }
            IndexStep::Short(line_no, pos) => {
                self.lines.line_no = line_no;
                self.lines.pos = pos;
                Err(error::ParseError::IncompleteFrame)
            }
        }
    }

    /// One line from the shared cursor (same as full-parse path).
    fn read_line_str(&mut self) -> Option<&'a str> {
        self.lines.clear_peek();
        self.lines.next_line()
    }

    /// memchr-backed equivalent of [`Self::forward`]. Skips the next
    /// frame without fully parsing its atom data. Shares the same line
    /// cursor as [`Iterator::next`], so skip and full parse interleave safely.
    #[inline(never)]
    pub fn forward_fast(&mut self) -> Option<Result<(), error::ParseError>> {
        self.forward_fast_to(None)
    }

    /// [`Self::forward_fast`] for a frame expected to end at byte `end`.
    ///
    /// The result and the new position are the same as without `end`.
    /// Line blocks that end exactly at `end` are counted, not searched.
    pub(crate) fn forward_fast_to(
        &mut self,
        end: Option<usize>,
    ) -> Option<Result<(), error::ParseError>> {
        self.lines.clear_peek();
        if self.lines.pos >= self.lines.bytes.len() {
            return None;
        }
        // Line 1, then line 2. The section declaration decides whether a
        // following blank belongs to this frame.
        if self.read_line_str().is_none() {
            return Some(Err(error::ParseError::IncompleteHeader));
        }
        let line2 = match self.read_line_str() {
            Some(line) => line.to_string(),
            None => return Some(Err(error::ParseError::IncompleteHeader)),
        };
        let mut declared = declared_sections(&line2);
        // Lines 3..=6.
        if let Err(e) = self.advance_lines(4) {
            return Some(Err(e));
        }
        // Line 7: natm_types.
        let natm_types: usize = match self.read_line_str() {
            Some(line) => match crate::parser::parse_count_line(line, 1) {
                Ok(v) => v[0],
                Err(e) => return Some(Err(e)),
            },
            None => return Some(Err(error::ParseError::IncompleteHeader)),
        };
        if natm_types == 0 {
            // Lines 8 and 9 are whitespace, or absent at EOF. Do not scan
            // for coordinates or a legacy velocity block (CON-count.1).
            return Some(crate::parser::read_zero_atom_tail(|| self.read_line_str()));
        }
        // Line 8: natms_per_type.
        let natms_per_type: Vec<usize> = match self.read_line_str() {
            Some(line) => match crate::parser::parse_count_line(line, natm_types) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            },
            None => return Some(Err(error::ParseError::IncompleteHeader)),
        };
        if natms_per_type.contains(&0) {
            return Some(Err(error::ParseError::InvalidCount(
                "each per-type count must be >= 1".into(),
            )));
        }
        // Line 9: masses_per_type, consumed.
        if let Err(e) = self.advance_lines(1) {
            return Some(Err(e));
        }
        let total_atoms: usize = natms_per_type.iter().sum();
        let left = self.lines.bytes.len().saturating_sub(self.lines.pos);
        if self.lines.input_complete && total_atoms > left {
            return Some(Err(error::ParseError::IncompleteFrame));
        }
        let coord_block_lines = total_atoms + natm_types * 2;
        if let Err(e) = self.advance_lines_to(coord_block_lines, end) {
            return Some(Err(e));
        }
        // Optional sections: a blank line then one block per declared
        // section. A missing `sections` key is the legacy velocity block.
        // The blank test is Unicode White_Space, the same test as `trim`.
        self.lines.clear_peek();
        loop {
            let rest = &self.lines.bytes[self.lines.pos..];
            if rest.is_empty() {
                break;
            }
            let next_eol = memchr::memchr(b'\n', rest);
            let raw = match next_eol {
                Some(pos) => &rest[..pos],
                None => rest,
            };
            let line = strip_cr(raw);
            let expecting = match &declared {
                None => true,
                Some(names) => !names.is_empty(),
            };
            let text = match std::str::from_utf8(line) {
                Ok(text) => text,
                Err(_) => {
                    if expecting {
                        return Some(Err(error::ParseError::ValidationError(
                            "invalid UTF-8".into(),
                        )));
                    }
                    break;
                }
            };
            if !text.trim().is_empty() {
                break;
            }
            let after = match next_eol {
                Some(pos) => &rest[pos + 1..],
                None => &[][..],
            };
            // A blank line and then only whitespace is the end of the file,
            // not a section (CON-frame.5).
            if after.iter().all(|b| b.is_ascii_whitespace()) {
                self.lines.pos = self.lines.bytes.len();
                break;
            }
            // An empty declaration does not take the following block.
            // That blank is line 1 of the next frame (CON-frame.7).
            if matches!(&declared, Some(names) if names.is_empty()) {
                break;
            }
            match probe_section(after) {
                SectionProbe::InvalidUtf8 => {
                    return Some(Err(error::ParseError::ValidationError(
                        "invalid UTF-8".into(),
                    )));
                }
                // A blank that does not begin a section is line 1 of the
                // next frame (CON-frame.7, CON-header.4). A declared
                // section still occupies the blank when its label is not
                // the name the section asked for (CON-section.2).
                SectionProbe::Closed => {
                    // An empty list already broke above, so a Some here has names.
                    let declared_block = matches!(&declared, Some(_));
                    if !declared_block {
                        break;
                    }
                }
                SectionProbe::Opens => {}
            }
            if declared.is_none() && !legacy_rows_complete(after, &natms_per_type) {
                // A short row that is present is not a block (CON-frame.7).
                // A block whose label was seen and whose rows are missing is
                // truncated: IncompleteFrame at EOF, and not yet decided on
                // a prefix.
                match legacy_velocity_prefix(after, &natms_per_type) {
                    LegacyVelocity::NeedsMore { opened: true }
                        if self.lines.input_complete =>
                    {
                        return Some(Err(error::ParseError::IncompleteFrame));
                    }
                    LegacyVelocity::NeedsMore { .. } if !self.lines.input_complete => {
                        return Some(Err(error::ParseError::IncompleteSection(
                            "velocities".into(),
                        )));
                    }
                    _ => break,
                }
            }
            if let Some(names) = &mut declared {
                names.remove(0);
            }
            // Consume the blank separator and the section block.
            self.lines.pos += next_eol.map(|p| p + 1).unwrap_or(rest.len());
            if let Err(e) = self.advance_lines_to(coord_block_lines, end) {
                return Some(Err(e));
            }
            // A legacy file has one velocity block. A later blank is the
            // next frame (CON-frame.7), not another section.
            if declared.is_none() {
                break;
            }
        }
        Some(Ok(()))
    }

    /// Skips the next frame without fully parsing its atomic data.
    ///
    /// This is more efficient than `next()` if you only need to advance the
    /// iterator. It reads the frame's header to determine how many lines to skip,
    /// including any velocity section if present.
    ///
    /// # Returns
    ///
    /// * `Some(Ok(()))` on a successful skip.
    /// * `Some(Err(ParseError::...))` if there's an error parsing the header.
    /// * `None` if the iterator is already at the end.
    pub fn forward(&mut self) -> Option<Result<(), error::ParseError>> {
        // Prefer the shared memchr skip path (same cursor as full parse).
        self.forward_fast()
    }

    /// Byte offset of the next frame start in the buffer passed to [`Self::new`].
    pub fn byte_pos(&self) -> usize {
        self.lines.pos
    }

    /// The frame just parsed left a blank that is a prefix of a legacy
    /// velocity block. More bytes can still attach that block to the frame.
    fn legacy_velocity_prefix_is_open(&self, header: &types::FrameHeader) -> bool {
        if header.sections_declared || !header.sections.is_empty() {
            return false;
        }
        let Some(peeked) = &self.lines.peeked else {
            return false;
        };
        if !peeked.trim().is_empty() {
            return false;
        }
        matches!(
            legacy_velocity_prefix(
                &self.lines.bytes[self.lines.pos.min(self.lines.bytes.len())..],
                &header.natms_per_type,
            ),
            LegacyVelocity::NeedsMore { .. }
        )
    }

    /// Skip `n` frames without parsing atom data. Returns how many were skipped.
    /// Named `skip_frames` so it does not collide with [`Iterator::skip`].
    pub fn skip_frames(&mut self, n: usize) -> Result<usize, error::ParseError> {
        let mut skipped = 0usize;
        for _ in 0..n {
            match self.forward() {
                None => break,
                Some(Err(e)) => return Err(e),
                Some(Ok(())) => skipped += 1,
            }
        }
        Ok(skipped)
    }

    /// Next frame plus the exact substring of the buffer passed to [`Self::new`].
    ///
    /// **Corpus ingest contract:** successive successful spans from the same
    /// `file_contents` are contiguous (`end` of frame *i* equals `start` of frame
    /// *i+1*) and, for a buffer that is only multi-frame CON (no prefix garbage),
    /// concatenating all spans reproduces the trajectory text. Campaign stores
    /// (`readcon-db`) must persist these spans as authoritative blobs—do not
    /// re-serialize on the hot ingest path unless the caller supplied in-memory
    /// [`types::ConFrame`] values without source text.
    ///
    /// See also [`crate::index_proj::frame_byte_spans`] and
    /// [`crate::index_proj::spans_cover_buffer`].
    #[inline(never)]
    pub fn next_with_raw_span(
        &mut self,
        file_contents: &'a str,
    ) -> Option<Result<(types::ConFrame, &'a str), error::ParseError>> {
        if self.check_bom
            && self.lines.pos == 0
            && self.lines.peeked.is_none()
            && file_contents.starts_with('\u{feff}')
        {
            return Some(Err(error::ParseError::ValidationError(
                "file begins with a byte-order mark".into(),
            )));
        }
        let base = file_contents.as_ptr() as usize;
        let start = {
            let line = self.lines.peek_line()?;
            line.as_ptr() as usize - base
        };
        let frame = match self.next()? {
            Ok(f) => f,
            Err(e) => return Some(Err(e)),
        };
        let end = match self.lines.peek_line() {
            Some(line) => line.as_ptr() as usize - base,
            None => file_contents.len(),
        };
        debug_assert!(end >= start && end <= file_contents.len());
        Some(Ok((frame, &file_contents[start..end])))
    }

    /// [`Iterator::next`] plus the byte offset where the frame ends.
    pub(crate) fn next_with_end(
        &mut self,
    ) -> Option<Result<(types::ConFrame, usize), error::ParseError>> {
        let frame = match self.next()? {
            Ok(f) => f,
            Err(e) => return Some(Err(e)),
        };
        let end = match self.lines.peek_line() {
            Some(line) => line.as_ptr() as usize - self.lines.bytes.as_ptr() as usize,
            None => self.lines.pos,
        };
        Some(Ok((frame, end)))
    }
}

impl<'a> Iterator for ConFrameIterator<'a> {
    /// The type of item yielded by the iterator.
    ///
    /// Each item is a `Result` that contains a successfully parsed `ConFrame` or a
    /// `ParseError` if the frame's data is malformed.
    type Item = Result<types::ConFrame, error::ParseError>;

    /// Advances the iterator and attempts to parse the next frame.
    ///
    /// This method will return `None` only when there are no more lines to consume.
    /// If there are lines but they do not form a complete frame, it will return
    /// `Some(Err(ParseError::...))`. Invalid UTF-8 is a validation error, not
    /// the end of the stream.
    #[inline(never)]
    fn next(&mut self) -> Option<Self::Item> {
        // A byte-order mark is rejected only at the start of the file
        // (CON-file.2), before any peek moves the cursor. On a later
        // frame it is text of that line.
        if self.check_bom
            && self.lines.pos == 0
            && self.lines.peeked.is_none()
            && self.lines.bytes.starts_with("\u{feff}".as_bytes())
        {
            self.lines.line_no = 1;
            return Some(Err(error::ParseError::ValidationError(
                "file begins with a byte-order mark".into(),
            )));
        }
        // A line rejected earlier in this cursor (for example while probing
        // for a velocity block) must not be reported as a clean EOF.
        if let Some(err) = self.lines.utf8_error() {
            return Some(Err(err));
        }
        // Trailing whitespace after the last complete frame is not a frame
        // (CON-frame.1, D2). An empty line that is followed by more text is
        // line 1 of the next frame (CON-frame.7).
        // `remainder_is_whitespace` is true only when this cursor has not
        // rejected a line. A decode error returns false there and is
        // reported by the check below.
        if self.lines.remainder_is_whitespace() {
            while self.lines.next_line().is_some() {}
            return None;
        }
        if let Some(err) = self.lines.utf8_error() {
            return Some(Err(err));
        }
        // Otherwise, attempt to parse the next frame from the available lines.
        let mut frame = match crate::parser::parse_single_frame_ls(&mut self.lines) {
            Ok(f) => f,
            Err(e) => return Some(Err(self.lines.prefer_utf8(e))),
        };
        // Optional sections mutate AoS; only re-sync section SoA when needed.
        // Plain .con assembly already filled positions/ids/masses (no O(N)
        // post-scan when no velocity/force sections were applied).
        let sections =
            match parse_declared_sections(&mut self.lines, &mut frame.header, &mut frame.atom_data)
            {
                Ok(n) => n,
                // A section line that failed UTF-8 makes `next_line` return
                // `None`. That must not surface as `IncompleteSection`.
                Err(e) => return Some(Err(self.lines.prefer_utf8(e))),
            };
        if sections > 0 {
            frame.sync_arrays_from_atom_data();
        }
        // A prefix may end inside a legacy velocity block. The block is part
        // of this frame once its rows arrive, so the frame is not finished.
        if sections == 0 && self.legacy_velocity_prefix_is_open(&frame.header) {
            // The label was seen. Missing rows are IncompleteFrame once the
            // file has ended, and not a finished frame while bytes remain.
            let opened = matches!(
                legacy_velocity_prefix(
                    &self.lines.bytes[self.lines.pos.min(self.lines.bytes.len())..],
                    &frame.header.natms_per_type,
                ),
                LegacyVelocity::NeedsMore { opened: true }
            );
            if !self.lines.input_complete {
                return Some(Err(error::ParseError::IncompleteSection(
                    "velocities".into(),
                )));
            }
            if opened {
                return Some(Err(error::ParseError::IncompleteFrame));
            }
        }
        Some(Ok(frame))
    }
}

#[cfg(test)]
mod aos_soa_agreement_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn iterator_vel_forces_soa_matches_aos() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/tiny_cuh2_vel_forces.con");
        let text = std::fs::read_to_string(&p).expect("fixture");
        let frames = [
            text,
            std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/tiny_cuh2.con"),
            )
            .expect("plain fixture"),
        ];
        let mut saw_velocity = false;
        let mut saw_force = false;
        let mut saw_plain = false;
        for text in frames {
            let fr = ConFrameIterator::new(&text)
                .next()
                .expect("frame")
                .expect("parse");
            let n = fr.atom_data.len();
            assert!(n > 0);
            assert_eq!(fr.positions.nrows(), n);
            let has_vel = fr.atom_data.iter().any(|a| a.velocity().is_some());
            let has_frc = fr.atom_data.iter().any(|a| a.force().is_some());
            saw_velocity |= has_vel;
            saw_force |= has_frc;
            saw_plain |= !has_vel && !has_frc;
            if has_vel {
                assert_eq!(fr.velocities.nrows(), n);
            }
            if has_frc {
                assert_eq!(fr.forces.nrows(), n, "SoA forces must match AoS");
            }
            assert_eq!(has_vel, fr.velocities.nrows() == n);
            assert_eq!(has_frc, fr.forces.nrows() == n);
            for (i, a) in fr.atom_data.iter().enumerate() {
                let p = fr.positions.as_f64_row(i);
                assert_eq!([a.x, a.y, a.z], p);
                if let Some(v) = a.velocity() {
                    assert_eq!(v, fr.velocities.as_f64_row(i));
                }
                if let Some(f) = a.force() {
                    assert_eq!(f, fr.forces.as_f64_row(i));
                }
                assert_eq!(a.velocity().is_some(), has_vel);
                assert_eq!(a.force().is_some(), has_frc);
            }
        }
        assert!(saw_velocity);
        assert!(saw_force);
        assert!(saw_plain);
    }

    /// After SoA-primary parse, section sync must not require rewriting positions
    /// (nrows already equals N); forces SoA still filled from AoS.
    #[test]
    fn sync_skips_pos_when_nrows_matches_keeps_force_soa() {
        let p =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/tiny_cuh2_forces.con");
        let text = std::fs::read_to_string(&p).expect("fixture");
        let fr = ConFrameIterator::new(&text)
            .next()
            .expect("frame")
            .expect("parse");
        let n = fr.atom_data.len();
        assert_eq!(fr.positions.nrows(), n);
        assert_eq!(fr.forces.nrows(), n);
        // Snapshot first position SoA row then re-sync; coords must stay bit-identical
        // (no needless rewrite would change nothing but we still require agreement).
        let p0 = fr.positions.as_f64_row(0);
        let mut fr2 = fr.clone();
        fr2.sync_arrays_from_atom_data();
        assert_eq!(fr2.positions.as_f64_row(0), p0);
        assert_eq!(fr2.forces.nrows(), n);
        assert_eq!(
            fr2.forces.as_f64_row(0),
            fr2.atom_data[0].force().expect("force")
        );
    }
}

/// Reads all frames from a file.
///
/// The bytes are read into one owned buffer and parsed on the calling
/// thread. A worker pool runs only when [`read_all_frames_with_threads`]
/// is asked for two or more threads.
pub fn read_all_frames(path: &Path) -> Result<Vec<types::ConFrame>, Box<dyn std::error::Error>> {
    read_all_frames_with_threads(path, None)
}

/// Collect every frame from `text`.
///
/// `None`, `Some(0)`, and `Some(1)` parse on the calling thread.
/// `Some(n)` with `n >= 2` uses a Rayon pool of that size when the
/// `parallel` feature is on. Builds without `parallel` always parse
/// sequentially.
pub fn frames_from_text(
    text: &str,
    num_threads: Option<usize>,
) -> Result<Vec<types::ConFrame>, error::ParseError> {
    #[cfg(feature = "parallel")]
    {
        if let Some(n) = num_threads.filter(|n| *n >= 2) {
            let parts = parse_frames_parallel_with_threads(text, Some(n));
            let mut frames = Vec::with_capacity(parts.len());
            for r in parts {
                frames.push(r?);
            }
            return Ok(frames);
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = num_threads;
    }
    ConFrameIterator::new(text).collect()
}

/// Like [`read_all_frames`], with an explicit worker count.
///
/// `None`, `Some(0)`, and `Some(1)` parse on the calling thread.
/// `Some(n)` with `n >= 2` pins a Rayon pool when `parallel` is on.
/// The file is read into one owned buffer. The caller who wants a map
/// uses the streaming iterator.
pub fn read_all_frames_with_threads(
    path: &Path,
    num_threads: Option<usize>,
) -> Result<Vec<types::ConFrame>, Box<dyn std::error::Error>> {
    let bytes = crate::compression::read_file_bytes(path)?;
    Ok(frames_from_bytes(&bytes, num_threads)?)
}

/// Positions of every frame, one row-major `f64` buffer.
///
/// The boundary walk records each frame start. Each worker then parses
/// only its span and writes that frame's coordinates at
/// `frame * n_atoms * 3`. The returned buffer is the only retained
/// coordinate storage. A file whose frames do not share one atom count
/// is rejected, because the buffer is rectangular.
pub struct PositionBlock {
    pub n_frames: usize,
    pub n_atoms: usize,
    pub values: Vec<f64>,
}

pub fn read_positions_with_threads(
    path: &Path,
    num_threads: Option<usize>,
) -> Result<PositionBlock, Box<dyn std::error::Error>> {
    let contents = crate::compression::read_file_contents(path)?;
    let block = match &contents {
        crate::compression::FileContents::Owned(text) => {
            fill_position_block(text.as_bytes(), num_threads)
        }
        crate::compression::FileContents::Mapped(map) => fill_position_block(map, num_threads),
    }?;
    Ok(block)
}

fn frame_starts(bytes: &[u8]) -> Result<Vec<usize>, error::ParseError> {
    let mut scanner = ConFrameIterator::from_bytes(bytes);
    let mut boundaries = Vec::new();
    loop {
        scanner.lines.clear_peek();
        let start = scanner.lines.pos;
        if start >= bytes.len() || bytes[start..].iter().all(u8::is_ascii_whitespace) {
            break;
        }
        boundaries.push(start);
        match scanner.forward_fast() {
            Some(Ok(())) => {}
            Some(Err(err)) => return Err(err),
            None => break,
        }
    }
    Ok(boundaries)
}

fn parse_span(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<types::ConFrame, error::ParseError> {
    let mut iter = ConFrameIterator::from_bytes(&bytes[start..end]);
    match iter.next() {
        Some(result) => result,
        None => Err(error::ParseError::IncompleteFrame),
    }
}

fn copy_positions(frame: &types::ConFrame, dest: &mut [f64]) -> Result<(), error::ParseError> {
    if frame.positions.nrows().saturating_mul(3) != dest.len() {
        return Err(error::ParseError::InvalidCount(
            "frame atom count does not match the first frame".into(),
        ));
    }
    if let Some(src) = frame.positions.f64_slice() {
        dest.copy_from_slice(src);
        return Ok(());
    }
    for (i, slot) in dest.chunks_exact_mut(3).enumerate() {
        slot.copy_from_slice(&frame.positions.as_f64_row(i));
    }
    Ok(())
}

fn fill_position_block(
    bytes: &[u8],
    num_threads: Option<usize>,
) -> Result<PositionBlock, error::ParseError> {
    let boundaries = frame_starts(bytes)?;
    if boundaries.is_empty() {
        return Ok(PositionBlock {
            n_frames: 0,
            n_atoms: 0,
            values: Vec::new(),
        });
    }
    let end0 = boundaries.get(1).copied().unwrap_or(bytes.len());
    let first = parse_span(bytes, boundaries[0], end0)?;
    let n_atoms = first.positions.nrows();
    let n_frames = boundaries.len();
    let stride = n_atoms.saturating_mul(3);
    let mut values = vec![0.0_f64; n_frames.saturating_mul(stride)];
    if stride == 0 {
        return Ok(PositionBlock {
            n_frames,
            n_atoms,
            values,
        });
    }
    fill_position_spans(bytes, &boundaries, n_atoms, num_threads, &mut values)?;
    Ok(PositionBlock {
        n_frames,
        n_atoms,
        values,
    })
}

fn fill_position_spans(
    bytes: &[u8],
    boundaries: &[usize],
    n_atoms: usize,
    num_threads: Option<usize>,
    values: &mut [f64],
) -> Result<(), error::ParseError> {
    let stride = n_atoms * 3;
    let write_one = |i: usize, dest: &mut [f64]| {
        let start = boundaries[i];
        let end = boundaries.get(i + 1).copied().unwrap_or(bytes.len());
        let frame = parse_span(bytes, start, end)?;
        copy_positions(&frame, dest)
    };
    #[cfg(feature = "parallel")]
    {
        let parallel = !matches!(num_threads, Some(n) if n <= 1);
        if parallel {
            use rayon::prelude::*;
            let mut run = || {
                values
                    .par_chunks_mut(stride)
                    .enumerate()
                    .try_for_each(|(i, dest)| write_one(i, dest))
            };
            // `parallel` above is false for `Some(0)` and `Some(1)`, so a
            // `Some` here already names two or more workers.
            return match num_threads {
                Some(n) => {
                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(n)
                        .build()
                        .map_err(|err| error::ParseError::ValidationError(err.to_string()))?;
                    pool.install(run)
                }
                None => run(),
            };
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = num_threads;
    }
    for (i, dest) in values.chunks_mut(stride).enumerate() {
        write_one(i, dest)?;
    }
    Ok(())
}

/// [`frames_from_text`] for bytes that may not be UTF-8 yet.
pub fn frames_from_bytes(
    bytes: &[u8],
    num_threads: Option<usize>,
) -> Result<Vec<types::ConFrame>, error::ParseError> {
    if bytes.starts_with("\u{feff}".as_bytes()) {
        return Err(error::ParseError::ValidationError(
            "file begins with a byte-order mark".into(),
        ));
    }
    #[cfg(feature = "parallel")]
    {
        if let Some(n) = num_threads.filter(|n| *n >= 2) {
            let parts = parse_frames_parallel_bytes(bytes, Some(n));
            let mut frames = Vec::with_capacity(parts.len());
            for r in parts {
                frames.push(r?);
            }
            return Ok(frames);
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = num_threads;
    }
    let mut frames = Vec::new();
    for r in ConFrameIterator::from_bytes(bytes) {
        frames.push(r?);
    }
    Ok(frames)
}

#[cfg(feature = "parallel")]
fn span_end(boundaries: &[usize], index: usize, len: usize) -> usize {
    if index + 1 < boundaries.len() {
        boundaries[index + 1]
    } else {
        len
    }
}

/// Pointer to slot storage shared by the workers that claim chunks.
///
/// Each claimed index is written by one thread, so the alias is the
/// partition. The pointer stays valid until those workers join.
#[cfg(feature = "parallel")]
#[derive(Copy, Clone)]
struct SlotWritePtr(*mut std::mem::MaybeUninit<Result<types::ConFrame, error::ParseError>>);

#[cfg(feature = "parallel")]
impl SlotWritePtr {
    /// # Safety
    /// `index` is in range and this call is the only write of that slot.
    unsafe fn write_slot(self, index: usize, value: Result<types::ConFrame, error::ParseError>) {
        unsafe {
            (*self.0.add(index)).write(value);
        }
    }
}

// SAFETY: `SlotWritePtr` is copied into the claim loop, and that loop
// writes each slot index from exactly one worker. The allocation
// outlives the join.
#[cfg(feature = "parallel")]
unsafe impl Send for SlotWritePtr {}

// SAFETY: same partition as `Send`. Workers do not read a slot another
// worker is writing.
#[cfg(feature = "parallel")]
unsafe impl Sync for SlotWritePtr {}

/// Minor faults of the calling thread.
#[cfg(feature = "parallel")]
fn thread_minflt() -> i64 {
    unsafe {
        let mut usage = std::mem::zeroed::<libc::rusage>();
        libc::getrusage(libc::RUSAGE_THREAD, &mut usage);
        usage.ru_minflt
    }
}

/// Hardware LLC-miss counter for this process.
///
/// `-1` means the guest refused the event. This KVM guest returns
/// `EPERM` for `perf_event_open`, so the parse explanation uses the
/// per-worker minor faults instead.
#[cfg(feature = "parallel")]
fn sample_llc_misses() -> i64 {
    sample_llc_from(perf_open_os, perf_read_os, perf_close_os)
}

/// Last-level cache misses, using the three OS calls the caller supplies.
///
/// `open` returns a file descriptor, or a negative value when the kernel
/// refuses the event. `read` writes one `u64` and returns the byte count.
/// Production always passes [`perf_open_os`], [`perf_read_os`], and
/// [`perf_close_os`]. There is no environment switch.
#[inline(never)]
fn sample_llc_from(
    open: impl FnOnce() -> i64,
    read: impl FnOnce(i32, &mut u64) -> isize,
    close: impl FnOnce(i32),
) -> i64 {
    let fd = open();
    if fd < 0 {
        return -1;
    }
    let fd = fd as i32;
    let mut value = 0u64;
    let n = read(fd, &mut value);
    close(fd);
    if n == 8 { value as i64 } else { -1 }
}

fn perf_open_os() -> i64 {
    #[repr(C)]
    struct PerfEventAttr {
        typ: u32,
        size: u32,
        config: u64,
        rest: [u64; 14],
    }
    let mut attr = PerfEventAttr {
        typ: 3,
        size: std::mem::size_of::<PerfEventAttr>() as u32,
        // PERF_COUNT_HW_CACHE_LL, OP_READ, RESULT_MISS.
        config: 2 | (1 << 16),
        rest: [0; 14],
    };
    unsafe {
        libc::syscall(
            libc::SYS_perf_event_open,
            &attr as *const PerfEventAttr,
            0i32,
            -1i32,
            -1i32,
            0u64,
        )
    }
}

fn perf_read_os(fd: i32, value: &mut u64) -> isize {
    unsafe { libc::read(fd, value as *mut u64 as *mut libc::c_void, 8) }
}

fn perf_close_os(fd: i32) {
    unsafe {
        libc::close(fd);
    }
}

/// Pages that do not sit on node 0.
///
/// A guest with one memory node has no remote pages. The count is 0
/// in that case. A wider node list sums the pages `numa_maps` records
/// on every node other than 0.
#[cfg(feature = "parallel")]
fn numa_remote_pages() -> i64 {
    let online = std::fs::read_to_string("/sys/devices/system/node/online").unwrap_or_default();
    let maps = std::fs::read_to_string("/proc/self/numa_maps").unwrap_or_default();
    numa_remote_from(&online, &maps)
}

/// Pages that `maps` records on a node other than 0.
///
/// `online` is the kernel node list (`0` or `0-3`). One node has no
/// remote pages. A wider list sums every `N<node>=<pages>` token whose
/// node is not 0.
#[cfg(feature = "parallel")]
fn numa_remote_from(online: &str, maps: &str) -> i64 {
    let online = online.trim();
    let nodes = if let Some((start, end)) = online.split_once('-') {
        let start: i64 = start.parse().unwrap_or(0);
        let end: i64 = end.parse().unwrap_or(0);
        end - start + 1
    } else {
        1
    };
    if nodes <= 1 {
        return 0;
    }
    let mut remote = 0i64;
    for line in maps.lines() {
        for token in line.split_whitespace() {
            if let Some(rest) = token.strip_prefix('N') {
                if let Some((node, pages)) = rest.split_once('=') {
                    if node != "0" {
                        remote += pages.parse::<i64>().unwrap_or(0);
                    }
                }
            }
        }
    }
    remote
}

/// Parse each frame into a slot allocated before any worker runs.
///
/// Workers claim chunk indexes from an atomic counter and write only
/// those slots. The returned vector is that slot storage, in boundary
/// order, which is the serial iterator order. `workers < 2` parses on
/// the calling thread.
#[cfg(feature = "parallel")]
fn parse_into_slots<F>(
    bytes: &[u8],
    len: usize,
    boundaries: &[usize],
    workers: usize,
    parse_span: F,
) -> Vec<Result<types::ConFrame, error::ParseError>>
where
    F: Fn(usize, usize) -> Result<types::ConFrame, error::ParseError> + Sync,
{
    parse_into_slots_timed(bytes, len, boundaries, workers, parse_span).slots
}

/// Slot allocation and the worker parse, timed apart.
#[cfg(feature = "parallel")]
struct SlotParse {
    slots: Vec<Result<types::ConFrame, error::ParseError>>,
    alloc_s: f64,
    work_s: f64,
    sum_s: f64,
    max_s: f64,
    /// One entry per worker range, in worker order. This is CPU time.
    chunk_s: Vec<f64>,
    /// Minor faults on each worker during its range.
    worker_minflt: Vec<i64>,
    /// Frames in each worker's share.
    share_frames: Vec<u64>,
    /// File bytes in each worker's share.
    share_bytes: Vec<u64>,
}

/// Slot allocation and the worker parse, timed apart.
///
/// `alloc_s` is the slot vector. `work_s` is the parse wall. `sum_s` is
/// the sum of per-chunk parse times and `max_s` is the slowest chunk.
/// `chunk_s` keeps every chunk. `workers < 2` still reserves the slot
/// vector before the calling thread fills it, and that one chunk's time
/// is the sum, the max, and the only entry.
#[cfg(feature = "parallel")]
fn parse_into_slots_timed<F>(
    bytes: &[u8],
    len: usize,
    boundaries: &[usize],
    workers: usize,
    parse_span: F,
) -> SlotParse
where
    F: Fn(usize, usize) -> Result<types::ConFrame, error::ParseError> + Sync,
{
    let n = boundaries.len();
    if n == 0 {
        return SlotParse {
            slots: Vec::new(),
            alloc_s: 0.0,
            work_s: 0.0,
            sum_s: 0.0,
            max_s: 0.0,
            chunk_s: Vec::new(),
            worker_minflt: Vec::new(),
            share_frames: Vec::new(),
            share_bytes: Vec::new(),
        };
    }
    if workers < 2 {
        let t_alloc = std::time::Instant::now();
        let mut slots = Vec::with_capacity(n);
        let alloc_s = t_alloc.elapsed().as_secs_f64();
        let flt0 = thread_minflt();
        let t_work = std::time::Instant::now();
        for i in 0..n {
            slots.push(parse_span(boundaries[i], span_end(boundaries, i, len)));
        }
        let work_s = t_work.elapsed().as_secs_f64();
        let worker_minflt = vec![thread_minflt().saturating_sub(flt0)];
        return SlotParse {
            slots,
            alloc_s,
            work_s,
            sum_s: work_s,
            max_s: work_s,
            chunk_s: vec![work_s],
            worker_minflt,
            share_frames: vec![n as u64],
            share_bytes: vec![len as u64],
        };
    }
    let t_alloc = std::time::Instant::now();
    let mut slots = take_slot_buf(n);
    let alloc_s = t_alloc.elapsed().as_secs_f64();
    // One contiguous range per worker. The width is
    // `ceil(frames / workers)`, so equal work stays sequential and the
    // claim count is the worker count. A short tail is the last range.
    let share = n.div_ceil(workers);
    let chunk_ns: Vec<std::sync::atomic::AtomicU64> = (0..workers)
        .map(|_| std::sync::atomic::AtomicU64::new(0))
        .collect();
    let chunk_flt: Vec<std::sync::atomic::AtomicI64> = (0..workers)
        .map(|_| std::sync::atomic::AtomicI64::new(0))
        .collect();
    let slots_ptr = SlotWritePtr(slots.as_mut_ptr());
    let t_work = std::time::Instant::now();
    {
        let parse_share = |worker: usize| {
            let start = worker * share;
            if start >= n {
                return;
            }
            let end = (start + share).min(n);
            let flt0 = thread_minflt();
            let t_chunk = std::time::Instant::now();
            // One iterator for the share, the same walk a single thread
            // uses. A fresh iterator per frame faulted a private heap
            // on eight workers.
            let byte_start = boundaries[start];
            let byte_end = span_end(boundaries, end - 1, len);
            let mut iter = ConFrameIterator::from_bytes(&bytes[byte_start..byte_end]);
            if byte_start > 0 {
                iter.check_bom = false;
            }
            let mut last_ok = false;
            for i in start..end {
                let frame = match iter.next() {
                    Some(result) => result,
                    None => Err(error::ParseError::IncompleteFrame),
                };
                last_ok = frame.is_ok();
                // SAFETY: this worker owns `start..end`, so `i` has one
                // writer. `slots` outlives the join.
                unsafe {
                    slots_ptr.write_slot(i, frame);
                }
            }
            // A tail after a successful last frame is the error the serial
            // iterator reports. An error already in that frame stays: it is
            // earlier than the tail.
            if last_ok {
                if let Some(extra) = iter.next() {
                    let err = match extra {
                        Err(err) => err,
                        Ok(_) => error::ParseError::ValidationError(
                            "span contained more than one frame".into(),
                        ),
                    };
                    unsafe {
                        slots_ptr.write_slot(end - 1, Err(err));
                    }
                }
            }
            chunk_ns[worker].store(
                t_chunk.elapsed().as_nanos() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            chunk_flt[worker].store(
                thread_minflt().saturating_sub(flt0),
                std::sync::atomic::Ordering::Relaxed,
            );
        };
        // The thread index is the share index. Teardown drops that same
        // share on that same thread, so the pages come back without a
        // fresh fault on the next file.
        if rayon::current_thread_index().is_some() {
            rayon::broadcast(|ctx| {
                let worker = ctx.index();
                if worker < workers {
                    parse_share(worker);
                }
            });
        } else {
            use rayon::prelude::*;
            (0..workers).into_par_iter().for_each(parse_share);
        }
    }
    let work_s = t_work.elapsed().as_secs_f64();
    let mut worker_sum_s = 0.0_f64;
    let mut worker_max_s = 0.0_f64;
    let mut chunk_s = Vec::new();
    let mut worker_minflt = Vec::new();
    let mut share_frames = Vec::new();
    let mut share_bytes = Vec::new();
    for worker in 0..workers {
        let start = worker * share;
        if start >= n {
            continue;
        }
        let seconds = chunk_ns[worker].load(std::sync::atomic::Ordering::Relaxed) as f64 / 1.0e9;
        chunk_s.push(seconds);
        worker_sum_s += seconds;
        worker_max_s = worker_max_s.max(seconds);
        worker_minflt.push(chunk_flt[worker].load(std::sync::atomic::Ordering::Relaxed));
        let end = (start + share).min(n);
        share_frames.push((end - start) as u64);
        let bytes = span_end(boundaries, end - 1, len).saturating_sub(boundaries[start]);
        share_bytes.push(bytes as u64);
    }
    // Each index in `0..n` was written once. Move the frames out and
    // keep the slot pages for the next file of this width, so those
    // writes do not fault again.
    let mut owned = Vec::with_capacity(n);
    for index in 0..n {
        owned.push(unsafe { slots.as_mut_ptr().add(index).read().assume_init() });
    }
    unsafe {
        slots.set_len(0);
    }
    recycle_slot_buf(slots);
    let slots = owned;
    SlotParse {
        slots,
        alloc_s,
        work_s,
        sum_s: worker_sum_s,
        max_s: worker_max_s,
        chunk_s,
        worker_minflt,
        share_frames,
        share_bytes,
    }
}

#[cfg(feature = "parallel")]
fn parse_byte_span(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<types::ConFrame, error::ParseError> {
    let mut iter = ConFrameIterator::from_bytes(&bytes[start..end]);
    // A byte-order mark is rejected at the start of the file only.
    // A later frame's span begins at 0 of that slice.
    if start > 0 {
        iter.check_bom = false;
    }
    let frame = match iter.next() {
        Some(result) => result?,
        None => return Err(error::ParseError::IncompleteFrame),
    };
    // The scan ends a span at the next frame. Bytes after the frame in
    // this span are a tail: whitespace is nothing, and any other line is
    // an error the serial iterator would report. Dropping that line made
    // a parallel read accept a file the serial read rejects.
    match iter.next() {
        None => Ok(frame),
        Some(Err(err)) => Err(err),
        Some(Ok(_)) => Err(error::ParseError::ValidationError(
            "span contained more than one frame".into(),
        )),
    }
}

#[cfg(feature = "parallel")]
fn parse_text_span(
    text: &str,
    start: usize,
    end: usize,
) -> Result<types::ConFrame, error::ParseError> {
    let mut iter = ConFrameIterator::new(&text[start..end]);
    if start > 0 {
        iter.check_bom = false;
    }
    let frame = match iter.next() {
        Some(result) => result?,
        None => return Err(error::ParseError::IncompleteFrame),
    };
    match iter.next() {
        None => Ok(frame),
        Some(Err(err)) => Err(err),
        Some(Ok(_)) => Err(error::ParseError::ValidationError(
            "span contained more than one frame".into(),
        )),
    }
}

#[cfg(feature = "parallel")]
struct TimedParse {
    slots: Vec<Result<types::ConFrame, error::ParseError>>,
    /// Still alive so later phases of this read use the same workers.
    /// `None` when the call stayed on the calling thread.
    pool: Option<std::sync::Arc<rayon::ThreadPool>>,
    pool_s: f64,
    /// Zero while `pool` is still alive. The owner times the join.
    pool_join_s: f64,
    scan_s: f64,
    scan_minflt: i64,
    slot_alloc_s: f64,
    worker_parse_s: f64,
    worker_sum_s: f64,
    worker_max_s: f64,
    chunk_s: Vec<f64>,
    worker_minflt: Vec<i64>,
    share_frames: Vec<u64>,
    share_bytes: Vec<u64>,
    llc_misses: i64,
    numa_remote: i64,
    /// [`SCAN_SERIAL`], [`SCAN_INDEXED`], or [`SCAN_EQUAL_SPAN`].
    scan_kind: u8,
}

/// Keep freed parse pages in the heap.
///
/// Eight workers otherwise return those pages to the kernel, and the
/// next file faults them again. One worker already keeps a smaller set.
/// The same policy on every width leaves the parse fault count flat.
#[cfg(feature = "parallel")]
fn keep_parse_pages() {
    unsafe {
        // glibc `M_MMAP_MAX` and `M_TRIM_THRESHOLD`.
        libc::mallopt(-4, 0);
        libc::mallopt(-1, i32::MAX);
    }
}

/// Slot storage kept between parses.
///
/// Eight workers write one `ConFrame` shell per frame into this buffer.
/// A fresh buffer is a new mapping, and those writes are the repeating
/// minor faults. The next parse reuses the same pages.
#[cfg(feature = "parallel")]
fn take_slot_buf(
    n: usize,
) -> Vec<std::mem::MaybeUninit<Result<types::ConFrame, error::ParseError>>> {
    let mut buf = slot_cache()
        .lock()
        .map(|mut slot| std::mem::take(&mut *slot))
        .unwrap_or_default();
    if buf.capacity() < n {
        buf = Vec::with_capacity(n);
    }
    unsafe { buf.set_len(n) };
    buf
}

#[cfg(feature = "parallel")]
#[inline(never)]
fn recycle_slot_buf(
    mut buf: Vec<std::mem::MaybeUninit<Result<types::ConFrame, error::ParseError>>>,
) {
    unsafe { buf.set_len(0) };
    if let Ok(mut slot) = slot_cache().lock() {
        if buf.capacity() >= slot.capacity() {
            *slot = buf;
        }
    }
}

#[cfg(feature = "parallel")]
fn slot_cache()
-> &'static std::sync::Mutex<Vec<std::mem::MaybeUninit<Result<types::ConFrame, error::ParseError>>>>
{
    static CACHE: std::sync::Mutex<
        Vec<std::mem::MaybeUninit<Result<types::ConFrame, error::ParseError>>>,
    > = std::sync::Mutex::new(Vec::new());
    &CACHE
}

/// One pool per worker count, kept for the life of the process.
///
/// A fresh pool on every file made each worker fault its frames again.
/// Reusing the workers lets the allocator hand back the pages just freed.
#[cfg(feature = "parallel")]
fn shared_pool(workers: usize) -> Option<std::sync::Arc<rayon::ThreadPool>> {
    use std::sync::{Arc, Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<Vec<(usize, Arc<rayon::ThreadPool>)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = cache.lock().ok()?;
    if let Some((_, pool)) = guard.iter().find(|(count, _)| *count == workers) {
        return Some(Arc::clone(pool));
    }
    keep_parse_pages();
    let pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .ok()?,
    );
    guard.push((workers, Arc::clone(&pool)));
    Some(pool)
}

/// Minor faults of this process.
#[cfg(feature = "parallel")]
pub fn process_minflt() -> i64 {
    unsafe {
        let mut usage = std::mem::zeroed::<libc::rusage>();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        usage.ru_minflt
    }
}

/// Page size from `sysconf`, with the POSIX page as the fallback.
#[cfg(feature = "parallel")]
fn host_page_size() -> usize {
    let raw = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if raw > 0 { raw as usize } else { 4096 }
}

/// Fault one word on each page of `values`.
///
/// The page count is split by the worker count. The step is the host
/// page, so the slice a worker faults is its own. No extra chunk size.
#[cfg(feature = "parallel")]
fn first_touch_f64(values: &mut [f64], workers: usize) {
    use rayon::prelude::*;
    if values.is_empty() || workers < 2 {
        return;
    }
    let page = host_page_size().max(std::mem::size_of::<f64>());
    let step = (page / std::mem::size_of::<f64>()).max(1);
    let pages = values.len().div_ceil(step);
    let share = pages.div_ceil(workers);
    if rayon::current_thread_index().is_some() {
        // One chunk per page. The installed pool hands each page to one
        // worker, so that worker faults its own slice.
        // `par_chunks_mut` yields a non-empty slice, and `step` is at least 1.
        values.par_chunks_mut(step).for_each(|chunk| {
            chunk[0] = 0.0;
        });
        return;
    }
    std::thread::scope(|scope| {
        let mut rest = values;
        for worker in 0..workers {
            if rest.is_empty() {
                break;
            }
            let pages_here = share.min(rest.len().div_ceil(step));
            let take = (pages_here * step).min(rest.len());
            let (head, tail) = rest.split_at_mut(take);
            rest = tail;
            let _ = worker;
            scope.spawn(move || {
                let mut at = 0;
                while at < head.len() {
                    head[at] = 0.0;
                    at += step;
                }
            });
        }
    });
}

/// Move every `Ok` frame on the read's pool.
///
/// A serial walk of the slot vector grows with the frame count. The
/// pool that parsed the frames does this move. One thread still walks
/// the slots when that pool is absent.
#[cfg(feature = "parallel")]
#[inline(never)]
fn finish_results(
    pool: Option<&rayon::ThreadPool>,
    slots: Vec<Result<types::ConFrame, error::ParseError>>,
) -> Result<Vec<types::ConFrame>, error::ParseError> {
    if slots.len() < 2 || pool.is_none() {
        let mut frames = Vec::with_capacity(slots.len());
        for part in slots {
            frames.push(part?);
        }
        return Ok(frames);
    }
    let pool = pool.expect("pool");
    pool.install(|| {
        use rayon::prelude::*;
        if let Some(index) = slots.par_iter().position_first(|item| item.is_err()) {
            return Err(match slots.into_iter().nth(index) {
                Some(Err(err)) => err,
                _ => error::ParseError::IncompleteFrame,
            });
        }
        Ok(slots
            .into_par_iter()
            .map(|item| item.expect("ok"))
            .collect())
    })
}

/// Header walk on the calling thread.
#[cfg(feature = "parallel")]
pub const SCAN_SERIAL: u8 = 0;
/// Newline index in chunks, then a header walk on the calling thread.
///
/// Frames that do not share one byte length take this path. The newline
/// bytes are split across the pool. The header walk is not, because a
/// later frame start is the end of the skip, and that end is not known
/// until the skip runs.
#[cfg(feature = "parallel")]
pub const SCAN_INDEXED: u8 = 1;
/// Equal-length frames. Each worker boundary is checked.
#[cfg(feature = "parallel")]
pub const SCAN_EQUAL_SPAN: u8 = 2;

/// Frame starts when every frame occupies the same number of bytes.
///
/// The first frame sets the span. Each worker boundary is skipped on
/// its own slice. A span that does not tile the
/// buffer, or a skip that does not land on the next span, returns
/// `None` and the caller keeps the serial walk. The starts are
/// `0, span, 2*span, ...` and match [`collect_frame_starts`].
#[cfg(feature = "parallel")]
#[inline(never)]
fn equal_span_boundaries(bytes: &[u8], workers: usize) -> Option<Vec<usize>> {
    if workers < 2 || bytes.len() < 2 {
        return None;
    }
    let mut first = ConFrameIterator::from_bytes(bytes);
    match first.forward_fast() {
        Some(Ok(())) => {}
        _ => return None,
    }
    let span = first.lines.pos;
    if span < 2 || bytes.len() % span != 0 {
        return None;
    }
    let n = bytes.len() / span;
    if n < 2 {
        return None;
    }
    // Equal byte ranges. A cut that falls inside a frame resyncs to the
    // next frame start, and only that header is checked. The checks stay
    // on this worker: a nested pool wake was larger than the checks.
    let byte_share = bytes.len().div_ceil(workers);
    for worker in 0..workers {
        let byte_cut = worker.saturating_mul(byte_share).min(bytes.len());
        if byte_cut >= bytes.len() {
            continue;
        }
        let index = if worker == 0 {
            0
        } else {
            byte_cut.div_ceil(span).min(n - 1)
        };
        // `index` is at most `n - 1` and `bytes.len()` is `n * span`, so
        // `start` is strictly inside the buffer.
        let start = index.saturating_mul(span);
        let end = (start + span).min(bytes.len());
        let mut iter = ConFrameIterator::from_bytes(&bytes[start..end]);
        match iter.forward_fast() {
            Some(Ok(())) if iter.lines.pos == end - start => {}
            _ => return None,
        }
    }
    Some(arithmetic_starts(span, n))
}

/// Starts `0, span, 2*span, ...` from a buffer kept between files.
///
/// Building that buffer from a fresh allocation stalled for milliseconds
/// after a large free. The next file fills the same pages.
#[cfg(feature = "parallel")]
static START_CACHE: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "parallel")]
#[inline(never)]
fn arithmetic_starts(span: usize, n: usize) -> Vec<usize> {
    let mut starts = START_CACHE
        .lock()
        .map(|mut slot| std::mem::take(&mut *slot))
        .unwrap_or_default();
    if starts.len() != n {
        starts.resize(n, 0);
    }
    for (index, slot) in starts.iter_mut().enumerate() {
        *slot = index * span;
    }
    starts
}

#[cfg(feature = "parallel")]
#[inline(never)]
fn recycle_starts(starts: Vec<usize>) {
    if let Ok(mut slot) = START_CACHE.lock() {
        if starts.capacity() >= slot.capacity() {
            *slot = starts;
        }
    }
}

#[cfg(feature = "parallel")]
fn parse_frames_parallel_bytes(
    bytes: &[u8],
    num_threads: Option<usize>,
) -> Vec<Result<types::ConFrame, error::ParseError>> {
    parse_bytes_timed(bytes, num_threads).slots
}

/// Scan, slot allocation, and worker parse of `bytes`.
///
/// `Some(n)` with `n >= 2` uses a pool of that size. Every other count
/// stays on the calling thread. `None` follows [`scan_thread_count`].
/// The pool build is `pool_s`. The pool join is `pool_join_s`.
/// Neither one is inside the scan or the parse.
#[cfg(feature = "parallel")]
fn parse_bytes_timed(bytes: &[u8], num_threads: Option<usize>) -> TimedParse {
    let threads = scan_thread_count(num_threads, bytes.len());
    // There is no byte-length cutoff. Two or more workers scan a short
    // file as well as a long one. Equal-length frames walk their spans
    // on that pool instead of one serial header walk.
    let parse_workers = match num_threads {
        Some(n) if n >= 2 => n,
        Some(_) => 1,
        None => threads,
    };
    let run = || {
        let fault0 = process_minflt();
        let t_scan = std::time::Instant::now();
        let (boundaries, scan_kind) = {
                let mut scanner = ConFrameIterator::from_bytes(bytes);
                // The frame bounds are the serial cursor's. An indexed scan
                // ended these corpus frames before the line the serial parser
                // rejects, so the workers accepted a prefix.
                let scan_kind = SCAN_SERIAL;
                // Whitespace after the final frame does not open another frame.
                (collect_frame_starts(scanner, true), scan_kind)
            };
        let scan_s = t_scan.elapsed().as_secs_f64();
        let scan_minflt = process_minflt().saturating_sub(fault0);
        let parsed = parse_into_slots_timed(
            bytes,
            bytes.len(),
            &boundaries,
            parse_workers,
            |start, end| parse_byte_span(bytes, start, end),
        );
        recycle_starts(boundaries);
        (parsed, scan_s, scan_kind, scan_minflt)
    };
    let (pool_s, pool, packed) = match num_threads {
        None => (0.0, None, run()),
        Some(n) if n <= 1 => {
            let _ = n;
            keep_parse_pages();
            let fault0 = process_minflt();
            let t_scan = std::time::Instant::now();
            let boundaries = collect_frame_starts(ConFrameIterator::from_bytes(bytes), true);
            let scan_s = t_scan.elapsed().as_secs_f64();
            let scan_minflt = process_minflt().saturating_sub(fault0);
            let parsed =
                parse_into_slots_timed(bytes, bytes.len(), &boundaries, 1, |start, end| {
                    parse_byte_span(bytes, start, end)
                });
            (0.0, None, (parsed, scan_s, SCAN_SERIAL, scan_minflt))
        }
        Some(n) => {
            let t_pool = std::time::Instant::now();
            match shared_pool(n) {
                Some(pool) => {
                    let built = t_pool.elapsed().as_secs_f64();
                    let t_wake = std::time::Instant::now();
                    let (wake, body) = pool.install(|| {
                        let wake = t_wake.elapsed().as_secs_f64();
                        (wake, run())
                    });
                    // The same workers serve later files of this width.
                    // Their freed blocks stay warm for the next parse.
                    (built + wake, Some(pool), body)
                }
                None => (t_pool.elapsed().as_secs_f64(), None, run()),
            }
        }
    };
    let (parsed, scan_s, scan_kind, scan_minflt) = packed;
    TimedParse {
        slots: parsed.slots,
        pool,
        pool_s,
        pool_join_s: 0.0,
        scan_s,
        scan_minflt,
        slot_alloc_s: parsed.alloc_s,
        worker_parse_s: parsed.work_s,
        worker_sum_s: parsed.sum_s,
        worker_max_s: parsed.max_s,
        chunk_s: parsed.chunk_s,
        worker_minflt: parsed.worker_minflt,
        share_frames: parsed.share_frames,
        share_bytes: parsed.share_bytes,
        llc_misses: sample_llc_misses(),
        numa_remote: numa_remote_pages(),
        scan_kind,
    }
}

/// Count frames without building atom payloads (uses [`ConFrameIterator::forward_fast`]
/// when possible, else [`ConFrameIterator::forward`]).
///
/// Prefer this over `read_all_frames(...).len()` when only the frame count is needed.
pub fn count_frames(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    // The count is the number of frames the serial parse accepts.
    // A skip that ignores a bad token would report a count for a file the
    // parser rejects.
    let bytes = crate::compression::read_file_bytes(path)?;
    Ok(frames_from_bytes(&bytes, None)?.len())
}

/// Reads only the first frame from a file.
///
/// More efficient than `read_all_frames` for single-frame access because it
/// stops parsing after the first frame rather than collecting all of them.
pub fn read_first_frame(path: &Path) -> Result<types::ConFrame, Box<dyn std::error::Error>> {
    let contents = crate::compression::read_file_contents(path)?;
    let text = contents.as_str()?;
    let mut iter = ConFrameIterator::new(text);
    match iter.next() {
        Some(Ok(frame)) => Ok(frame),
        Some(Err(e)) => Err(Box::new(e)),
        None => Err("No frames found in file".into()),
    }
}

/// Skip `index` frames (no atom parse), then parse the next one.
///
/// `index == 0` is [`read_first_frame`]. Out of range is an error.
pub fn read_nth_frame(
    path: &Path,
    index: usize,
) -> Result<types::ConFrame, Box<dyn std::error::Error>> {
    let contents = crate::compression::read_file_contents(path)?;
    let text = contents.as_str()?;
    read_nth_frame_from_text(text, index)
}

/// Same as [`read_nth_frame`] on an already-loaded buffer.
pub fn read_nth_frame_from_text(
    text: &str,
    index: usize,
) -> Result<types::ConFrame, Box<dyn std::error::Error>> {
    let mut iter = ConFrameIterator::new(text);
    let skipped = iter.skip_frames(index)?;
    if skipped < index {
        return Err(format!("frame {index} out of range (only {skipped} frames)").into());
    }
    match iter.next() {
        Some(Ok(frame)) => Ok(frame),
        Some(Err(e)) => Err(Box::new(e)),
        None => Err(format!("frame {index} out of range").into()),
    }
}

/// Reads all frames using the checked `.rcidx` sidecar next to `path`.
///
/// Uncompressed input uses (and refreshes) `path.rcidx`; compressed
/// input is scanned in memory each time. UTF-8 is validated in
/// newline-aligned slices.
#[cfg(feature = "parallel")]
pub fn read_all_frames_indexed(
    path: &Path,
    num_threads: Option<usize>,
) -> Result<Vec<types::ConFrame>, Box<dyn std::error::Error>> {
    let contents = crate::compression::read_file_contents(path)?;
    let text = contents.as_str_sliced()?;
    let idx = match contents {
        crate::compression::FileContents::Mapped(_) => {
            crate::frame_index::load_or_build(path, text, true)?
        }
        crate::compression::FileContents::Owned(_) => {
            let on_disk = std::fs::metadata(path)?.len();
            crate::frame_index::load_or_build(path, text, on_disk == text.len() as u64)?
        }
    };
    let offs: Vec<usize> = idx.offsets.iter().map(|&o| o as usize).collect();
    parse_frames_at_offsets(text, &offs, num_threads)
        .into_iter()
        .map(|r| r.map_err(|e| Box::new(e) as Box<dyn std::error::Error>))
        .collect()
}

enum IndexStep {
    Stay(usize),
    Missing(usize),
    Landed(usize, usize),
    Short(usize, usize),
}

struct IndexedSpan {
    start: usize,
    content_end: usize,
    next_pos: usize,
}

/// Lines in `bytes`, counting a final line that has no `\n`.
fn line_count(nl: &[usize], len: usize) -> usize {
    if len == 0 {
        0
    } else if nl.last().copied() == Some(len - 1) {
        nl.len()
    } else {
        nl.len() + 1
    }
}

fn line_start_pos(nl: &[usize], len: usize, line_no: usize) -> usize {
    if line_no == 0 {
        return 0;
    }
    if line_no >= line_count(nl, len) {
        return len;
    }
    nl[line_no - 1] + 1
}

/// Line index of the line that contains `pos`, or the line count at `len`.
fn line_no_for_pos(nl: &[usize], len: usize, pos: usize) -> usize {
    if pos >= len {
        return line_count(nl, len);
    }
    nl.partition_point(|&p| p < pos)
}

fn indexed_span(nl: &[usize], bytes: &[u8], line_no: usize) -> Option<IndexedSpan> {
    let len = bytes.len();
    if line_no >= line_count(nl, len) {
        return None;
    }
    let start = if line_no == 0 { 0 } else { nl[line_no - 1] + 1 };
    let (raw_end, next_pos) = if line_no < nl.len() {
        (nl[line_no], nl[line_no] + 1)
    } else {
        (len, len)
    };
    let mut content_end = raw_end;
    if content_end > start && bytes[content_end - 1] == b'\r' {
        content_end -= 1;
    }
    Some(IndexedSpan {
        start,
        content_end,
        next_pos,
    })
}

/// `n` cuts covering `0..=len`. A cut may fall in the middle of a header line.
#[cfg(feature = "parallel")]
fn equal_cuts(len: usize, n: usize) -> Vec<usize> {
    let n = n.max(1);
    let mut cuts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let at = ((i as u128) * (len as u128) / (n as u128)) as usize;
        let at = at.min(len);
        if cuts.last().copied() != Some(at) {
            cuts.push(at);
        }
    }
    // `i == n` yields `len`, so the last cut is the buffer length.
    cuts
}

/// Newline offsets in `bytes`.
///
/// `cuts` splits the buffer into `[cuts[i], cuts[i + 1])`. Each piece is
/// scanned on its own and the offsets are concatenated. A `\n` belongs to
/// the piece that contains that byte, so a header line whose body crosses
/// a cut still contributes one newline.
#[cfg(feature = "parallel")]
fn newline_positions_at_cuts(bytes: &[u8], cuts: &[usize]) -> Vec<usize> {
    let spans: Vec<(usize, usize)> = cuts
        .windows(2)
        .map(|w| (w[0].min(bytes.len()), w[1].min(bytes.len())))
        .filter(|(start, end)| start < end)
        .collect();
    if spans.len() <= 1 {
        let (start, end) = spans.first().copied().unwrap_or((0, 0));
        return memchr::memchr_iter(b'\n', &bytes[start..end])
            .map(|p| start + p)
            .collect();
    }
    // The current Rayon pool, or the global pool when the caller has not
    // installed one. Chunk count is the worker cap: two chunks do not
    // wake every core.
    use rayon::prelude::*;
    let parts: Vec<Vec<usize>> = spans
        .par_iter()
        .map(|&(start, end)| {
            memchr::memchr_iter(b'\n', &bytes[start..end])
                .map(|p| start + p)
                .collect()
        })
        .collect();
    let mut all = Vec::with_capacity(bytes.len() / 32);
    for part in parts {
        all.extend(part);
    }
    all
}

#[cfg(feature = "parallel")]
fn newline_positions(bytes: &[u8], threads: usize) -> Vec<usize> {
    newline_positions_at_cuts(bytes, &equal_cuts(bytes.len(), threads))
}

/// Worker count for a newline scan.
///
/// `Some(n)` with `n >= 2` uses that many workers at every file length.
/// A file shorter than two bytes has nothing to split, and [`equal_cuts`]
/// drops a cut that repeats the previous byte, so a short file does not
/// invent an empty chunk. `None`, `Some(0)`, and `Some(1)` stay on the
/// calling thread. An omitted count must not start the global pool:
/// that pool made a one-frame file slower. There is no byte-length
/// cutoff taken from one host.
#[cfg(feature = "parallel")]
fn scan_thread_count(num_threads: Option<usize>, len: usize) -> usize {
    let requested = match num_threads {
        Some(n) if n >= 2 => n,
        _ => return 1,
    };
    if len < 2 { 1 } else { requested }
}

fn collect_frame_starts(
    mut scanner: ConFrameIterator<'_>,
    skip_whitespace_tail: bool,
) -> Vec<usize> {
    let mut boundaries = Vec::new();
    loop {
        scanner.lines.clear_peek();
        let start = scanner.lines.pos;
        if start >= scanner.lines.bytes.len() {
            break;
        }
        if skip_whitespace_tail {
            let rest = &scanner.lines.bytes[start..];
            if rest.iter().all(u8::is_ascii_whitespace) {
                break;
            }
        }
        boundaries.push(start);
        match scanner.forward_fast() {
            Some(Ok(())) => {}
            Some(Err(_)) | None => break,
        }
    }
    boundaries
}

/// Serial header walk. This is the reference the chunked scan must match.
pub fn frame_start_offsets_serial(file_contents: &str) -> Vec<usize> {
    collect_frame_starts(ConFrameIterator::new(file_contents), false)
}

#[cfg(feature = "parallel")]
fn scanner_with_newlines<'a>(text: &'a str, newlines: Vec<usize>) -> ConFrameIterator<'a> {
    let mut scanner = ConFrameIterator::new(text);
    scanner.lines.line_no = 0;
    scanner.lines.newlines = Some(newlines);
    scanner
}

#[cfg(feature = "parallel")]
fn frame_start_offsets_indexed(text: &str, newlines: Vec<usize>) -> Vec<usize> {
    collect_frame_starts(scanner_with_newlines(text, newlines), false)
}

/// Frame starts for `text` when the newline scan is split at `cuts`.
///
/// `cuts` is the chunk edges, including 0 and `text.len()`. A cut inside a
/// header line is the straddling case: the stitched offsets are still the
/// serial offsets.
#[cfg(all(feature = "parallel", test))]
fn frame_start_offsets_at_cuts(text: &str, cuts: &[usize]) -> Vec<usize> {
    let newlines = newline_positions_at_cuts(text.as_bytes(), cuts);
    frame_start_offsets_indexed(text, newlines)
}

/// Frame start byte offsets via [`ConFrameIterator::forward_fast`] (no atom parse).
///
/// Stops at the first skip error (that start is still included).
///
/// With the `parallel` feature and more than one worker, the buffer is
/// split into chunks and the newline bytes are scanned together. The
/// length of the buffer does not choose the worker count. The chunks
/// are not aligned to lines. A header
/// that straddles a cut is one line, because each `\n` is recorded by the
/// chunk that holds that byte and the lists are stitched in order. The
/// returned offsets match the serial walk.
pub fn frame_start_offsets(file_contents: &str) -> Vec<usize> {
    frame_start_offsets_with_threads(file_contents, None)
}

/// [`frame_start_offsets`] using `num_threads` chunks.
///
/// `None` uses the machine's parallelism. `Some(0)` and `Some(1)` stay on
/// the calling thread. The offsets do not depend on the thread count.
///
/// The timed work includes the cut setup, the per-chunk scans, the
/// stitch, and the frame walk that reads the stitched newline index.
pub fn frame_start_offsets_with_threads(
    file_contents: &str,
    num_threads: Option<usize>,
) -> Vec<usize> {
    #[cfg(feature = "parallel")]
    {
        let threads = scan_thread_count(num_threads, file_contents.len());
        if threads >= 2 {
            let newlines = newline_positions(file_contents.as_bytes(), threads);
            return frame_start_offsets_indexed(file_contents, newlines);
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = num_threads;
    }
    frame_start_offsets_serial(file_contents)
}

/// Parses frames in parallel using rayon, splitting on frame boundaries.
///
/// Phase 1: frame-start offsets from [`frame_start_offsets_with_threads`].
/// The newline scan is split into chunks and stitched, including a header
/// that crosses a chunk edge. The offsets match a serial
/// [`ConFrameIterator::forward_fast`] walk. An earlier walk built a
/// `Vec<&str>` of every line and summed lengths up to each frame, which
/// is O(N^2) in the line count.
///
/// Phase 2: each worker parses a contiguous run of those frames into
/// slots allocated before the parse. The slot order is the boundary
/// order. See [`parse_frames_parallel_with_threads`].
///
/// Requires the `parallel` feature.
#[cfg(feature = "parallel")]
pub fn parse_frames_parallel(
    file_contents: &str,
) -> Vec<Result<types::ConFrame, error::ParseError>> {
    parse_frames_parallel_with_threads(file_contents, None)
}

/// Like [`parse_frames_parallel`], but runs both phases on an explicit
/// Rayon pool with `num_threads` workers when `Some(n)` (`n` is clamped
/// to at least 1). `None` uses the global pool for a buffer of at least
/// two bytes, and the calling thread otherwise.
///
/// Phase 2 splits the frame offsets into one contiguous range per
/// worker. Each worker writes its frames into slots allocated before
/// the parse. The result matches a serial iterator, field for field
/// and in frame order.
///
/// Requires the `parallel` feature.
#[cfg(feature = "parallel")]
pub fn parse_frames_parallel_with_threads(
    file_contents: &str,
    num_threads: Option<usize>,
) -> Vec<Result<types::ConFrame, error::ParseError>> {
    // Phase 1 runs on the same pool as phase 2. Building the pool twice
    // would put the newline scan on the global pool and the parse on
    // another.
    let threads = scan_thread_count(num_threads, file_contents.len());
    let parse_workers = match num_threads {
        Some(n) if n >= 2 => n,
        Some(_) => 1,
        None => threads,
    };
    let run = || {
        // A whitespace tail is not a frame. The serial iterator stops
        // there. The offset scan used by callers still records that byte
        // when a skip error includes it.
        let boundaries = if threads >= 2 {
            let newlines = newline_positions(file_contents.as_bytes(), threads);
            collect_frame_starts(scanner_with_newlines(file_contents, newlines), true)
        } else {
            collect_frame_starts(ConFrameIterator::new(file_contents), true)
        };
        parse_into_slots(
            file_contents.as_bytes(),
            file_contents.len(),
            &boundaries,
            parse_workers,
            |start, end| parse_text_span(file_contents, start, end),
        )
    };
    match num_threads {
        Some(n) if n >= 2 => match rayon::ThreadPoolBuilder::new().num_threads(n).build() {
            Ok(pool) => pool.install(run),
            Err(_) => run(),
        },
        _ => run(),
    }
}

/// Phase 2 of [`parse_frames_parallel_with_threads`]: parse the frames
/// that start at `boundaries` (strictly increasing byte offsets into
/// `file_contents`; frame `i` ends where frame `i + 1` starts).
///
/// Offsets may come from [`frame_start_offsets`] or from a checked
/// [`crate::frame_index::FrameIndex`]. The output slots are allocated
/// first. Each worker then fills one contiguous index range. Results
/// stay in frame order.
///
/// The timer for this function starts after `boundaries` exist. It
/// includes slot allocation and the worker parse. It does not include
/// the newline scan or the stitch.
///
/// Requires the `parallel` feature.
#[cfg(feature = "parallel")]
pub fn parse_frames_at_offsets(
    file_contents: &str,
    boundaries: &[usize],
    num_threads: Option<usize>,
) -> Vec<Result<types::ConFrame, error::ParseError>> {
    let workers = match num_threads {
        Some(n) if n <= 1 => 1,
        Some(n) => n,
        None => rayon::current_num_threads().max(1),
    };
    let run = || {
        parse_into_slots(
            file_contents.as_bytes(),
            file_contents.len(),
            boundaries,
            workers,
            |start, end| parse_text_span(file_contents, start, end),
        )
    };
    match num_threads {
        Some(n) if n >= 2 => match rayon::ThreadPoolBuilder::new().num_threads(n).build() {
            Ok(pool) => pool.install(run),
            Err(_) => run(),
        },
        _ => run(),
    }
}

/// Seconds for one phased read. The open time is the file read or the
/// map. Scan, slot allocation, and the worker parse are separate.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadPhaseTimes {
    pub open_mmap_s: f64,
    /// Rayon pool construction. Zero when the call stays on one thread.
    pub pool_s: f64,
    /// Joining the local pool after the scan and the parse. Zero when
    /// the call stays on one thread.
    pub pool_join_s: f64,
    pub scan_s: f64,
    pub slot_alloc_s: f64,
    pub worker_parse_s: f64,
    /// Moving each parsed `Result` into the frame vector.
    pub finish_s: f64,
    /// Sum of per-chunk parse times. This is CPU time, not wall time.
    pub worker_sum_s: f64,
    /// Slowest parse chunk. This is inside `worker_parse_s`.
    pub worker_max_s: f64,
    /// Drop of the mapped or owned file buffer after the parse.
    pub close_s: f64,
    /// Wake of the worker pool and the other gaps inside the read.
    pub schedule_s: f64,
    /// Minor faults during the scan.
    pub scan_minflt: i64,
    /// Sum of per-worker minor faults during the parse.
    pub parse_minflt: i64,
    /// LLC read misses, or `-1` when the guest refuses the counter.
    pub llc_misses: i64,
    /// Pages on a node other than node 0. Zero on a one-node guest.
    pub numa_remote: i64,
    /// Minor faults while the coordinate slab is filled.
    pub slab_minflt: i64,
    /// Minor faults during the touch checksum.
    pub touch_minflt: i64,
    /// True when the bytes came from a map rather than an owned buffer.
    pub mapped: bool,
}

/// Frames from one phased read, plus the byte length of the buffer.
pub struct PhasedFrames {
    pub frames: Vec<types::ConFrame>,
    pub phases: ReadPhaseTimes,
    pub n_bytes: usize,
    /// Per-chunk parse seconds, in claim order.
    pub chunk_s: Vec<f64>,
    /// [`SCAN_SERIAL`], [`SCAN_INDEXED`], or [`SCAN_EQUAL_SPAN`] when
    /// `parallel` is on. Zero otherwise.
    pub scan_kind: u8,
    /// Minor faults on each parse worker.
    pub worker_minflt: Vec<i64>,
    /// Frames in each worker share.
    pub share_frames: Vec<u64>,
    /// File bytes in each worker share.
    pub share_bytes: Vec<u64>,
    /// Pool kept for the slab, the touch, and frame teardown.
    #[cfg(feature = "parallel")]
    pub pool: Option<std::sync::Arc<rayon::ThreadPool>>,
}

/// Read `path` and time open, scan, slot allocation, and the worker parse.
///
/// Plain text at or above the map threshold is mapped. Smaller files and
/// compressed files are owned buffers. `Some(n)` with `n >= 2` parses on
/// that many workers. Every other count parses on the calling thread.
/// The frame list matches [`frames_from_bytes`].
pub fn read_all_frames_phased(
    path: &Path,
    num_threads: Option<usize>,
) -> Result<PhasedFrames, Box<dyn std::error::Error>> {
    let t_all = std::time::Instant::now();
    let t_open = std::time::Instant::now();
    let contents = crate::compression::read_file_contents(path)?;
    let open_mmap_s = t_open.elapsed().as_secs_f64();
    let mapped = matches!(contents, crate::compression::FileContents::Mapped(_));
    let n_bytes = match &contents {
        crate::compression::FileContents::Owned(text) => text.len(),
        crate::compression::FileContents::Mapped(map) => map.len(),
    };
    let bytes: &[u8] = match &contents {
        crate::compression::FileContents::Owned(text) => text.as_bytes(),
        crate::compression::FileContents::Mapped(map) => map,
    };
    let phased = frames_from_bytes_phased(bytes, num_threads)?;
    let workers = match num_threads {
        Some(n) if n >= 2 => n,
        _ => 1,
    };
    let t_close = std::time::Instant::now();
    if let crate::compression::FileContents::Mapped(map) = &contents {
        // The serial unmap of a fully resident file is the multi-worker
        // tail. Advise the pages away on the read's pool first.
        // One worker unmaps directly, so a serial read does not walk
        // the pages twice.
        if workers >= 2 {
            #[cfg(feature = "parallel")]
            {
                if let Some(pool) = phased.pool.as_ref() {
                    pool.install(|| {
                        crate::compression::advise_dontneed_parallel(map, workers);
                    });
                }
            }
            #[cfg(not(feature = "parallel"))]
            {
                let _ = (map, workers);
            }
        }
    }
    drop(contents);
    let close_s = t_close.elapsed().as_secs_f64();
    let elapsed = t_all.elapsed().as_secs_f64();
    let named = open_mmap_s
        + phased.pool_s
        + phased.pool_join_s
        + phased.scan_s
        + phased.slot_alloc_s
        + phased.worker_parse_s
        + phased.finish_s
        + close_s;
    let schedule_s = (elapsed - named).max(0.0);
    let parse_minflt = phased.worker_minflt.iter().sum();
    Ok(PhasedFrames {
        frames: phased.frames,
        n_bytes,
        chunk_s: phased.chunk_s,
        scan_kind: phased.scan_kind,
        worker_minflt: phased.worker_minflt,
        share_frames: phased.share_frames,
        share_bytes: phased.share_bytes,
        phases: ReadPhaseTimes {
            open_mmap_s,
            pool_s: phased.pool_s,
            pool_join_s: phased.pool_join_s,
            scan_s: phased.scan_s,
            slot_alloc_s: phased.slot_alloc_s,
            worker_parse_s: phased.worker_parse_s,
            finish_s: phased.finish_s,
            worker_sum_s: phased.worker_sum_s,
            worker_max_s: phased.worker_max_s,
            close_s,
            schedule_s,
            scan_minflt: phased.scan_minflt,
            parse_minflt,
            llc_misses: phased.llc_misses,
            numa_remote: phased.numa_remote,
            mapped,
            ..ReadPhaseTimes::default()
        },
        #[cfg(feature = "parallel")]
        pool: phased.pool,
    })
}

/// [`frames_from_bytes`] with scan, slot allocation, and worker-parse times.
///
/// `Some(n)` with `n >= 2` uses that pool. Every other count, including
/// `None`, stays on the calling thread. That is the `read_con` policy:
/// an omitted count does not start a pool.
pub struct PhasedByteTimes {
    pub frames: Vec<types::ConFrame>,
    pub pool_s: f64,
    pub pool_join_s: f64,
    pub scan_s: f64,
    pub slot_alloc_s: f64,
    pub worker_parse_s: f64,
    pub finish_s: f64,
    pub worker_sum_s: f64,
    pub worker_max_s: f64,
    /// Per-chunk parse seconds, in claim order.
    pub chunk_s: Vec<f64>,
    /// [`SCAN_SERIAL`], [`SCAN_INDEXED`], or [`SCAN_EQUAL_SPAN`] when
    /// `parallel` is on. Zero otherwise.
    pub scan_kind: u8,
    /// Minor faults during the scan.
    pub scan_minflt: i64,
    /// Minor faults on each parse worker, in worker order.
    pub worker_minflt: Vec<i64>,
    pub share_frames: Vec<u64>,
    pub share_bytes: Vec<u64>,
    /// LLC read misses, or `-1` when the guest refuses the counter.
    pub llc_misses: i64,
    /// Pages on a node other than node 0.
    pub numa_remote: i64,
    /// Pool kept for the slab, the touch, and frame teardown.
    #[cfg(feature = "parallel")]
    pub pool: Option<std::sync::Arc<rayon::ThreadPool>>,
}

pub fn frames_from_bytes_phased(
    bytes: &[u8],
    num_threads: Option<usize>,
) -> Result<PhasedByteTimes, error::ParseError> {
    #[cfg(feature = "parallel")]
    {
        // One thread walks the file once. A scan plus a second parse made
        // a short file slower than the plain iterator.
        if !matches!(num_threads, Some(n) if n >= 2) {
            let flt0 = thread_minflt();
            let t_work = std::time::Instant::now();
            let mut frames = Vec::new();
            for part in ConFrameIterator::from_bytes(bytes) {
                frames.push(part?);
            }
            let worker_parse_s = t_work.elapsed().as_secs_f64();
            let worker_flt = thread_minflt().saturating_sub(flt0);
            let n_frames = frames.len() as u64;
            return Ok(PhasedByteTimes {
                frames,
                pool_s: 0.0,
                pool_join_s: 0.0,
                scan_s: 0.0,
                slot_alloc_s: 0.0,
                worker_parse_s,
                finish_s: 0.0,
                worker_sum_s: worker_parse_s,
                worker_max_s: worker_parse_s,
                chunk_s: vec![worker_parse_s],
                scan_kind: SCAN_SERIAL,
                scan_minflt: 0,
                worker_minflt: vec![worker_flt],
                share_frames: vec![n_frames],
                share_bytes: vec![bytes.len() as u64],
                llc_misses: sample_llc_misses(),
                numa_remote: numa_remote_pages(),
                pool: None,
            });
        }
        let requested = Some(num_threads.expect("workers"));
        let timed = parse_bytes_timed(bytes, requested);
        let t_finish = std::time::Instant::now();
        let frames = finish_results(timed.pool.as_deref(), timed.slots)?;
        let finish_s = t_finish.elapsed().as_secs_f64();
        return Ok(PhasedByteTimes {
            frames,
            pool_s: timed.pool_s,
            pool_join_s: timed.pool_join_s,
            scan_s: timed.scan_s,
            slot_alloc_s: timed.slot_alloc_s,
            worker_parse_s: timed.worker_parse_s,
            finish_s,
            worker_sum_s: timed.worker_sum_s,
            worker_max_s: timed.worker_max_s,
            chunk_s: timed.chunk_s,
            scan_kind: timed.scan_kind,
            scan_minflt: timed.scan_minflt,
            worker_minflt: timed.worker_minflt,
            share_frames: timed.share_frames,
            share_bytes: timed.share_bytes,
            llc_misses: timed.llc_misses,
            numa_remote: timed.numa_remote,
            pool: timed.pool,
        });
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = num_threads;
        let t_work = std::time::Instant::now();
        let mut frames = Vec::new();
        for part in ConFrameIterator::from_bytes(bytes) {
            frames.push(part?);
        }
        let worker_parse_s = t_work.elapsed().as_secs_f64();
        Ok(PhasedByteTimes {
            frames,
            pool_s: 0.0,
            pool_join_s: 0.0,
            scan_s: 0.0,
            slot_alloc_s: 0.0,
            worker_parse_s,
            finish_s: 0.0,
            worker_sum_s: worker_parse_s,
            worker_max_s: worker_parse_s,
            chunk_s: vec![worker_parse_s],
            scan_kind: 0,
            scan_minflt: 0,
            worker_minflt: Vec::new(),
            share_frames: Vec::new(),
            share_bytes: Vec::new(),
            llc_misses: -1,
            numa_remote: 0,
        })
    }
}

/// Copy every frame's coordinates into one row-major buffer.
///
/// `None` means the frames do not share one atom count. `workers >= 2`
/// writes disjoint frame runs. The copy is bit-identical to reading
/// each frame's `f64` positions in order.
pub fn pack_position_slab(frames: &[types::ConFrame], workers: usize) -> Option<Vec<f64>> {
    if frames.is_empty() {
        return None;
    }
    let n_atoms = frames[0].positions.nrows();
    if frames
        .iter()
        .any(|frame| frame.positions.nrows() != n_atoms)
    {
        return None;
    }
    let stride = n_atoms.saturating_mul(3);
    let total = frames.len().saturating_mul(stride);
    let mut values = Vec::<f64>::with_capacity(total);
    if total > 0 {
        // The parallel writers are the first touch of these pages.
        // A zeroed allocation would fault the whole slab on this thread.
        // SAFETY: every index is overwritten before the buffer is read.
        unsafe {
            values.set_len(total);
        }
    }
    if stride == 0 {
        return Some(values);
    }
    let write = |frame: &types::ConFrame, dest: &mut [f64]| {
        if let Some(src) = frame.positions.f64_slice() {
            dest.copy_from_slice(src);
        } else {
            for (row, slot) in dest.chunks_exact_mut(3).enumerate() {
                slot.copy_from_slice(&frame.positions.as_f64_row(row));
            }
        }
    };
    #[cfg(feature = "parallel")]
    {
        if workers >= 2 {
            let chunk_frames = frames.len().div_ceil(workers).max(1);
            let chunk_vals = chunk_frames.saturating_mul(stride);
            let copy_chunks = |dest_all: &mut [f64]| {
                use rayon::prelude::*;
                dest_all
                    .par_chunks_mut(chunk_vals)
                    .enumerate()
                    .for_each(|(chunk_index, dest)| {
                        let base = chunk_index * chunk_frames;
                        // The slab length is `frames.len() * stride`, and each
                        // chunk is at most `chunk_frames` rows, so every slot
                        // names a frame.
                        for (offset, slot) in dest.chunks_exact_mut(stride).enumerate() {
                            write(&frames[base + offset], slot);
                        }
                    });
            };
            // `par_chunks_mut` follows the installed pool. Outside one,
            // a private scope of `workers` threads keeps the global pool
            // out of this phase.
            if rayon::current_thread_index().is_some() {
                first_touch_f64(&mut values, workers);
                copy_chunks(&mut values);
            } else {
                first_touch_f64(&mut values, workers);
                std::thread::scope(|scope| {
                    let mut rest = &mut values[..];
                    let mut base = 0usize;
                    let mut handles = Vec::new();
                    while base < frames.len() {
                        let take_frames = chunk_frames.min(frames.len() - base);
                        let take_vals = take_frames * stride;
                        let (dest, tail) = rest.split_at_mut(take_vals);
                        rest = tail;
                        let frame_slice = &frames[base..base + take_frames];
                        handles.push(scope.spawn(move || {
                            for (frame, slot) in
                                frame_slice.iter().zip(dest.chunks_exact_mut(stride))
                            {
                                write(frame, slot);
                            }
                        }));
                        base += take_frames;
                    }
                    for handle in handles {
                        handle.join().expect("slab worker");
                    }
                });
            }
            return Some(values);
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = workers;
    }
    for (frame, dest) in frames.iter().zip(values.chunks_exact_mut(stride)) {
        write(frame, dest);
    }
    Some(values)
}

fn mix_touch(hash: u64, value: u64) -> u64 {
    hash.wrapping_mul(0x9E3779B185EBCA87) ^ value
}

fn one_frame_touch(frame: &types::ConFrame, coords: &[f64], index: u64) -> u64 {
    let mut hash = index;
    for value in frame.header.boxl {
        hash = mix_touch(hash, value.to_bits());
    }
    for atom in &frame.atom_data {
        hash = mix_touch(hash, atom.symbol.len() as u64);
        for byte in atom.symbol.as_bytes() {
            hash = mix_touch(hash, u64::from(*byte));
        }
    }
    for value in coords {
        hash = mix_touch(hash, value.to_bits());
    }
    hash
}

/// XOR of positions, cell lengths, and atom-type symbols.
///
/// `positions` is frame-major `[frames, atoms, 3]`. `None` means the
/// slice does not match the frames. `workers >= 2` folds disjoint
/// frame runs on that many threads. Each frame's hash includes its
/// index, and the returned word is the XOR of those hashes.
pub fn touch_frame_checksum(
    frames: &[types::ConFrame],
    positions: &[f64],
    workers: usize,
) -> Option<u64> {
    if frames.is_empty() {
        return Some(0);
    }
    let n_atoms = frames[0].atom_data.len();
    let stride = n_atoms.saturating_mul(3);
    if stride == 0 || positions.len() != frames.len().saturating_mul(stride) {
        return None;
    }
    if frames.iter().any(|frame| frame.atom_data.len() != n_atoms) {
        return None;
    }
    let one = |index: usize| {
        let start = index * stride;
        one_frame_touch(
            &frames[index],
            &positions[start..start + stride],
            index as u64,
        )
    };
    Some(reduce_touch(frames.len(), workers, one))
}

/// [`touch_frame_checksum`] for frames still sitting in optional slots.
///
/// `None` means a slot is empty or the position slice does not match.
#[inline(never)]
pub fn touch_option_checksum(
    frames: &[Option<types::ConFrame>],
    positions: &[f64],
    workers: usize,
) -> Option<u64> {
    if frames.is_empty() {
        return Some(0);
    }
    if frames.iter().any(Option::is_none) {
        return None;
    }
    let n_atoms = frames[0].as_ref()?.atom_data.len();
    let stride = n_atoms.saturating_mul(3);
    if stride == 0 || positions.len() != frames.len().saturating_mul(stride) {
        return None;
    }
    if frames
        .iter()
        .any(|frame| frame.as_ref().map(|item| item.atom_data.len()) != Some(n_atoms))
    {
        return None;
    }
    let one = |index: usize| {
        let start = index * stride;
        one_frame_touch(
            frames[index].as_ref().expect("slot"),
            &positions[start..start + stride],
            index as u64,
        )
    };
    Some(reduce_touch(frames.len(), workers, one))
}

fn reduce_touch(n: usize, workers: usize, one: impl Fn(usize) -> u64 + Sync) -> u64 {
    #[cfg(feature = "parallel")]
    if workers >= 2 {
        let fold_range = |first: usize, last: usize| {
            let mut acc = 0u64;
            for index in first..last {
                acc ^= one(index);
            }
            acc
        };
        let chunk = n.div_ceil(workers).max(1);
        // Reuse the installed read pool. A fresh pool per touch is the
        // build this phase used to pay on every call.
        if rayon::current_thread_index().is_some() {
            use rayon::prelude::*;
            return (0..workers)
                .into_par_iter()
                .map(|worker| {
                    let first = worker * chunk;
                    let last = (first + chunk).min(n);
                    fold_range(first, last)
                })
                .reduce(|| 0u64, |left, right| left ^ right);
        }
        let mut acc = 0u64;
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(workers);
            for worker in 0..workers {
                handles.push(scope.spawn(move || {
                    let first = worker * chunk;
                    let last = (first + chunk).min(n);
                    fold_range(first, last)
                }));
            }
            for handle in handles {
                acc ^= handle.join().unwrap_or(0);
            }
        });
        return acc;
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = workers;
    }
    (0..n).map(one).fold(0, |acc, item| acc ^ item)
}

/// Time a newline scan of `text`.
///
/// `chunked` with `threads >= 2` splits the buffer into that many
/// chunks, stitches the newline index, and walks frame starts. The
/// pool is built before the timer starts. The serial walk is
/// [`frame_start_offsets_serial`]. The returned offsets are the frame
/// starts. The caller chooses the worker count. File length does not.
pub fn measure_newline_scan(text: &str, threads: usize, chunked: bool) -> (Vec<usize>, f64) {
    #[cfg(feature = "parallel")]
    {
        if chunked && threads >= 2 {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build();
            let t_scan = std::time::Instant::now();
            let offsets = match pool {
                Ok(pool) => pool.install(|| {
                    let newlines = newline_positions(text.as_bytes(), threads);
                    frame_start_offsets_indexed(text, newlines)
                }),
                Err(_) => {
                    let newlines = newline_positions(text.as_bytes(), threads);
                    frame_start_offsets_indexed(text, newlines)
                }
            };
            return (offsets, t_scan.elapsed().as_secs_f64());
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = (threads, chunked);
    }
    let t_scan = std::time::Instant::now();
    let offsets = frame_start_offsets_serial(text);
    (offsets, t_scan.elapsed().as_secs_f64())
}

#[cfg(all(test, feature = "parallel"))]
mod parallel_strong_scale_tests {
    use super::*;
    use std::path::PathBuf;

    fn multi_frame_fixture() -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/tiny_cuh2.con");
        let one = std::fs::read_to_string(p).expect("fixture");
        // Enough frames for >1 worker to exercise the pool.
        one.repeat(8)
    }

    fn sequential_frames(text: &str) -> Vec<types::ConFrame> {
        ConFrameIterator::new(text)
            .map(|r| r.expect("seq frame"))
            .collect()
    }

    fn frames_payload_key(f: &types::ConFrame) -> (usize, Vec<(String, f64, f64, f64)>) {
        let atoms: Vec<_> = f
            .atom_data
            .iter()
            .map(|a| (a.symbol.to_string(), a.x, a.y, a.z))
            .collect();
        (f.atom_data.len(), atoms)
    }

    #[test]
    fn parallel_workers_match_sequential_payloads() {
        let text = multi_frame_fixture();
        let seq = sequential_frames(&text);
        assert!(seq.len() >= 8);
        let seq_keys: Vec<_> = seq.iter().map(frames_payload_key).collect();

        for workers in [1usize, 2, 4] {
            let par = parse_frames_parallel_with_threads(&text, Some(workers));
            assert_eq!(par.len(), seq.len(), "workers={workers}");
            let par_keys: Vec<_> = par
                .into_iter()
                .map(|r| frames_payload_key(&r.expect("par frame")))
                .collect();
            assert_eq!(par_keys, seq_keys, "workers={workers} frame payloads");
        }

        // Global pool path agrees too.
        let par_default = parse_frames_parallel(&text);
        let def_keys: Vec<_> = par_default
            .into_iter()
            .map(|r| frames_payload_key(&r.expect("par")))
            .collect();
        assert_eq!(def_keys, seq_keys);
    }

    fn live_threads() -> usize {
        std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find(|line| line.starts_with("Threads:"))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap()
    }

    /// A file past the old map and pool gates stays on this thread.
    #[test]
    fn default_read_of_a_wide_file_stays_on_the_calling_thread() {
        const CHILD: &str = "READCON_DEFAULT_THREAD_CHILD";
        const NAME: &str = "iterators::parallel_strong_scale_tests::default_read_of_a_wide_file_stays_on_the_calling_thread";
        if std::env::var_os(CHILD).is_none() {
            let out = std::process::Command::new(std::env::current_exe().unwrap())
                .args([NAME, "--exact", "--test-threads=1"])
                .env(CHILD, "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(out.status.success() && stdout.contains("test result: ok. 1 passed;"));
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.con");
        let one = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/tiny_cuh2.con"),
        )
        .unwrap();
        std::fs::write(&path, one.repeat(200)).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 64 * 1024);
        let before = live_threads();
        let frames = super::read_all_frames(&path).unwrap();
        let after = live_threads();
        assert_eq!(before, after, "default read started worker threads");
        assert!(frames.len() > 1);
        let text = std::fs::read_to_string(&path).unwrap();
        let sequential = super::frames_from_text(&text, None).unwrap();
        assert_eq!(sequential.len(), frames.len());
        assert_eq!(sequential[0].atom_data[0].x, frames[0].atom_data[0].x);
    }
}

#[cfg(all(test, feature = "parallel"))]
mod parallel_header_scan_tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test")
            .join(name);
        std::fs::read_to_string(path).expect(name)
    }

    /// A cut on the byte between `\r` and `\n`. The `\n` is in the next chunk.
    fn assert_crlf_gap_cut(text: &str) {
        let bytes = text.as_bytes();
        let mut found = 0usize;
        let mut i = 0usize;
        while i + 1 < bytes.len() && found < 6 {
            if bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
                let cut = i + 1;
                assert_eq!(bytes[cut - 1], b'\r');
                assert_eq!(bytes[cut], b'\n');
                assert_cuts_match(text, &[0, cut, bytes.len()]);
                found += 1;
                i += 2;
                continue;
            }
            i += 1;
        }
        assert!(found > 0, "fixture has no CRLF pair");
    }

    fn assert_cuts_match(text: &str, cuts: &[usize]) {
        let serial_nl: Vec<usize> = memchr::memchr_iter(b'\n', text.as_bytes()).collect();
        let stitched = newline_positions_at_cuts(text.as_bytes(), cuts);
        assert_eq!(stitched, serial_nl, "newline stitch at {cuts:?}");
        let serial = frame_start_offsets_serial(text);
        let parallel = frame_start_offsets_at_cuts(text, cuts);
        assert_eq!(parallel, serial, "frame offsets at {cuts:?}");
    }

    fn header_straddle_cuts(text: &str) -> Vec<Vec<usize>> {
        let serial = frame_start_offsets_serial(text);
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        // Equal splits, plus a cut forced into a header line of every frame.
        for n in [2usize, 3, 4, 8] {
            out.push(equal_cuts(bytes.len(), n));
        }
        for (index, &start) in serial.iter().enumerate() {
            let next = serial.get(index + 1).copied().unwrap_or(bytes.len());
            // The header ends at the coordinate label. The window is that
            // header, not a fixed count of bytes past the frame start.
            let window = super::header_window_end(bytes, start, next);
            let mut line = start;
            while line < window {
                let Some(rel) = memchr::memchr(b'\n', &bytes[line..window]) else {
                    break;
                };
                let end = line + rel;
                if end - line > 1 {
                    let mid = line + (end - line) / 2;
                    // mid is inside the line, so the header byte sits across two chunks.
                    assert!(mid > line);
                    assert!(mid < end);
                    out.push(vec![0, mid, bytes.len()]);
                }
                line = end + 1;
            }
        }
        out
    }

    fn assert_slab(text: &str) {
        assert!(!text.is_empty());
        for cuts in header_straddle_cuts(&text) {
            assert_cuts_match(&text, &cuts);
        }
        let bare = text.trim_end_matches(['\n', '\r']);
        if !bare.is_empty() {
            assert_cuts_match(bare, &equal_cuts(bare.len(), 4));
            let mid = bare.len() / 2;
            let line_start = bare[..mid].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = bare[mid..]
                .find('\n')
                .map(|i| mid + i)
                .unwrap_or(bare.len());
            if line_end - line_start > 1 {
                let cut = line_start + (line_end - line_start) / 2;
                assert_cuts_match(bare, &[0, cut, bare.len()]);
            }
        }
        let tailed = format!("{text}\n   \n\t\n");
        assert_cuts_match(&tailed, &[
            0,
            3,
            tailed.len() / 2,
            tailed.len() - 1,
            tailed.len(),
        ]);
        let crlf = text.replace('\n', "\r\n");
        assert_cuts_match(&crlf, &equal_cuts(crlf.len(), 5));
        if crlf.contains("\r\n") {
            assert_crlf_gap_cut(&crlf);
        }
        let serial = frame_start_offsets_serial(text);
        if serial.len() >= 2 {
            let second = serial[1];
            let next = serial.get(2).copied().unwrap_or(text.len());
            let broken_at = second + (next - second) / 2;
            let broken = &text[..broken_at.max(1)];
            assert_cuts_match(broken, &[0, 1.max(broken.len() / 3), broken.len()]);
        }
    }

    #[test]
    fn chunked_header_scan_matches_serial_offsets() {
        assert_eq!(frame_start_offsets_serial(""), frame_start_offsets(""));
        assert_cuts_match("\n", &[0, 1]);
        assert_cuts_match("   \n\n", &[0, 2, 5]);
        for name in [
            "tiny_cuh2.con",
            "tiny_cuh2.convel",
            "tiny_cuh2_forces.con",
            "tiny_cuh2_vel_forces.con",
            "cuh2.con",
        ] {
            let one = fixture(name);
            assert_slab(&one);
            assert_slab(&one.repeat(3));
        }
        // The bytes parser (the read_con path) uses the same stitch.
        // The single frame is the short file. The repeat is a wider file.
        // Both use the requested worker count. There is no length cutoff.
        let one = fixture("cuh2.con");
        for wide in [one.clone(), one.repeat(8)] {
            let serial = frame_start_offsets_serial(&wide);
            for workers in [2usize, 3, 4, 8] {
                let parallel = frame_start_offsets_with_threads(&wide, Some(workers));
                assert_eq!(parallel, serial, "workers={workers} bytes={}", wide.len());
                let parsed =
                    frames_from_bytes(wide.as_bytes(), Some(workers)).expect("bytes parse");
                assert_eq!(parsed.len(), serial.len());
                let seq = frames_from_text(&wide, Some(1)).expect("serial parse");
                assert_eq!(parsed.len(), seq.len());
                assert_eq!(parsed[0].atom_data[0].x, seq[0].atom_data[0].x);
                assert_eq!(
                    parsed.last().unwrap().atom_data.last().unwrap().z,
                    seq.last().unwrap().atom_data.last().unwrap().z
                );
            }
        }
    }

    #[test]
    fn header_window_covers_a_blank_line_and_a_missing_marker() {
        let bytes = b"\nCoordinates of Component 1\n";
        assert_eq!(super::header_window_end(bytes, 0, bytes.len()), 1);
        assert_eq!(super::header_window_end(b"abcdef", 0, 6), 6);
        assert_eq!(super::header_window_end(b"Coordinates\n", 4, 4), 4);
        let cuts = header_straddle_cuts("abcdef");
        assert!(!cuts.is_empty());
        assert_cuts_match("abcdef", &cuts[0]);
        assert_slab("a\n");
        assert_slab("\n");
        assert_slab("plain");
        assert_crlf_gap_cut("\r\r\n");
    }
}

/// End of the header window used by straddle cuts.
///
/// The header runs up to the line that starts the coordinate block.
/// When that label is absent the window is the next frame start.
#[cfg(all(test, feature = "parallel"))]
fn header_window_end(bytes: &[u8], start: usize, limit: usize) -> usize {
    let limit = limit.min(bytes.len());
    let marker = b"Coordinates";
    let mut line = start;
    while line < limit {
        let rest = &bytes[line..limit];
        let rel = memchr::memchr(b'\n', rest).unwrap_or(rest.len());
        let content_end = line + rel;
        let content = &bytes[line..content_end];
        let trimmed = match content.iter().position(|byte| !byte.is_ascii_whitespace()) {
            Some(index) => &content[index..],
            None => b"",
        };
        if trimmed.starts_with(marker) {
            return line;
        }
        if content_end >= limit {
            break;
        }
        line = content_end + 1;
    }
    limit
}

#[cfg(all(test, feature = "parallel"))]
mod parallel_frame_parse_tests {
    use super::*;
    use std::path::PathBuf;

    const ZERO_ATOM: &str = "\
zero-atom frame
{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\"}}
10.0 10.0 10.0
90.0 90.0 90.0


0


";

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test")
            .join(name);
        std::fs::read_to_string(path).expect(name)
    }

    fn serial_results(text: &str) -> Vec<Result<types::ConFrame, error::ParseError>> {
        ConFrameIterator::new(text).collect()
    }

    fn written(frame: &types::ConFrame) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut writer = crate::writer::ConFrameWriter::new(&mut out);
            writer.write_frame(frame).expect("write");
            writer.flush().expect("flush");
        }
        out
    }

    fn assert_same_results(
        label: &str,
        serial: Vec<Result<types::ConFrame, error::ParseError>>,
        parallel: Vec<Result<types::ConFrame, error::ParseError>>,
    ) {
        assert_eq!(serial.len(), parallel.len(), "{label} count");
        for (left, right) in serial.into_iter().zip(parallel) {
            assert_eq!(left.is_ok(), right.is_ok(), "{label}");
            if let (Ok(a), Ok(b)) = (&left, &right) {
                assert_eq!(written(a), written(b));
                assert_eq!(a, b);
            }
            if let (Err(a), Err(b)) = (&left, &right) {
                assert_eq!(format!("{a:?}"), format!("{b:?}"));
            }
        }
    }

    #[test]
    fn equal_errors_compare_without_a_frame() {
        let err = error::ParseError::IncompleteFrame;
        assert_same_results(
            "both",
            vec![Err(error::ParseError::IncompleteFrame)],
            vec![Err(err)],
        );
    }

    fn assert_workers(label: &str, text: &str) {
        let serial_ok = serial_results(text).iter().all(|item| item.is_ok());
        for workers in [1usize, 2, 3, 4, 8] {
            let parallel = parse_frames_parallel_with_threads(text, Some(workers));
            assert_same_results(
                &format!("{label} workers {workers}"),
                serial_results(text),
                parallel,
            );
            let bounds = collect_frame_starts(ConFrameIterator::new(text), true);
            let at = parse_frames_at_offsets(text, &bounds, Some(workers));
            assert_same_results(
                &format!("{label} offsets workers {workers}"),
                serial_results(text),
                at,
            );
        }
        if serial_ok {
            for workers in [2usize, 3, 8] {
                let bytes = frames_from_bytes(text.as_bytes(), Some(workers)).expect(label);
                let seq = frames_from_text(text, Some(1)).expect(label);
                assert_eq!(bytes.len(), seq.len(), "{label} byte count");
                for (i, (a, b)) in bytes.iter().zip(seq.iter()).enumerate() {
                    assert_eq!(written(a), written(b), "{label} byte path frame {i}");
                    assert_eq!(a, b, "{label} byte path fields frame {i}");
                }
            }
        }
    }

    fn crlf_gap_offsets(text: &str) -> Option<Vec<usize>> {
        let bytes = text.as_bytes();
        let mut cut = None;
        let mut i = 0usize;
        while i + 1 < bytes.len() {
            if bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
                cut = Some(i + 1);
                break;
            }
            i += 1;
        }
        let cut = cut?;
        Some(frame_start_offsets_at_cuts(text, &[0, cut, bytes.len()]))
    }

    #[test]
    fn parallel_slots_match_serial_frames() {
        assert_workers("empty", "");
        assert_workers("blank", "\n");
        assert_workers("space", "   \n\n");
        assert_workers("not a frame", "not-a-frame\n");
        assert!(crlf_gap_offsets("nope").is_none());
        assert!(crlf_gap_offsets("\rabc").is_none());
        let lone = crlf_gap_offsets("\rx\r\n").expect("crlf after a bare cr");
        assert_eq!(lone, frame_start_offsets_serial("\rx\r\n"));
        assert_workers("zero", ZERO_ATOM);
        assert_workers("zero-repeat", &ZERO_ATOM.repeat(5));
        for name in [
            "tiny_cuh2.con",
            "tiny_cuh2.convel",
            "tiny_cuh2_forces.con",
            "tiny_cuh2_vel_forces.con",
            "tiny_cuh2_charges_spins_magmoms.con",
            "tiny_cuh2_displacements.con",
            "tiny_cuh2_spreads.con",
            "cuh2.con",
        ] {
            let one = fixture(name);
            assert_workers(name, &one);
            assert_workers(&format!("{name} x5"), &one.repeat(5));
            let bare = one.trim_end_matches(['\n', '\r']);
            assert_workers(&format!("{name} bare"), bare);
            let tailed = format!("{one}\n   \n\t\n");
            assert_workers(&format!("{name} tail"), &tailed);
            let crlf = one.replace('\n', "\r\n");
            assert_workers(&format!("{name} crlf"), &crlf);
            let gap = crlf_gap_offsets(&crlf).expect("fixture crlf");
            assert_eq!(gap, frame_start_offsets_serial(&crlf));
            let parsed = parse_frames_at_offsets(&crlf, &gap, Some(4));
            assert_same_results(
                &format!("{name} crlf gap parse"),
                serial_results(&crlf),
                parsed,
            );
            let doubled = one.repeat(2);
            for sample in [one.as_str(), doubled.as_str(), "ab"] {
                let serial_offs = frame_start_offsets_serial(sample);
                let broken: &str = if serial_offs.len() >= 2 {
                    let second = serial_offs[1];
                    let next = serial_offs.get(2).copied().unwrap_or(sample.len());
                    &sample[..second + (next - second) / 2]
                } else if sample.len() > 8 {
                    &sample[..sample.len() / 2]
                } else {
                    ""
                };
                if !broken.is_empty() {
                    let serial_err = frames_from_text(broken, Some(1))
                        .err()
                        .map(|e| format!("{e:?}"));
                    assert!(serial_err.is_some(), "{name} chopped still parsed");
                    for workers in [1usize, 2, 4, 8] {
                        let parallel_err = frames_from_text(broken, Some(workers))
                            .err()
                            .map(|e| format!("{e:?}"));
                        assert_eq!(parallel_err, serial_err, "{name} chopped workers {workers}");
                        let byte_err = frames_from_bytes(broken.as_bytes(), Some(workers))
                            .err()
                            .map(|e| format!("{e:?}"));
                        assert_eq!(byte_err, serial_err);
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "parallel"))]
mod phased_read_tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test")
            .join(name);
        std::fs::read_to_string(path).expect(name)
    }

    #[test]
    fn phased_bytes_match_serial_frames() {
        let text = fixture("cuh2.con").repeat(4);
        let serial = frames_from_text(&text, Some(1)).expect("serial");
        for workers in [1usize, 2, 4, 8] {
            let phased = frames_from_bytes_phased(text.as_bytes(), Some(workers)).expect("phased");
            let parallel = phased.frames;
            assert_eq!(parallel.len(), serial.len(), "workers={workers}");
            assert!(phased.scan_s >= 0.0);
            assert!(phased.slot_alloc_s >= 0.0);
            assert!(phased.worker_parse_s >= 0.0);
            assert_eq!(parallel[0], serial[0], "workers={workers} first");
            assert_eq!(parallel.last().unwrap(), serial.last().unwrap());
            let slab = pack_position_slab(&serial, workers).expect("slab");
            let stride = serial[0].positions.nrows() * 3;
            let src = serial[0].positions.f64_slice().expect("f64");
            assert_eq!(&slab[..stride], src);
            let (chunked, _) = measure_newline_scan(&text, workers.max(2), true);
            let (plain, _) = measure_newline_scan(&text, 1, false);
            if workers >= 2 {
                assert_eq!(chunked, plain);
            }
        }
    }

    #[test]
    fn phased_file_matches_serial_and_maps_a_wide_file() {
        let text = fixture("cuh2.con").repeat(5);
        let dir = std::env::temp_dir();
        let path = dir.join("readcon-phased-wide.con");
        std::fs::write(&path, &text).expect("write");
        let phased = read_all_frames_phased(&path, Some(4)).expect("phased");
        let serial = frames_from_text(&text, Some(1)).expect("serial");
        assert_eq!(phased.frames, serial);
        assert!(phased.phases.mapped, "a file past 64 KiB is mapped");
        assert!(phased.phases.worker_parse_s >= 0.0);
        assert_eq!(phased.n_bytes, text.len());
        let _ = std::fs::remove_file(&path);

        let tiny = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/tiny_cuh2.con");
        let one = read_all_frames_phased(&tiny, Some(1)).expect("one worker");
        assert!(!one.phases.mapped, "a tiny file stays in an owned buffer");
        assert_eq!(one.frames.len(), 1);
        let auto = read_all_frames_phased(&tiny, None).expect("default");
        assert_eq!(auto.frames.len(), 1);
        let zero = read_all_frames_phased(&tiny, Some(0)).expect("zero means one");
        assert_eq!(
            zero.frames[0].atom_data.len(),
            one.frames[0].atom_data.len()
        );
    }

    fn outcome(result: Result<Vec<types::ConFrame>, error::ParseError>) -> Result<usize, String> {
        result
            .map(|frames| frames.len())
            .map_err(|err| err.class_name().to_string())
    }

    #[test]
    fn span_parser_matches_the_serial_parser_on_corpus31() {
        let dir = std::path::Path::new("paper/cpc/data/corpus31");
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .expect("corpus31")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "con" || ext == "convel")
            })
            .collect();
        names.sort();
        for path in names {
            let bytes = std::fs::read(&path).expect("read");
            let serial = outcome(frames_from_bytes(&bytes, None));
            for threads in [2usize, 8] {
                let parallel = outcome(frames_from_bytes(&bytes, Some(threads)));
                assert_eq!(serial, parallel);
            }
        }
    }

    #[test]
    fn span_parser_matches_the_serial_parser_on_the_corpus_edges() {
        for name in [
            "paper/cpc/data/corpus31/v31_unicode_blank_before_section.convel",
            "paper/cpc/data/corpus31/sections_empty_velocity.con",
            "paper/cpc/data/corpus31/sections_empty_velocity_6001_frames.con",
            "paper/cpc/data/corpus31/v31_utf8_separator_line.convel",
        ] {
            let bytes = std::fs::read(name).expect(name);
            let serial = outcome(frames_from_bytes(&bytes, None));
            for threads in [2usize, 8] {
                let parallel = outcome(frames_from_bytes(&bytes, Some(threads)));
                assert_eq!(serial, parallel, "{name} at {threads} threads");
            }
        }
    }

    #[test]
    fn equal_span_boundaries_match_the_serial_walk() {
        let text = fixture("cuh2.con").repeat(8);
        let bytes = text.as_bytes();
        let got = equal_span_boundaries(bytes, 4).expect("equal span");
        let expect = collect_frame_starts(ConFrameIterator::from_bytes(bytes), true);
        assert_eq!(got, expect);
        assert_eq!(got.len(), 8);
    }

    #[test]
    fn a_whitespace_tail_skips_the_equal_span_walk() {
        let mut text = fixture("cuh2.con").repeat(3);
        text.push('\n');
        assert!(equal_span_boundaries(text.as_bytes(), 4).is_none());
        let serial = frames_from_text(&text, Some(1)).expect("serial");
        let parallel = frames_from_bytes_phased(text.as_bytes(), Some(4)).expect("phased");
        assert_eq!(parallel.frames, serial);
    }

    #[test]
    fn touch_checksum_matches_across_worker_counts() {
        let text = fixture("cuh2.con").repeat(4);
        let serial = frames_from_bytes_phased(text.as_bytes(), Some(1)).expect("serial");
        let parallel = frames_from_bytes_phased(text.as_bytes(), Some(4)).expect("parallel");
        let slab_s = pack_position_slab(&serial.frames, 1).expect("slab");
        let slab_p = pack_position_slab(&parallel.frames, 4).expect("slab");
        let left = touch_frame_checksum(&serial.frames, &slab_s, 1).expect("touch");
        let right = touch_frame_checksum(&parallel.frames, &slab_p, 4).expect("touch");
        assert_eq!(left, right);
        assert_ne!(left, 0);
    }
}


/// Seeded mutations of the conformance corpus and the test files.
///
/// Each source file is paired with each mutation. The case count is that
/// product. Every entry point must agree with a one-thread string read.
#[cfg(test)]
mod differential_entry_points {
    use super::{
        frames_from_bytes, frames_from_text, read_all_frames, read_all_frames_with_threads,
        read_positions_with_threads,
    };
    use crate::error::ParseError;
    use crate::stream::ConFrameReader;
    use crate::trajectory::ConTrajectory;
    use crate::types::ConFrame;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;

    const MUTATIONS: usize = 4;
    const MAPPED_STRIDE: usize = 305;

    fn sources() -> Vec<String> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut paths = Vec::new();
        for dir in [
            "resources/conformance",
            "resources/test",
            "resources/not-a-directory",
        ] {
            let walk = root.join(dir);
            let mut stack = vec![walk];
            while let Some(dir) = stack.pop() {
                let entries = match std::fs::read_dir(&dir) {
                    Ok(entries) => entries,
                    Err(_) => continue,
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                        continue;
                    }
                    let ext = path.extension().and_then(|ext| ext.to_str());
                    if ext == Some("con") || ext == Some("convel") {
                        paths.push(path);
                    }
                }
            }
        }
        paths.sort();
        let mut texts: Vec<String> = paths
            .into_iter()
            .map(|path| std::fs::read_to_string(&path).unwrap_or_default())
            .filter(|text| !text.is_empty())
            .collect();
        texts.push("\n".to_string());
        texts.push(
            "user\n{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\"},\"sections\":[],\"storage_dtypes\":{\"positions\":\"float32\"}}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n1\n1\n1.0\nH\nCoordinates of Component 1\n0.0 0.0 0.0 0 1\n"
                .to_string(),
        );
        texts
    }

    fn mutate(text: &str, kind: usize, _index: usize) -> String {
        match kind {
            0 => format!("{text}\n\u{000c}\n"),
            1 => format!("{text}\n\u{00a0}\n"),
            2 => format!("{text}\n\n"),
            _ => format!("{text}\n\nEnergies of Component 1\n"),
        }
    }

    fn class(err: &ParseError) -> String {
        let rendered = format!("{err:?}");
        rendered
            .split(['(', '{', ' '])
            .next()
            .unwrap_or("")
            .to_string()
    }

    fn agree(reference: &Result<Vec<ConFrame>, ParseError>, got: Result<Vec<ConFrame>, ParseError>, what: &str) {
        assert_eq!(reference.is_ok(), got.is_ok(), "{what}");
        if let (Ok(left), Ok(right)) = (reference, &got) {
            assert_eq!(left, right, "{what}");
        }
        if let (Err(left), Err(right)) = (reference, &got) {
            assert_eq!(class(left), class(right), "{what}");
        }
    }

    fn stream_frames(bytes: &[u8], size: usize) -> Result<Vec<ConFrame>, ParseError> {
        let mut reader = ConFrameReader::with_read_size(Cursor::new(bytes), size);
        let mut frames = Vec::new();
        for item in &mut reader {
            frames.push(item.map_err(|err| ParseError::ValidationError(err.to_string()))?);
        }
        Ok(frames)
    }

    #[test]
    fn differential_corpus_mutations_agree_across_entry_points() {
        let sources = sources();
        let n_cases = sources.len() * MUTATIONS;
        assert!(n_cases > 0, "the differential has no source files");
        let dir = std::env::temp_dir().join("readcon-differential");
        let _ = std::fs::create_dir_all(&dir);
        for index in 0..n_cases {
            let text = mutate(&sources[index / MUTATIONS], index % MUTATIONS, index);
            let reference = frames_from_text(&text, Some(1));
            for threads in 1..=16 {
                agree(
                    &reference,
                    frames_from_text(&text, Some(threads)),
                    &format!("string threads={threads} case={index}"),
                );
                agree(
                    &reference,
                    frames_from_bytes(text.as_bytes(), Some(threads)),
                    &format!("bytes threads={threads} case={index}"),
                );
            }
            for size in [1usize, 61, crate::stream::DEFAULT_READ_SIZE] {
                let streamed = stream_frames(text.as_bytes(), size);
                assert_eq!(reference.is_ok(), streamed.is_ok());
                if let (Ok(left), Ok(right)) = (&reference, &streamed) {
                    assert_eq!(left, right);
                }
            }
            let plain = dir.join(format!("case-{index}.con"));
            std::fs::write(&plain, text.as_bytes()).unwrap();
            let from_file = read_all_frames(&plain).map_err(|err| err.to_string());
            assert_eq!(from_file.is_ok(), reference.is_ok(), "plain file case={index}");
            if let (Ok(left), Ok(right)) = (&reference, &from_file) {
                assert_eq!(left, right, "plain file case={index}");
            }
            let positions = read_positions_with_threads(&plain, Some(1));
            assert_eq!(positions.is_ok(), reference.is_ok(), "positions case={index}");
            if let (Ok(frames), Ok(block)) = (&reference, &positions) {
                if let Some(frame) = frames.first() {
                    let coords = frame.positions.f64_slice().map(|row| row.to_vec());
                    if frames.len() == 1 {
                        if let Some(coords) = coords {
                            assert_eq!(block.values, coords, "positions case={index}");
                        }
                    }
                }
            }
            let gz_path = dir.join(format!("case-{index}.con.gz"));
            {
                let file = std::fs::File::create(&gz_path).unwrap();
                let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                enc.write_all(text.as_bytes()).unwrap();
                enc.finish().unwrap();
            }
            let from_gz = read_all_frames(&gz_path).map_err(|err| err.to_string());
            assert_eq!(from_gz.is_ok(), reference.is_ok(), "gzip case={index}");
            if let (Ok(left), Ok(right)) = (&reference, &from_gz) {
                assert_eq!(left, right, "gzip case={index}");
            }
            let opened = ConTrajectory::open(&plain, false);
            if let (Ok(frames), Ok(traj)) = (&reference, &opened) {
                assert_eq!(traj.len(), frames.len());
                for (i, frame) in frames.iter().enumerate() {
                    assert_eq!(traj.frame(i).expect("trajectory frame"), *frame);
                    let _ = i;
                }
            } else if let (Err(_), Ok(traj)) = (&reference, &opened) {
                if !traj.is_empty() {
                    assert!(traj.frame(0).is_err());
                }
            } else {
                assert_eq!(reference.is_ok(), opened.is_ok());
            }
            if index % MAPPED_STRIDE == 0 {
                let padded = format!("{text}{}", "\n".repeat(64 * 1024));
                let mapped_path = dir.join(format!("mapped-{index}.con"));
                std::fs::write(&mapped_path, padded.as_bytes()).unwrap();
                let serial = frames_from_text(&padded, Some(1));
                for threads in 1..=16 {
                    let mapped = read_all_frames_with_threads(&mapped_path, Some(threads))
                        .map_err(|err| err.to_string());
                    assert_eq!(mapped.is_ok(), serial.is_ok(), "mapped threads={threads}");
                }
            }
            let _ = std::fs::remove_file(&plain);
            let _ = std::fs::remove_file(&gz_path);
        }
    }
}

#[cfg(all(test, feature = "parallel"))]
mod one_thread_and_numa {
    use super::{
        error, frames_from_bytes, line_count, line_no_for_pos, line_start_pos, numa_remote_from,
        copy_positions, fill_position_block, pack_position_slab, parse_byte_span, parse_bytes_timed,
        parse_text_span, perf_close_os, perf_open_os, perf_read_os, read_all_frames_indexed,
        read_first_frame, sample_llc_from, sample_llc_misses,
    };

    #[test]
    fn one_worker_and_the_default_stay_on_the_caller() {
        let bytes = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
        let serial = frames_from_bytes(&bytes, Some(1)).unwrap();
        for requested in [None, Some(0), Some(1)] {
            let timed = parse_bytes_timed(&bytes, requested);
            let got: Vec<_> = timed.slots.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
            assert_eq!(got, serial);
            assert!(timed.pool.is_none());
        }
    }

    #[test]
    fn a_span_reports_an_empty_slice_a_tail_and_a_second_frame() {
        let bytes = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
        let frame = parse_byte_span(&bytes, 0, bytes.len()).unwrap();
        assert!(frame.atom_data.len() >= 1);
        let err = parse_byte_span(&bytes, 0, 0).unwrap_err();
        assert!(matches!(err, error::ParseError::IncompleteFrame));
        let mut two = bytes.clone();
        two.extend_from_slice(&bytes);
        let err = parse_byte_span(&two, 0, two.len()).unwrap_err();
        assert!(matches!(err, error::ParseError::ValidationError(_)));
        let mut tail = bytes.clone();
        tail.extend_from_slice(b"\nnot-a-frame\n");
        assert!(parse_byte_span(&tail, 0, tail.len()).is_err());
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(parse_text_span(text, 0, text.len()).is_ok());
        assert!(matches!(
            parse_text_span(text, 0, 0),
            Err(error::ParseError::IncompleteFrame)
        ));
    }

    #[test]
    fn indexed_reads_and_line_positions_follow_the_newlines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.con");
        std::fs::copy("resources/test/tiny_cuh2.con", &path).unwrap();
        let frames = read_all_frames_indexed(&path, Some(1)).unwrap();
        assert!(!frames.is_empty());
        let again = read_all_frames_indexed(&path, None).unwrap();
        assert_eq!(again.len(), frames.len());
        let nl = [3usize, 7];
        assert_eq!(line_count(&nl, 0), 0);
        assert_eq!(line_count(&nl, 8), 2);
        assert_eq!(line_count(&nl, 9), 3);
        assert_eq!(line_no_for_pos(&nl, 8, 8), 2);
        assert_eq!(line_no_for_pos(&nl, 8, 0), 0);
        assert_eq!(line_no_for_pos(&nl, 8, 4), 1);
        assert_eq!(line_start_pos(&nl, 8, 0), 0);
        assert_eq!(line_start_pos(&nl, 8, 1), 4);
        assert_eq!(line_start_pos(&nl, 8, 9), 8);
        let sample = sample_llc_misses();
        assert!(sample >= -1);
    }

    #[test]
    fn position_blocks_and_real_descriptor_reads() {
        let bytes = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
        let block = fill_position_block(&bytes, Some(2)).unwrap();
        assert!(block.n_frames >= 1);
        assert_eq!(block.values.len(), block.n_frames * block.n_atoms * 3);
        let empty = fill_position_block(b"", Some(1)).unwrap();
        assert_eq!(empty.n_frames, 0);
        assert!(empty.values.is_empty());
        let frame = parse_byte_span(&bytes, 0, bytes.len()).unwrap();
        assert!(copy_positions(&frame, &mut [0.0]).is_err());
        let mut stored = frame.clone();
        let mut dtypes = crate::storage_dtype::StorageDtypes::default();
        dtypes.positions = crate::storage_dtype::ElementKind::Float32;
        stored.project_storage_dtypes(&dtypes);
        let mut dest = vec![0.0; stored.positions.nrows() * 3];
        copy_positions(&stored, &mut dest).unwrap();
        assert_eq!(dest.len(), block.n_atoms * 3);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eight.bin");
        std::fs::write(&path, 42u64.to_le_bytes()).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        use std::os::fd::AsRawFd;
        let mut value = 0u64;
        assert_eq!(perf_read_os(file.as_raw_fd(), &mut value), 8);
        assert_eq!(value, 42);
        let extra = unsafe { libc::dup(file.as_raw_fd()) };
        assert!(extra >= 0);
        perf_close_os(extra);

        let text = std::str::from_utf8(&bytes).unwrap();
        let mut bad = text.to_string();
        bad.push_str("\nnot a number\n");
        assert!(parse_text_span(&bad, 0, bad.len()).is_err());
        let mut iter = super::ConFrameIterator::from_bytes(&bytes);
        assert_eq!(iter.cursor_line(), iter.lines_read());

        let raw = b"ok\n\xff\n";
        let mut indexed = super::MemchrLines::from_bytes(raw);
        indexed.newlines = Some(vec![2, 4]);
        assert_eq!(indexed.next_line(), Some("ok"));
        assert!(indexed.next_line().is_none());
        let mut named = super::MemchrLines::new("Velocities of Component 1\n");
        assert!(crate::parser::LineStream::blank_opens_velocity_block(
            &mut named
        ));

        let mut stored = frame.clone();
        let mut dtypes = crate::storage_dtype::StorageDtypes::default();
        dtypes.positions = crate::storage_dtype::ElementKind::Float32;
        stored.project_storage_dtypes(&dtypes);
        let packed = super::pack_position_slab(&[stored], 1).unwrap();
        assert_eq!(packed.len(), block.n_atoms * 3);
        assert!(super::pack_position_slab(&[], 1).is_none());

        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.con");
        std::fs::write(&empty, "").unwrap();
        assert!(super::read_first_frame(&empty).is_err());
        let bad = dir.path().join("bad.con");
        let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
        std::fs::write(&bad, text.replacen("0.63940000000000108", "notanumber", 1)).unwrap();
        assert!(super::read_first_frame(&bad).is_err());
        assert!(super::read_nth_frame_from_text("", 0).is_err());
        assert!(super::read_nth_frame_from_text(&text, 4).is_err());
        let doubled = text.clone() + &text;
        assert!(super::parse_byte_span(doubled.as_bytes(), 1, doubled.len()).is_err());

        assert_eq!(super::touch_frame_checksum(&[], &[], 1), Some(0));
        assert!(super::touch_frame_checksum(&[frame.clone()], &[], 1).is_none());
        assert_eq!(super::touch_option_checksum(&[], &[], 1), Some(0));
        assert!(super::touch_option_checksum(&[None], &[], 1).is_none());
        let stride = frame.atom_data.len() * 3;
        let coords = vec![0.0; stride];
        assert!(super::touch_option_checksum(&[Some(frame.clone())], &coords[..stride - 1], 1).is_none());
    }

    #[test]
    fn indexed_rows_spans_and_finish_report_errors() {
        use crate::parser::LineStream;
        let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
        let mut peeked = super::MemchrLines::new("1.0 2.0 3.0 0 4\n");
        assert!(peeked.peek_line().is_some());
        let row = peeked.next_coord_row(2, 0).unwrap();
        assert_eq!(row.2, 4);
        let mut extra = super::MemchrLines::new("1 2 3 0 4 leftover\n");
        assert!(extra.next_coord_row(2, 0).is_err());
        let mut spaced = super::MemchrLines::new("1.0 2.0 3.0 0 4   \n");
        assert!(spaced.next_coord_row(2, 0).is_ok());
        let mut blank = super::MemchrLines::new("Cu\n");
        assert!(!crate::parser::LineStream::blank_line_opens_section(&mut blank));
        blank.peek_line();
        assert!(!blank.blank_line_opens_section());

        let mut indexed = super::ConFrameIterator::from_bytes(text.as_bytes());
        indexed.lines.newlines = Some(super::newline_positions(text.as_bytes(), 1));
        indexed.advance_lines_indexed(0).unwrap();

        let bom = format!("\u{feff}{text}");
        let mut marked = super::ConFrameIterator::new(&bom);
        let err = marked.next_with_raw_span(&bom).unwrap().unwrap_err();
        assert!(matches!(err, error::ParseError::ValidationError(_)));

        let frame = super::parse_byte_span(text.as_bytes(), 0, text.len()).unwrap();
        assert_eq!(
            super::finish_results(None, vec![Ok(frame.clone())])
                .unwrap()
                .len(),
            1
        );
        let pool = super::shared_pool(2).unwrap();
        assert!(
            super::finish_results(
                Some(pool.as_ref()),
                vec![Ok(frame.clone()), Err(error::ParseError::IncompleteFrame)],
            )
            .is_err()
        );
        let multi = std::fs::read("resources/test/tiny_multi_cuh2.con").unwrap();
        let span_err = super::parse_byte_span(&multi, 0, multi.len()).unwrap_err();
        assert!(span_err.to_string().contains("more than one frame"));
        let multi_text = std::str::from_utf8(&multi).unwrap();
        let text_err = super::parse_text_span(multi_text, 0, multi_text.len()).unwrap_err();
        assert!(text_err.to_string().contains("more than one frame"));
        let mut mid = super::MemchrLines::from_bytes(b"abcd\n");
        mid.newlines = Some(vec![4]);
        mid.pos = 2;
        let indexed = mid.next_line().expect("newline index yields the line");
        assert_eq!(indexed, "abcd");
        assert_eq!(mid.line_no, 1);
        let bytes = multi.as_slice();
        let parse = |start: usize, end: usize| super::parse_byte_span(bytes, start, end);
        let again = parse(0, bytes.len()).expect_err("the same span");
        assert!(again.to_string().contains("more than one frame"));
        let parsed = super::parse_into_slots_timed(bytes, bytes.len(), &[0], 2, parse);
        assert!(parsed.slots[0].is_err());
        let bad = text.replacen("0.63940000000000108", "notanumber", 1);
        assert!(super::read_nth_frame_from_text(&bad, 0).is_err());
        let mut other = frame.clone();
        other.atom_data.pop();
        let stride = frame.atom_data.len() * 3;
        let coords = vec![0.0; stride * 2];
        assert!(super::touch_frame_checksum(&[frame.clone(), other.clone()], &coords, 1).is_none());
        assert!(super::touch_option_checksum(
            &[Some(frame), Some(other)],
            &coords,
            1
        )
        .is_none());
        let mut empty = Vec::<f64>::new();
        super::first_touch_f64(&mut empty, 2);
        let mut cursor = super::MemchrLines::new("abc\ndef\n");
        cursor.newlines = Some(vec![3, 7]);
        cursor.peek_line();
        cursor.clear_peek();
        cursor.utf8_bad = true;
        assert!(!crate::parser::LineStream::remainder_is_whitespace(&mut cursor));
    }

    #[test]
    fn llc_reads_follow_the_supplied_descriptor() {
        assert_eq!(sample_llc_from(|| -1, |_, _| 8, |_| {}), -1);
        let mut closed = false;
        let value = sample_llc_from(
            || 7,
            |fd, slot| {
                assert_eq!(fd, 7);
                *slot = 42;
                8
            },
            |fd| {
                assert_eq!(fd, 7);
                closed = true;
            },
        );
        assert_eq!(value, 42);
        assert!(closed);
        assert_eq!(sample_llc_from(|| 4, |_, _| 3, |_| {}), -1);
        assert_eq!(sample_llc_from(|| 4, |_, _| -1, |_| {}), -1);
    }

    #[test]
    fn this_kernel_denies_the_real_perf_open() {
        let fd = perf_open_os();
        let err = std::io::Error::last_os_error();
        if fd >= 0 {
            perf_close_os(fd as i32);
            let value = sample_llc_misses();
            assert!(value >= -1);
        } else {
            let errno = err.raw_os_error();
            assert!(matches!(errno, Some(libc::EPERM | libc::EACCES)));
            assert_eq!(sample_llc_misses(), -1);
        }
    }

    #[test]
    fn remote_pages_sum_nodes_other_than_zero() {
        assert_eq!(numa_remote_from("0", "N1=9"), 0);
        assert_eq!(numa_remote_from("0-0", "N1=9"), 0);
        assert_eq!(numa_remote_from("nope-also", "N1=9"), 0);
        assert_eq!(numa_remote_from("0-1", "abc N0=3 N1=4 N2=bad Nx"), 4);
        assert_eq!(numa_remote_from("0-3", "N1=2\nN3=5 N0=1"), 7);
    }

    #[test]
    fn timed_parse_and_position_copy_cover_the_worker_arms() {
        let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
        let bytes = text.as_bytes();
        let pooled = parse_bytes_timed(bytes, Some(2));
        assert_eq!(pooled.slots.len(), 1);
        assert!(pooled.slots[0].is_ok());
        assert!(pooled.pool.is_some());
        let serial = parse_bytes_timed(bytes, Some(1));
        assert!(serial.pool.is_none());
        assert_eq!(serial.slots.len(), 1);
        assert!(serial.slots[0].is_ok());
        let defaulted = parse_bytes_timed(bytes, None);
        assert_eq!(defaulted.slots.len(), 1);
        assert!(defaulted.slots[0].is_ok());
        let frame = read_first_frame(std::path::Path::new("resources/test/tiny_cuh2.con")).unwrap();
        let mut short = vec![0.0; 3];
        assert!(matches!(
            copy_positions(&frame, &mut short),
            Err(error::ParseError::InvalidCount(_))
        ));
        let mut wide = vec![0.0; frame.positions.nrows() * 3];
        copy_positions(&frame, &mut wide).unwrap();
        assert_eq!(wide, frame.positions.f64_slice().unwrap());
        assert!(pack_position_slab(&[], 2).is_none());
        let mut one = crate::types::ConFrameBuilder::new([8.0, 8.0, 8.0], [90.0, 90.0, 90.0]);
        one.add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 1, 1.0);
        let mut two = crate::types::ConFrameBuilder::new([8.0, 8.0, 8.0], [90.0, 90.0, 90.0]);
        two.add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 1, 1.0)
            .add_atom("O", 1.0, 0.0, 0.0, [false, false, false], 8, 16.0);
        assert!(pack_position_slab(&[one.build().unwrap(), two.build().unwrap()], 3).is_none());
        let parsed = parse_byte_span(bytes, 0, bytes.len()).unwrap();
        assert_eq!(parsed.atom_data.len(), frame.atom_data.len());
        assert!(matches!(
            parse_byte_span(bytes, bytes.len(), bytes.len()),
            Err(error::ParseError::IncompleteFrame)
        ));
    }

    #[test]
    fn cold_span_and_slab_arms_return_the_documented_value() {
        let one = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
        assert!(super::equal_span_boundaries(&one, 1).is_none());
        assert!(super::equal_span_boundaries(b"a", 4).is_none());
        assert!(super::equal_span_boundaries(b"not a frame", 2).is_none());
        assert!(super::equal_span_boundaries(&one, 2).is_none());
        let mut mixed = one.clone();
        mixed.extend(std::iter::repeat(b'x').take(one.len()));
        assert!(super::equal_span_boundaries(&mixed, 2).is_none());
        let repeated = {
            let mut buf = Vec::new();
            for _ in 0..4 {
                buf.extend_from_slice(&one);
            }
            buf
        };
        let starts = super::equal_span_boundaries(&repeated, 64).expect("many workers");
        assert_eq!(starts.len(), 4);
        let again = super::equal_span_boundaries(&repeated, 2).expect("cached starts");
        assert_eq!(again, starts);

        let mut empty = [];
        super::first_touch_f64(&mut empty, 4);
        let mut one_slot = [1.0];
        super::first_touch_f64(&mut one_slot, 1);
        assert_eq!(one_slot[0], 1.0);
        let mut pages = vec![1.0; 64];
        super::first_touch_f64(&mut pages, 8);
        assert!(pages.iter().any(|v| *v == 0.0));

        let mut builder = crate::types::ConFrameBuilder::new([8.0, 8.0, 8.0], [90.0, 90.0, 90.0]);
        builder.add_atom("H", 1.5, 2.5, 3.5, [false, false, false], 1, 1.0);
        let mut frame = builder.build().unwrap();
        let mut dtypes = crate::storage_dtype::StorageDtypes::default();
        dtypes.positions = crate::storage_dtype::ElementKind::Float32;
        frame.project_storage_dtypes(&dtypes);
        assert!(frame.positions.f64_slice().is_none());
        let slab = pack_position_slab(&[frame.clone(), frame.clone()], 2).unwrap();
        assert!((slab[0] - 1.5).abs() < 1e-5);
        assert!((slab[3] - 1.5).abs() < 1e-5);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let from_pool = pool.install(|| pack_position_slab(&[frame.clone()], 2).unwrap());
        assert!((from_pool[1] - 2.5).abs() < 1e-5);

        let mut cursor = super::MemchrLines::from_bytes(b"   \n1 2 3 0 1\r\n");
        let err = crate::parser::LineStream::next_coord_row(&mut cursor, 2, 0).unwrap_err();
        assert!(matches!(
            err,
            error::ParseError::InvalidVectorLength { found: 0, .. }
        ));
        let row = crate::parser::LineStream::next_coord_row(&mut cursor, 2, 7).unwrap();
        assert_eq!(row.0, [1.0, 2.0, 3.0]);
        let mut done = super::MemchrLines::from_bytes(b"");
        assert!(matches!(
            crate::parser::LineStream::next_coord_row(&mut done, 2, 0).unwrap_err(),
            error::ParseError::IncompleteFrame
        ));
    }

}

#[cfg(test)]
mod missed_branch_tests {
    use super::*;
    use crate::parser::LineStream;

    fn one_frame() -> String {
        "user\n{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\"},\"sections\":[]}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n1\n1\n1.0\nH\nCoordinates of Component 1\n0.0 0.0 0.0 0 1\n".to_string()
    }

    #[test]
    fn cursor_edges_report_the_rejected_line() {
        let raw = b"ok\n\xff\nnext\n";
        let mut cursor = MemchrLines::from_bytes(raw);
        assert_eq!(cursor.next_line(), Some("ok"));
        assert!(cursor.next_line().is_none());
        assert!(cursor.utf8_error().is_some());
        assert!(cursor.next_line().is_none());

        let mut indexed = MemchrLines::from_bytes(raw);
        indexed.newlines = Some(vec![2, 4, 9]);
        assert_eq!(indexed.next_line(), Some("ok"));
        assert!(indexed.next_line().is_none());
        assert!(indexed.next_line().is_none());
        assert_eq!(indexed.line_no, 1);

        let mut indexed_peek = MemchrLines::new("ab\ncd\n");
        indexed_peek.newlines = Some(vec![2, 5]);
        assert_eq!(indexed_peek.peek_line(), Some("ab"));
        indexed_peek.clear_peek();
        assert_eq!(indexed_peek.pos, 0);
        assert_eq!(indexed_peek.peek_line(), Some("ab"));
        let mut plain_peek = MemchrLines::new("ab\ncd\n");
        assert!(plain_peek.newlines.is_none());
        assert_eq!(plain_peek.peek_line(), Some("ab"));
        plain_peek.clear_peek();
        assert_eq!(plain_peek.pos, 0);
        assert_eq!(plain_peek.peek_line(), Some("ab"));

        let mut tail = MemchrLines::new("ab\ncd");
        tail.newlines = Some(vec![2]);
        assert_eq!(tail.next_line(), Some("ab"));
        assert_eq!(tail.next_line(), Some("cd"));
        assert!(tail.next_line().is_none());
        assert_eq!(tail.line_no, 2);

        assert!(indexed_span(&[2], b"ab\ncd", 9).is_none());
        let span = indexed_span(&[2], b"ab\ncd", 1).unwrap();
        assert_eq!(&b"ab\ncd"[span.start..span.content_end], b"cd");

        assert!(matches!(
            probe_section(b"\xff\nH\n"),
            SectionProbe::InvalidUtf8
        ));
        assert!(matches!(
            probe_section(b"   \n\xff\n"),
            SectionProbe::InvalidUtf8
        ));
        assert!(matches!(
            probe_section(b"H\nvelocities of Component 1\n"),
            SectionProbe::Opens
        ));
        assert!(matches!(
            legacy_velocity_prefix(b"H\nvelocities of Component 1\n0 0 0 0 1\n", &[1]),
            LegacyVelocity::Complete
        ));

        let mut blank = MemchrLines::new("\nH\nvelocities of Component 1\n");
        assert!(blank.blank_opens_velocity_block());
    }

    #[test]
    fn skip_and_span_errors_name_the_failure() {
        let mut broken = ConFrameIterator::new("only\n");
        let err = broken.skip_frames(1).unwrap_err();
        assert!(matches!(err, error::ParseError::IncompleteHeader));

        let text = "only\n";
        let mut spanned = ConFrameIterator::new(text);
        let err = spanned.next_with_raw_span(text).unwrap().unwrap_err();
        assert!(matches!(err, error::ParseError::IncompleteHeader));

        let marked = format!("\u{feff}{}", one_frame());
        let mut allowed = ConFrameIterator::new(&marked);
        allowed.allow_mid_file_bom();
        let (frame, span) = allowed.next_with_raw_span(&marked).unwrap().unwrap();
        assert_eq!(frame.atom_data[0].symbol.as_ref(), "H");
        assert!(span.starts_with('\u{feff}'));

        let mut peeked = ConFrameIterator::new(&marked);
        assert!(peeked.lines.peek_line().unwrap().starts_with('\u{feff}'));
        let (frame, _) = peeked.next_with_raw_span(&marked).unwrap().unwrap();
        assert_eq!(frame.atom_data[0].symbol.as_ref(), "H");

        let err = parse_span(b"   ", 0, 3).unwrap_err();
        assert!(matches!(err, error::ParseError::IncompleteFrame));
        let bytes = one_frame().into_bytes();
        let err = fill_position_block(b"not a frame\n", Some(1)).err().unwrap();
        assert!(!err.to_string().is_empty());
        let block = fill_position_block(&bytes, None).unwrap();
        assert_eq!(block.n_frames, 1);
        assert_eq!(block.n_atoms, 1);
        assert_eq!(block.values, vec![0.0, 0.0, 0.0]);

        let mut trailing = one_frame().into_bytes();
        trailing.extend_from_slice(&[0xff, b'\n']);
        let mut iter = ConFrameIterator::from_bytes(&trailing);
        iter.forward_fast().unwrap().unwrap();
        let err = iter.next().unwrap().unwrap_err();
        assert!(
            err.to_string().to_ascii_lowercase().contains("utf-8"),
            "{err}"
        );
    }

    #[test]
    fn mapped_reads_and_worker_pools_match_the_serial_frame() {
        let text = one_frame();
        let (offsets, seconds) = measure_newline_scan(&text, 1, true);
        assert!(seconds >= 0.0);
        assert_eq!(offsets, frame_start_offsets_serial(&text));

        let empty = types::ConFrameBuilder::new([1.0, 1.0, 1.0], [90.0, 90.0, 90.0])
            .build()
            .unwrap();
        assert_eq!(empty.positions.nrows(), 0);
        let slab = pack_position_slab(&[empty], 2).unwrap();
        assert!(slab.is_empty());

        #[cfg(feature = "parallel")]
        {
            let bytes = text.as_bytes();
            let at_end =
                parse_into_slots_timed(bytes, bytes.len(), &[bytes.len()], 2, |start, end| {
                    parse_byte_span(bytes, start, end)
                });
            assert!(matches!(
                at_end.slots[0],
                Err(error::ParseError::IncompleteFrame)
            ));

            let wide = text.repeat(4);
            let wide_bytes = wide.as_bytes();
            let boundaries = collect_frame_starts(ConFrameIterator::from_bytes(wide_bytes), true);
            assert!(boundaries.len() >= 2);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .unwrap();
            pool.install(|| {
                let parsed = parse_into_slots_timed(
                    wide_bytes,
                    wide_bytes.len(),
                    &boundaries,
                    2,
                    |start, end| parse_byte_span(wide_bytes, start, end),
                );
                let frames = parsed
                    .slots
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                assert_eq!(frames.len(), boundaries.len());
                let packed = pack_position_slab(&frames, 2).unwrap();
                let checksum = touch_frame_checksum(&frames, &packed, 2).unwrap();
                let serial = touch_frame_checksum(&frames, &packed, 1).unwrap();
                assert_eq!(checksum, serial);
                assert_eq!(packed.len(), frames.len() * frames[0].positions.nrows() * 3);
            });
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.con");
        let mapped = text.repeat(400);
        std::fs::write(&path, &mapped).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 64 * 1024);
        let positions = read_positions_with_threads(&path, None).unwrap();
        assert_eq!(positions.n_frames, 400);
        assert_eq!(positions.values.len(), 400 * 3);
        assert_eq!(positions.values[0], 0.0);
        let indexed = read_all_frames_indexed(&path, Some(1)).unwrap();
        assert_eq!(indexed.len(), 400);
        assert_eq!(indexed[0].atom_data[0].x, 0.0);
        let serial = read_all_frames_phased(&path, None).unwrap();
        assert!(serial.phases.mapped);
        assert_eq!(serial.frames.len(), 400);
        let parallel = read_all_frames_phased(&path, Some(2)).unwrap();
        assert_eq!(parallel.frames.len(), 400);
        assert_eq!(
            parallel.frames[399].atom_data[0].atom_id,
            serial.frames[399].atom_data[0].atom_id
        );
        #[cfg(feature = "parallel")]
        assert!(parallel.pool.is_some());
    }

    #[test]
    fn a_stored_section_list_is_not_a_legacy_velocity_prefix() {
        let frame = ConFrameIterator::new(&one_frame()).next().unwrap().unwrap();
        let mut header = frame.header.clone();
        header.sections_declared = false;
        header.sections = vec!["velocities".into()];
        let mut it = ConFrameIterator::from_bytes(b"\n");
        assert!(!it.legacy_velocity_prefix_is_open(&header));
        header.sections_declared = true;
        header.sections.clear();
        assert!(!it.legacy_velocity_prefix_is_open(&header));
        header.sections_declared = false;
        let mut bare = ConFrameIterator::from_bytes(b"");
        assert!(!bare.legacy_velocity_prefix_is_open(&header));
        let mut word = ConFrameIterator::from_bytes(b"Cu\n");
        word.lines.peeked = Some("Cu");
        assert!(!word.legacy_velocity_prefix_is_open(&header));
    }

    #[test]
    fn the_second_span_is_not_at_the_start_of_the_buffer() {
        let text = one_frame() + &one_frame();
        let mut it = ConFrameIterator::new(&text);
        let (first, span) = it.next_with_raw_span(&text).unwrap().unwrap();
        assert_eq!(first.atom_data.len(), 1);
        assert!(span.starts_with("user\n"), "{span}");
        let (second, span2) = it.next_with_raw_span(&text).unwrap().unwrap();
        assert_eq!(&*second.atom_data[0].symbol, "H");
        assert!(span2.contains("Coordinates of Component 1"), "{span2}");
        assert!(it.next_with_raw_span(&text).is_none());
    }

    #[test]
    fn a_bad_label_byte_and_a_short_checksum_are_rejected() {
        assert!(matches!(
            probe_section(b"H\n\xff\n"),
            SectionProbe::InvalidUtf8
        ));
        let text = one_frame();
        let frame = ConFrameIterator::new(&text).next().unwrap().unwrap();
        assert_eq!(touch_frame_checksum(&[], &[], 1), Some(0));
        assert!(touch_frame_checksum(std::slice::from_ref(&frame), &[], 1).is_none());
        let mut empty = frame.clone();
        empty.atom_data.clear();
        assert!(touch_frame_checksum(std::slice::from_ref(&empty), &[], 1).is_none());
        assert!(touch_option_checksum(&[Some(empty)], &[], 1).is_none());
        let stride = frame.atom_data.len() * 3;
        let coords = vec![0.0; stride];
        let one = touch_frame_checksum(std::slice::from_ref(&frame), &coords, 1).unwrap();
        assert_ne!(one, 0);
        let missing = [Some(frame.clone()), None];
        assert!(touch_option_checksum(&missing, &coords, 1).is_none());
        let pair = [Some(frame.clone()), Some(frame.clone())];
        assert!(touch_option_checksum(&pair, &[], 1).is_none());
        let wide = vec![0.0; stride * 2];
        let both = touch_option_checksum(&pair, &wide, 1).unwrap();
        assert_ne!(both, 0);
        let mut extra = text.into_bytes();
        extra.push(b'x');
        assert!(equal_span_boundaries(&extra, 2).is_none());
        let peeked_text = one_frame();
        let mut peeked = ConFrameIterator::new(&peeked_text);
        assert!(peeked.lines.peek_line().is_some());
        let (got, span) = peeked
            .next_with_raw_span(&peeked_text)
            .unwrap()
            .unwrap();
        assert_eq!(&*got.atom_data[0].symbol, "H");
        assert!(span.contains("H"));
    }

    #[test]
    fn outlined_readers_take_the_remaining_sides() {
        let legacy = b"\
user
{\"con_spec_version\":2}
10.0 10.0 10.0
90.0 90.0 90.0


1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1

not-a-section
";
        let skipped = ConFrameIterator::from_bytes(legacy).forward_fast();
        assert!(skipped.unwrap().is_ok());

        let empty_decl = b"\
user
{\"con_spec_version\":2,\"sections\":[]}
10.0 10.0 10.0
90.0 90.0 90.0


1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1
\xff
";
        let bad = ConFrameIterator::from_bytes(empty_decl).forward_fast();
        assert!(bad.unwrap().is_ok());

        let partial = b"\
user
{\"con_spec_version\":2}
10.0 10.0 10.0
90.0 90.0 90.0


1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1

H";
        let cut = ConFrameIterator::from_bytes(partial).forward_fast();
        assert!(cut.unwrap().is_ok());

        let text = one_frame();
        let mut peeked = ConFrameIterator::new(&text);
        assert!(peeked.lines.peek_line().is_some());
        let spanned = peeked.next_with_raw_span(&text).unwrap().unwrap();
        assert_eq!(&*spanned.0.atom_data[0].symbol, "H");
        let mut again = ConFrameIterator::new(&text);
        assert!(again.lines.peek_line().is_some());
        let frame = again.next().unwrap().unwrap();
        assert_eq!(&*frame.atom_data[0].symbol, "H");

        let two = super::finish_results(None, vec![Ok(frame.clone()), Ok(frame)]).unwrap();
        assert_eq!(two.len(), 2);

        let first = super::arithmetic_starts(4, 3);
        assert_eq!(first, vec![0, 4, 8]);
        super::recycle_starts(first);
        let second = super::arithmetic_starts(4, 3);
        assert_eq!(second, vec![0, 4, 8]);

        use std::sync::atomic::{AtomicI64, Ordering};
        static OPEN_MODE: AtomicI64 = AtomicI64::new(-1);
        static READ_MODE: AtomicI64 = AtomicI64::new(8);
        fn open_mode() -> i64 {
            OPEN_MODE.load(Ordering::Relaxed)
        }
        fn read_mode(_fd: i32, slot: &mut u64) -> isize {
            let n = READ_MODE.load(Ordering::Relaxed);
            if n == 8 {
                *slot = 9;
            }
            n as isize
        }
        fn close_mode(_fd: i32) {}
        assert_eq!(super::sample_llc_from(open_mode, read_mode, close_mode), -1);
        OPEN_MODE.store(3, Ordering::Relaxed);
        READ_MODE.store(1, Ordering::Relaxed);
        assert_eq!(super::sample_llc_from(open_mode, read_mode, close_mode), -1);
        READ_MODE.store(8, Ordering::Relaxed);
        assert_eq!(super::sample_llc_from(open_mode, read_mode, close_mode), 9);

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = super::START_CACHE.lock().unwrap();
            panic!("poison the start cache");
        }));
        super::recycle_starts(vec![0, 4]);
        super::START_CACHE.clear_poison();
        assert!(super::START_CACHE.lock().is_ok());

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = super::slot_cache().lock().unwrap();
            panic!("poison the slot cache");
        }));
        super::recycle_slot_buf(Vec::new());
        super::slot_cache().clear_poison();
        assert!(super::slot_cache().lock().is_ok());

        let marked = one_frame();
        let mut held = ConFrameIterator::new(&marked);
        assert!(held.lines.peek_line().is_some());
        held.lines.pos = 0;
        assert!(held.lines.peeked.is_some());
        let held_err = held.next_with_raw_span(&marked).unwrap().unwrap_err();
        let held_text = format!("{held_err:?}");
        assert!(
            held_text.contains("InvalidNumberFormat"),
            "{held_text}"
        );

        let mut bad = ConFrameIterator::from_bytes(b"ok\n\xff\n");
        assert_eq!(bad.lines.next_line(), Some("ok"));
        assert!(bad.lines.next_line().is_none());
        let utf8_err = bad.next().unwrap().unwrap_err();
        let utf8_text = format!("{utf8_err:?}");
        assert!(utf8_text.contains("UTF-8"), "{utf8_text}");
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn a_later_equal_span_stops_before_the_slice_ends() {
        let first = one_frame();
        let mut probe = ConFrameIterator::new(&first);
        assert!(probe.forward_fast().unwrap().is_ok());
        let span = probe.byte_pos();
        assert_eq!(span, first.len());
        let mut second = String::from(
            "user\n{\"con_spec_version\":2,\"sections\":[]}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n1\n1\n1.0\nH\nCoordinates of Component 1\n0.0 0.0 0.0 0 1\n\nZ\n",
        );
        let mut early = ConFrameIterator::new(&second);
        assert!(early.forward_fast().unwrap().is_ok());
        let stopped = early.byte_pos();
        assert!(stopped < second.len());
        assert!(second.len() <= span);
        second.push_str(&"x".repeat(span - second.len()));
        let buffer = format!("{first}{second}");
        assert_eq!(buffer.len() % span, 0);
        assert!(super::equal_span_boundaries(buffer.as_bytes(), 2).is_none());
    }
}
