use crate::error::ParseError;
use crate::helpers::symbol_to_atomic_number;
use crate::types::{
    AtomDatum, ConFrame, FrameHeader, PreboxHeader, SECTION_CHARGES, SECTION_DISPLACEMENTS,
    SECTION_ENERGIES, SECTION_FORCES, SECTION_MAGMOMS, SECTION_SPINS, SECTION_SPREADS,
    SECTION_VELOCITIES, meta,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::iter::Peekable;
use std::sync::Arc;

/// Line source with peek for section detection (blank-line separators).
///
/// Implemented by the memchr cursor on the hot iterator path and by
/// `Peekable<I>` for tests / ad-hoc callers.
pub trait LineStream<'a> {
    fn next_line(&mut self) -> Option<&'a str>;
    fn peek_line(&mut self) -> Option<&'a str>;
    /// True when every remaining line, including the current peek, is
    /// empty or whitespace. Default is false so a one-line peek cannot
    /// hide a following section.
    fn remainder_is_whitespace(&mut self) -> bool {
        false
    }
    /// Called while a blank line is peeked. True when that blank begins a
    /// section: the next line is a symbol and the line after it is a
    /// component label. A blank that does not is line 1 of the next frame.
    fn blank_line_opens_section(&mut self) -> bool {
        false
    }

    /// With a blank line peeked, true when [`Self::blank_line_opens_section`]
    /// is true. A stream that cannot look ahead keeps the legacy rule and
    /// returns true.
    fn blank_opens_velocity_block(&mut self) -> bool {
        true
    }

    /// With a blank line peeked, true when every velocity row of the block
    /// has five fields. A short row means the blank is not a velocity block
    /// (CON-frame.7). Streams that cannot look ahead keep the legacy rule.
    fn legacy_velocity_rows_complete(&mut self, _counts: &[usize]) -> bool {
        true
    }

    /// Bytes not yet consumed, including a peeked line. `None` when the
    /// stream has no byte cursor. A declared atom count larger than this
    /// cannot appear in the file.
    fn bytes_remaining(&mut self) -> Option<usize> {
        None
    }

    /// One coordinate row, fused with the line scan when the stream can do it.
    ///
    /// The default pulls a `&str` line. [`crate::iterators::MemchrLines`]
    /// parses the row directly from the byte cursor so tokenize and numeric
    /// parse are one pass.
    fn next_coord_row(
        &mut self,
        spec_version: u32,
        default_id: u64,
    ) -> Result<([f64; 3], [bool; 3], u64), ParseError> {
        let line = self.next_line().ok_or(ParseError::IncompleteFrame)?;
        parse_coord_row(line, spec_version, default_id)
    }
}

impl<'a, I> LineStream<'a> for Peekable<I>
where
    I: Iterator<Item = &'a str> + Clone,
{
    #[inline]
    fn next_line(&mut self) -> Option<&'a str> {
        Iterator::next(self)
    }
    #[inline]
    fn peek_line(&mut self) -> Option<&'a str> {
        Peekable::peek(self).copied()
    }
    fn blank_line_opens_section(&mut self) -> bool {
        let mut probe = self.clone();
        let Some(blank) = probe.next_line() else {
            return false;
        };
        if !blank.trim().is_empty() {
            return false;
        }
        section_follows(&mut probe)
    }

    fn bytes_remaining(&mut self) -> Option<usize> {
        let mut n = 0usize;
        for line in self.clone() {
            n = n.saturating_add(line.len().saturating_add(1));
        }
        Some(n)
    }

    fn blank_opens_velocity_block(&mut self) -> bool {
        match self.peek_line() {
            Some(line) if line.trim().is_empty() => self.blank_line_opens_section(),
            _ => true,
        }
    }
}

/// True when `line` is `velocities of Component {index}` and the index is canonical.
///
/// A legacy velocity block opens only on that label (CON-frame.5). `01` is not
/// the index 1, and component 0 is not component 1 (CON-count.8).
pub(crate) fn line_has_component_label(line: &str, index: u32) -> bool {
    let mut words = line.split_ascii_whitespace();
    let Some(name) = words.next() else {
        return false;
    };
    name.eq_ignore_ascii_case("velocities")
        && words.next().is_some_and(|word| word.eq_ignore_ascii_case("of"))
        && words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("component"))
        && words.next().is_some_and(|got| got == index.to_string())
        && words.next().is_none()
}

fn section_follows<'a>(lines: &mut impl LineStream<'a>) -> bool {
    let Some(symbol) = lines.next_line() else {
        return false;
    };
    if symbol.trim().is_empty() || symbol.trim_start().starts_with('{') {
        return false;
    }
    let Some(label) = lines.next_line() else {
        return false;
    };
    line_has_component_label(label, 1)
}

/// Hot-path: parse up to 5 whitespace-separated f64s into a stack buffer.
/// Returns count of tokens actually present (before padding).
/// Pads `out[found..max]` from `defaults` when `found < max` and `found >= min`.
///
/// Single-pass over the line bytes: skip ASCII whitespace, then
/// [`fast_float2::parse_partial`] (Eisel–Lemire / SIMD-class decimal kernel)
/// with a token-boundary check. No `SplitWhitespace`, no per-token `&str`,
/// no heap `Vec`. Prefer this over allocating [`parse_line_of_n_f64`] on atom lines.
#[inline]
pub fn parse_line_of_range_f64_stack(
    line: &str,
    min: usize,
    max: usize,
    defaults: &[f64],
    out: &mut [f64; 5],
) -> Result<usize, ParseError> {
    debug_assert!(max <= 5 && min <= max && defaults.len() >= max);
    let bytes = line.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut found = 0usize;
    while found < max {
        while i < n && is_field_ws(bytes[i]) {
            i += 1;
        }
        if i >= n {
            break;
        }
        let end = bytes[i..]
            .iter()
            .position(|b| is_field_ws(*b))
            .map(|k| i + k)
            .unwrap_or(n);
        let token = std::str::from_utf8(&bytes[i..end]).unwrap_or("");
        if is_nonfinite_spelling(token) {
            return Err(ParseError::NonFiniteValue(token.to_string()));
        }
        let (val, consumed) = fast_float2::parse_partial::<f64, _>(&bytes[i..])
            .map_err(|_| ParseError::InvalidNumberFormat(format!("invalid float: {token}")))?;
        let next = i + consumed;
        // Reject partial tokens like "1.2abc" (must end at whitespace or EOS).
        if next != end {
            return Err(ParseError::InvalidNumberFormat(format!(
                "invalid float: {token}"
            )));
        }
        if !val.is_finite() {
            return Err(ParseError::NonFiniteValue(token.to_string()));
        }
        out[found] = val;
        found += 1;
        i = next;
    }
    while i < n && is_field_ws(bytes[i]) {
        i += 1;
    }
    // Reject trailing extra tokens (same contract as the old split loop).
    if i < n {
        return Err(ParseError::InvalidVectorLength {
            expected: max,
            found: found + 1,
        });
    }
    // `found` increments only while `found < max`, so it cannot exceed `max`.
    if found < min {
        return Err(ParseError::InvalidVectorLength {
            expected: max,
            found,
        });
    }
    while found < max {
        out[found] = defaults[found];
        found += 1;
    }
    Ok(found)
}

/// Parses a line of whitespace-separated f64 values using fast-float2.
///
/// This is the hot-path parser for coordinate and velocity lines. It uses
/// `fast_float2::parse` instead of `str::parse::<f64>()` for better throughput
/// on the numeric-heavy atom data lines. Fixed-width atom lines use
/// [`parse_line_of_range_f64_stack`] to avoid a heap `Vec` per line.
///
/// # Arguments
///
/// * `line` - A string slice representing a single line of data.
/// * `n` - The exact number of f64 values expected on the line.
pub fn parse_line_of_n_f64(line: &str, n: usize) -> Result<Vec<f64>, ParseError> {
    if n <= 5 {
        let defaults = [0.0f64; 5];
        let mut buf = [0.0f64; 5];
        parse_line_of_range_f64_stack(line, n, n, &defaults, &mut buf)?;
        return Ok(buf[..n].to_vec());
    }
    let mut values = Vec::with_capacity(n);
    for token in line.split_ascii_whitespace() {
        values.push(parse_finite_float_token(token)?);
    }
    if values.len() == n {
        Ok(values)
    } else {
        Err(ParseError::InvalidVectorLength {
            expected: n,
            found: values.len(),
        })
    }
}

/// Parses a line of whitespace-separated f64 values, accepting between `min`
/// and `max` values (inclusive). Returns a vector of exactly `max` elements,
/// padding with values from `defaults` when fewer than `max` are present.
///
/// Used for atom lines where column 5 (atom_index) is optional.
/// Prefer [`parse_line_of_range_f64_stack`] on the atom hot path (`max <= 5`).
pub fn parse_line_of_range_f64(
    line: &str,
    min: usize,
    max: usize,
    defaults: &[f64],
) -> Result<Vec<f64>, ParseError> {
    if max <= 5 {
        let mut buf = [0.0f64; 5];
        parse_line_of_range_f64_stack(line, min, max, defaults, &mut buf)?;
        return Ok(buf[..max].to_vec());
    }
    let mut values = Vec::with_capacity(max);
    for token in line.split_ascii_whitespace() {
        values.push(parse_finite_float_token(token)?);
    }
    if values.len() < min || values.len() > max {
        return Err(ParseError::InvalidVectorLength {
            expected: max,
            found: values.len(),
        });
    }
    while values.len() < max {
        let idx = values.len();
        values.push(defaults[idx]);
    }
    Ok(values)
}

/// CON-num.1 spellings. A leading sign is optional. Case is ignored.
/// Digit and dot tokens return before any letter compare so the atom
/// hot path does not allocate.
fn is_nonfinite_spelling(token: &str) -> bool {
    let rest = token
        .strip_prefix('+')
        .or_else(|| token.strip_prefix('-'))
        .unwrap_or(token);
    match rest.as_bytes().first() {
        Some(b) if b.is_ascii_digit() || *b == b'.' => return false,
        None => return false,
        _ => {}
    }
    rest.eq_ignore_ascii_case("nan")
        || rest.eq_ignore_ascii_case("inf")
        || rest.eq_ignore_ascii_case("infinity")
}

fn parse_finite_float_token(token: &str) -> Result<f64, ParseError> {
    if is_nonfinite_spelling(token) {
        return Err(ParseError::NonFiniteValue(token.to_string()));
    }
    let (val, consumed) = fast_float2::parse_partial::<f64, _>(token.as_bytes())
        .map_err(|_| ParseError::InvalidNumberFormat(format!("invalid float: {token}")))?;
    if consumed != token.len() {
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid float: {token}"
        )));
    }
    if !val.is_finite() {
        return Err(ParseError::NonFiniteValue(token.to_string()));
    }
    Ok(val)
}

fn skip_ascii_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_field_ws(bytes[i]) {
        i += 1;
    }
    i
}

fn take_token(line: &str, i: usize) -> Option<(&str, usize)> {
    let bytes = line.as_bytes();
    if i >= bytes.len() {
        return None;
    }
    let mut j = i;
    while j < bytes.len() && !is_field_ws(bytes[j]) {
        j += 1;
    }
    Some((&line[i..j], j))
}

/// Column 4. A sign, a decimal point, or a value outside the spec's set
/// is `InvalidFixedFlag` in every mode (CON-mask.1, CON-mask.5, D8).
fn parse_fixed_flag(token: &str, spec_version: u32) -> Result<[bool; 3], ParseError> {
    let bad = || {
        ParseError::InvalidFixedFlag(format!(
            "column 4 flag {token:?} is not a legal integer for spec {spec_version}"
        ))
    };
    if token.is_empty() || !token.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let val: u64 = token.parse::<u64>().map_err(|_| bad())?;
    if spec_version < 2 {
        return match val {
            0 => Ok([false, false, false]),
            1 => Ok([true, true, true]),
            _ => Err(bad()),
        };
    }
    if val > 7 {
        return Err(bad());
    }
    let v = val as u8;
    Ok([v & 1 != 0, v & 2 != 0, v & 4 != 0])
}

/// Column 5. Digits only, parsed as `u64`, never through `f64` (CON-id.1, D9).
/// A sign or a decimal point is `InvalidNumberFormat` at the token level.
fn parse_atom_id_token(token: &str) -> Result<u64, ParseError> {
    if token.is_empty() || !token.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid integer: {token}"
        )));
    }
    token.parse::<u64>().map_err(|_| {
        ParseError::InvalidNumberFormat(format!("integer out of range 0..=2^64-1: {token}"))
    })
}

/// Coordinate row: three finite floats, an integer flag, and an optional id.
/// Four columns take `default_id`, the 0-based position in the frame
/// (CON-id.9). Any other width is `InvalidVectorLength` (CON-num.9).
fn parse_coord_row(
    line: &str,
    spec_version: u32,
    default_id: u64,
) -> Result<([f64; 3], [bool; 3], u64), ParseError> {
    parse_vector_row(line, spec_version, Some(default_id))
}

/// Vector section row. The flag and id columns are required (exact width 5).
fn parse_vector_section_row(
    line: &str,
    spec_version: u32,
) -> Result<([f64; 3], [bool; 3], u64), ParseError> {
    parse_vector_row(line, spec_version, None)
}

fn parse_vector_row(
    line: &str,
    spec_version: u32,
    default_id: Option<u64>,
) -> Result<([f64; 3], [bool; 3], u64), ParseError> {
    // Success already consumed trailing spaces and tabs. Any other leftover
    // byte is an error from `parse_coord_fields`, not a second length check.
    let (row, _) = parse_coord_fields(line.as_bytes(), 0, spec_version, default_id)?;
    Ok(row)
}

/// Field separator inside a coordinate row. Newline ends the row.
#[inline(always)]
fn is_field_ws(b: u8) -> bool {
    // Separators are spaces and tabs (CON-token.3). A form feed or a
    // carriage return that does not end the line is part of the field.
    matches!(b, b' ' | b'\t')
}

#[inline(always)]
fn skip_field_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_field_ws(bytes[i]) {
        i += 1;
    }
    i
}

fn token_on_line(bytes: &[u8], i: usize) -> String {
    let mut j = i;
    while j < bytes.len() && !is_field_ws(bytes[j]) && bytes[j] != b'\n' {
        j += 1;
    }
    String::from_utf8_lossy(&bytes[i..j]).into_owned()
}

fn count_tokens_on_line(bytes: &[u8], mut i: usize) -> usize {
    let mut n = 0usize;
    loop {
        i = skip_field_ws(bytes, i);
        if i >= bytes.len() || bytes[i] == b'\n' {
            break;
        }
        n += 1;
        while i < bytes.len() && !is_field_ws(bytes[i]) && bytes[i] != b'\n' {
            i += 1;
        }
    }
    n
}

/// Invalid UTF-8 in a numeric row, otherwise a normal number-format error.
fn utf8_or_number(bytes: &[u8], line_start: usize, i: usize) -> ParseError {
    let end = bytes[line_start..]
        .iter()
        .position(|b| *b == b'\n')
        .map(|p| line_start + p)
        .unwrap_or(bytes.len());
    if std::str::from_utf8(&bytes[line_start..end]).is_err() {
        return ParseError::ValidationError("invalid UTF-8".into());
    }
    ParseError::InvalidNumberFormat(format!("invalid float: {}", token_on_line(bytes, i)))
}

#[inline(always)]
fn parse_f64_at(bytes: &[u8], i: usize, line_start: usize) -> Result<(f64, usize), ParseError> {
    if i >= bytes.len() || bytes[i] == b'\n' {
        return Err(ParseError::InvalidVectorLength {
            expected: 5,
            found: 0,
        });
    }
    if bytes[i] >= 0x80 {
        return Err(utf8_or_number(bytes, line_start, i));
    }
    if looks_like_nonfinite(bytes, i) {
        let token = token_on_line(bytes, i);
        if is_nonfinite_spelling(&token) {
            return Err(ParseError::NonFiniteValue(token));
        }
    }
    let (val, consumed) = fast_float2::parse_partial::<f64, _>(&bytes[i..]).map_err(|_| {
        ParseError::InvalidNumberFormat(format!("invalid float: {}", token_on_line(bytes, i)))
    })?;
    let next = i + consumed;
    let boundary = next >= bytes.len() || is_field_ws(bytes[next]) || bytes[next] == b'\n';
    if !boundary || !val.is_finite() {
        let token = token_on_line(bytes, i);
        // Exact non-finite spellings return before the numeric kernel. A
        // complete token that reached here is non-finite. A partial token
        // ("NaN.5", "1.2abc") is a format error (CON-num.1).
        if boundary {
            return Err(ParseError::NonFiniteValue(token));
        }
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid float: {token}"
        )));
    }
    Ok((val, next))
}

#[inline(always)]
fn parse_flag_at(
    bytes: &[u8],
    i: usize,
    spec_version: u32,
) -> Result<([bool; 3], usize), ParseError> {
    if i >= bytes.len() || bytes[i] == b'\n' {
        return Err(ParseError::InvalidVectorLength {
            expected: 5,
            found: 3,
        });
    }
    let bad = || {
        ParseError::InvalidFixedFlag(format!(
            "column 4 flag {:?} is not a legal integer for spec {spec_version}",
            token_on_line(bytes, i)
        ))
    };
    if !bytes[i].is_ascii_digit() {
        return Err(bad());
    }
    let mut j = i;
    let mut val: u64 = 0;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        let d = u64::from(bytes[j] - b'0');
        val = val
            .checked_mul(10)
            .and_then(|v| v.checked_add(d))
            .ok_or_else(bad)?;
        j += 1;
    }
    if j < bytes.len() && !is_field_ws(bytes[j]) && bytes[j] != b'\n' {
        return Err(bad());
    }
    let fixed = if spec_version < 2 {
        match val {
            0 => [false, false, false],
            1 => [true, true, true],
            _ => return Err(bad()),
        }
    } else if val <= 7 {
        let v = val as u8;
        [v & 1 != 0, v & 2 != 0, v & 4 != 0]
    } else {
        return Err(bad());
    };
    Ok((fixed, j))
}

#[inline(always)]
fn parse_id_at(bytes: &[u8], i: usize) -> Result<(u64, usize), ParseError> {
    if i >= bytes.len() || bytes[i] == b'\n' || !bytes[i].is_ascii_digit() {
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid integer: {}",
            token_on_line(bytes, i)
        )));
    }
    let mut j = i;
    let mut val: u64 = 0;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        let d = u64::from(bytes[j] - b'0');
        val = val
            .checked_mul(10)
            .and_then(|v| v.checked_add(d))
            .ok_or_else(|| {
                ParseError::InvalidNumberFormat(format!(
                    "integer out of range 0..=2^64-1: {}",
                    token_on_line(bytes, i)
                ))
            })?;
        j += 1;
    }
    if j < bytes.len() && !is_field_ws(bytes[j]) && bytes[j] != b'\n' {
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid integer: {}",
            token_on_line(bytes, i)
        )));
    }
    Ok((val, j))
}

/// One parsed coordinate row: xyz, fixed flags, and the atom id.
type ParsedCoord = ([f64; 3], [bool; 3], u64);

/// One coordinate or vector row starting at `i`.
///
/// Stops at newline or end of `bytes` and returns the index just past the
/// row (after the newline, when one was present). Floats, the flag, and the
/// id are parsed in that single scan. The success path does not build error
/// strings; those stay on the cold fallback.
#[inline(always)]
pub(crate) fn parse_coord_fields(
    bytes: &[u8],
    i: usize,
    spec_version: u32,
    default_id: Option<u64>,
) -> Result<(ParsedCoord, usize), ParseError> {
    if let Some(parsed) = parse_coord_fields_fast(bytes, i, spec_version, default_id) {
        return Ok(parsed);
    }
    parse_coord_fields_detailed(bytes, i, spec_version, default_id)
}

/// Common coordinate row: three finite decimals, a small flag, an optional id.
#[inline(always)]
fn parse_coord_fields_fast(
    bytes: &[u8],
    i: usize,
    spec_version: u32,
    default_id: Option<u64>,
) -> Option<(ParsedCoord, usize)> {
    let mut i = skip_field_ws(bytes, i);
    let mut xyz = [0.0f64; 3];
    for dest in &mut xyz {
        if i >= bytes.len() || bytes[i] == b'\n' || bytes[i] >= 0x80 {
            return None;
        }
        if looks_like_nonfinite(bytes, i) {
            return None;
        }
        let (val, consumed) = fast_float2::parse_partial::<f64, _>(&bytes[i..]).ok()?;
        if !val.is_finite() {
            return None;
        }
        let next = i + consumed;
        if next < bytes.len() && !is_field_ws(bytes[next]) && bytes[next] != b'\n' {
            return None;
        }
        *dest = val;
        i = skip_field_ws(bytes, next);
    }
    let (fixed, next) = parse_flag_fast(bytes, i, spec_version)?;
    i = skip_field_ws(bytes, next);
    let (atom_id, next) = if i >= bytes.len() || bytes[i] == b'\n' {
        (default_id?, i)
    } else {
        parse_u64_fast(bytes, i)?
    };
    i = skip_field_ws(bytes, next);
    if i < bytes.len() && bytes[i] != b'\n' {
        return None;
    }
    // The test above leaves a newline when any byte remains.
    if i < bytes.len() {
        i += 1;
    }
    Some(((xyz, fixed, atom_id), i))
}

#[inline(always)]
fn parse_u64_fast(bytes: &[u8], i: usize) -> Option<(u64, usize)> {
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let mut j = i;
    let mut val: u64 = 0;
    let mut digits = 0u32;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        digits += 1;
        // 19 digits can overflow u64; the detailed parser reports that.
        if digits > 18 {
            return None;
        }
        val = val * 10 + u64::from(bytes[j] - b'0');
        j += 1;
    }
    if j < bytes.len() && !is_field_ws(bytes[j]) && bytes[j] != b'\n' {
        return None;
    }
    Some((val, j))
}

#[inline(always)]
fn parse_flag_fast(bytes: &[u8], i: usize, spec_version: u32) -> Option<([bool; 3], usize)> {
    let (val, j) = parse_u64_fast(bytes, i)?;
    let fixed = if spec_version < 2 {
        match val {
            0 => [false, false, false],
            1 => [true, true, true],
            _ => return None,
        }
    } else if val <= 7 {
        let v = val as u8;
        [v & 1 != 0, v & 2 != 0, v & 4 != 0]
    } else {
        return None;
    };
    Some((fixed, j))
}

#[cold]
fn parse_coord_fields_detailed(
    bytes: &[u8],
    i: usize,
    spec_version: u32,
    default_id: Option<u64>,
) -> Result<(ParsedCoord, usize), ParseError> {
    let line_start = i;
    let mut i = skip_field_ws(bytes, i);
    if i >= bytes.len() || bytes[i] == b'\n' {
        return Err(ParseError::InvalidVectorLength {
            expected: 5,
            found: 0,
        });
    }
    let mut xyz = [0.0f64; 3];
    for (slot, dest) in xyz.iter_mut().enumerate() {
        if i >= bytes.len() || bytes[i] == b'\n' {
            return Err(ParseError::InvalidVectorLength {
                expected: 5,
                found: slot,
            });
        }
        if bytes[i] >= 0x80 {
            return Err(utf8_or_number(bytes, line_start, i));
        }
        let (val, next) = parse_f64_at(bytes, i, line_start)?;
        *dest = val;
        i = skip_field_ws(bytes, next);
    }
    let (fixed, next) = parse_flag_at(bytes, i, spec_version)?;
    i = skip_field_ws(bytes, next);
    let (atom_id, next) = if i >= bytes.len() || bytes[i] == b'\n' {
        (
            default_id.ok_or(ParseError::InvalidVectorLength {
                expected: 5,
                found: 4,
            })?,
            i,
        )
    } else {
        parse_id_at(bytes, i)?
    };
    i = skip_field_ws(bytes, next);
    if i < bytes.len() && bytes[i] != b'\n' {
        let found = 5 + count_tokens_on_line(bytes, i);
        return Err(ParseError::InvalidVectorLength { expected: 5, found });
    }
    // The test above leaves a newline when any byte remains.
    if i < bytes.len() {
        i += 1;
    }
    Ok(((xyz, fixed, atom_id), i))
}

/// Coordinate row at byte offset `i` in a multi-line buffer.
///
/// The returned index is the start of the next line. ASCII rows are valid
/// UTF-8 without a separate scan; a non-ASCII byte is checked only then.
pub(crate) fn parse_coord_row_at(
    bytes: &[u8],
    i: usize,
    spec_version: u32,
    default_id: u64,
) -> Result<(ParsedCoord, usize), ParseError> {
    if i >= bytes.len() {
        return Err(ParseError::IncompleteFrame);
    }
    parse_coord_fields(bytes, i, spec_version, Some(default_id))
}

/// Scalar section row: one finite float, a flag, and an id. Exact width 3.
fn parse_scalar_section_row(
    line: &str,
    spec_version: u32,
) -> Result<(f64, [bool; 3], u64), ParseError> {
    let bytes = line.as_bytes();
    let i = skip_ascii_ws(bytes, 0);
    let (value, i) = parse_finite_at(line, i)?;
    let i = skip_ascii_ws(bytes, i);
    let (flag_tok, i) = take_token(line, i).ok_or(ParseError::InvalidVectorLength {
        expected: 3,
        found: 1,
    })?;
    let fixed = parse_fixed_flag(flag_tok, spec_version)?;
    let i = skip_ascii_ws(bytes, i);
    let (id_tok, i) = take_token(line, i).ok_or(ParseError::InvalidVectorLength {
        expected: 3,
        found: 2,
    })?;
    let atom_id = parse_atom_id_token(id_tok)?;
    let i = skip_ascii_ws(bytes, i);
    if i < bytes.len() {
        let found = 3 + line[i..].split_ascii_whitespace().count();
        return Err(ParseError::InvalidVectorLength { expected: 3, found });
    }
    Ok((value, fixed, atom_id))
}

fn parse_finite_at(line: &str, i: usize) -> Result<(f64, usize), ParseError> {
    let bytes = line.as_bytes();
    if i >= bytes.len() {
        return Err(ParseError::InvalidVectorLength {
            expected: 3,
            found: 0,
        });
    }
    // "infinity" parses as the prefix "inf" in fast_float2. Name that
    // spelling before the numeric kernel so the leftover is not a format error.
    if looks_like_nonfinite(bytes, i) {
        let token = take_token(line, i).map(|(tok, _)| tok).unwrap_or("");
        if is_nonfinite_spelling(token) {
            return Err(ParseError::NonFiniteValue(token.to_string()));
        }
    }
    let (val, consumed) = fast_float2::parse_partial::<f64, _>(&bytes[i..]).map_err(|_| {
        let token = take_token(line, i).map(|(tok, _)| tok).unwrap_or("");
        ParseError::InvalidNumberFormat(format!("invalid float: {token}"))
    })?;
    let next = i + consumed;
    let boundary = next == bytes.len() || is_field_ws(bytes[next]);
    if !boundary || !val.is_finite() {
        let token = take_token(line, i).map(|(tok, _)| tok).unwrap_or("");
        // Exact non-finite spellings return before the numeric kernel. A
        // complete token that reached here is non-finite.
        if boundary {
            return Err(ParseError::NonFiniteValue(token.to_string()));
        }
        return Err(ParseError::InvalidNumberFormat(format!(
            "invalid float: {token}"
        )));
    }
    Ok((val, next))
}

/// True when the token may be a non-finite spelling. Digit and dot
/// tokens, the common atom-line case, are false after one byte.
fn looks_like_nonfinite(bytes: &[u8], i: usize) -> bool {
    let mut j = i;
    if bytes[j] == b'+' || bytes[j] == b'-' {
        j += 1;
        if j >= bytes.len() {
            return false;
        }
    }
    let b = bytes[j];
    !b.is_ascii_digit() && b != b'.'
}

/// Case-insensitive, after collapsing each run of spaces (CON-file.5, D17).
fn labels_equal(found: &str, expected: &str) -> bool {
    let mut found_words = found.split_ascii_whitespace();
    let mut expected_words = expected.split_ascii_whitespace();
    loop {
        match (found_words.next(), expected_words.next()) {
            (Some(a), Some(b)) => {
                if !a.eq_ignore_ascii_case(b) {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// True when `line`, after the same folding, starts with every word of `prefix`.
fn label_has_prefix(line: &str, prefix: &str) -> bool {
    let mut found_words = line.split_ascii_whitespace();
    for word in prefix.split_ascii_whitespace() {
        match found_words.next() {
            Some(got) if got.eq_ignore_ascii_case(word) => {}
            _ => return false,
        }
    }
    true
}

pub(crate) fn token_count(line: &str) -> usize {
    line.split(|c: char| c == ' ' || c == '\t')
        .filter(|token| !token.is_empty())
        .count()
}

fn json_u64(value: &Value) -> Option<u64> {
    if let Some(n) = value.as_u64() {
        return Some(n);
    }
    // 3.0 and 3e0 are JSON numbers that name an integer (CON-line2.4).
    let f = value.as_f64()?;
    // JSON numbers are finite. `as_f64` is `None` for every other value.
    if f >= 0.0 && f <= u64::MAX as f64 && f.fract() == 0.0 {
        Some(f as u64)
    } else {
        None
    }
}

fn json_negative_integer(value: &Value) -> bool {
    if let Some(n) = value.as_i64() {
        return n < 0;
    }
    let Some(f) = value.as_f64() else {
        return false;
    };
    f < 0.0 && f.fract() == 0.0
}

fn json_nonneg_integer(value: &Value) -> bool {
    if value.as_u64().is_some() {
        return true;
    }
    // A minor has no upper bound (CON-line2.7). 2^64 does not fit in u64
    // and still names a non-negative integer. 0.0 fits in u64, so a float
    // spelling of an in-range integer is not a minor (CON-line2.9).
    let Some(f) = value.as_f64() else {
        return false;
    };
    f >= u64::MAX as f64 && f.fract() == 0.0
}

fn validate_section_arity_value(value: &Value) -> Result<(), ParseError> {
    let obj = value.as_object().ok_or_else(|| {
        metadata_json_error("section_arity must be an object mapping section names to integers")
    })?;
    for (name, entry) in obj {
        let n = json_u64(entry).ok_or_else(|| {
            metadata_json_error(format!("section_arity.{name} must be an integer >= 1"))
        })?;
        if n < 1 {
            return Err(ParseError::InvalidCount(format!(
                "section_arity.{name} must be >= 1"
            )));
        }
        // Reserved names have a fixed value-field count (CON-section.13).
        if let Some(expect) = crate::types::reserved_section_value_arity(name)
            && n != expect
        {
            return Err(ParseError::InvalidVectorLength {
                expected: usize::try_from(expect).unwrap_or(usize::MAX),
                found: usize::try_from(n).unwrap_or(usize::MAX),
            });
        }
    }
    Ok(())
}

/// Integer counts. A sign, a decimal point, or any other non-digit token
/// is `InvalidCount` (CON-error.3). Zero is a legal token here. The caller
/// rejects a per-type count of 0 when the type count is 1 or more (D4).
pub(crate) fn parse_count_line(line: &str, n: usize) -> Result<Vec<usize>, ParseError> {
    // One token needs at least one byte. Allocating `n` slots before reading
    // the line is what turns a declared count of 2^64 into a multi-terabyte
    // buffer.
    if n > line.len() {
        return Err(ParseError::InvalidVectorLength {
            expected: n,
            found: line.split_whitespace().count(),
        });
    }
    let mut values = Vec::with_capacity(n);
    for token in line.split_ascii_whitespace() {
        // `split_ascii_whitespace` does not yield an empty token.
        if !token.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseError::InvalidCount(format!(
                "count field is not an unsigned integer: {token}"
            )));
        }
        let val: u64 = token.parse::<u64>().map_err(|_| {
            ParseError::InvalidCount(format!("count field is out of range: {token}"))
        })?;
        // A `u64` count fits in `usize` on this width. A narrower target
        // still rejects a count that does not fit the address size.
        #[cfg(target_pointer_width = "64")]
        let val = val as usize;
        #[cfg(not(target_pointer_width = "64"))]
        let val = usize::try_from(val).map_err(|_| {
            ParseError::InvalidCount(format!("count field is out of range: {token}"))
        })?;
        values.push(val);
    }
    if values.len() == n {
        Ok(values)
    } else {
        Err(ParseError::InvalidVectorLength {
            expected: n,
            found: values.len(),
        })
    }
}

/// True when `line` has no tokens. Spaces and tabs count as empty.
pub(crate) fn line_is_whitespace(line: &str) -> bool {
    line.split_ascii_whitespace().next().is_none()
}

/// Lines 8 and 9 of a type count of 0.
///
/// A token on either line is `InvalidCount`. At end of file, a missing
/// line is empty (CON-count.4), and that frame is not `IncompleteHeader`
/// (CON-frame.3). Mid-file, both lines are required.
pub(crate) fn read_zero_atom_tail<'a>(
    mut next_line: impl FnMut() -> Option<&'a str>,
) -> Result<(), ParseError> {
    let bad = || {
        ParseError::InvalidCount("a zero-atom frame requires whitespace-only lines 8 and 9".into())
    };
    match next_line() {
        None => Ok(()),
        Some(line) if line_is_whitespace(line) => match next_line() {
            None => Ok(()),
            Some(line) if line_is_whitespace(line) => Ok(()),
            Some(_) => Err(bad()),
        },
        Some(_) => Err(bad()),
    }
}

fn metadata_json_error(message: impl Into<String>) -> ParseError {
    ParseError::InvalidMetadataJson(message.into())
}

fn validate_metadata_number(key: &str, value: &Value) -> Result<(), ParseError> {
    if value.as_f64().is_some() {
        Ok(())
    } else {
        Err(metadata_json_error(format!(
            "{key} must be a finite number"
        )))
    }
}

fn validate_metadata_integer(key: &str, value: &Value) -> Result<(), ParseError> {
    if value.as_u64().is_some() {
        Ok(())
    } else {
        Err(metadata_json_error(format!(
            "{key} must be a non-negative integer"
        )))
    }
}

/// Validates a parsed metadata JSON object against the spec v2 schema.
///
/// Type-checks the `validate` and `sections` keys, then runs per-key
/// schema checks for the recommended metadata keys. Used by the
/// builder/Python `set_metadata_json` paths to fail fast on malformed
/// input, and by the parser when the file requested strict validation
/// via `"validate": true`.
pub fn validate_metadata_schema(
    json_obj: &serde_json::Map<String, Value>,
) -> Result<(), ParseError> {
    let strict_requested = match json_obj.get(meta::VALIDATE) {
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(metadata_json_error("validate must be a boolean")),
        None => false,
    };

    match json_obj.get(meta::SECTIONS) {
        Some(Value::Array(values)) => {
            if values.iter().any(|entry| !entry.is_string()) {
                return Err(metadata_json_error("sections must be an array of strings"));
            }
        }
        Some(_) => return Err(metadata_json_error("sections must be an array of strings")),
        None if strict_requested => {
            return Err(metadata_json_error(
                "validate=true requires a sections array, even when empty",
            ));
        }
        None => {}
    }

    for (key, value) in json_obj {
        match key.as_str() {
            meta::CON_SPEC_VERSION | meta::SECTIONS | meta::VALIDATE => {}
            meta::CON_SPEC_MINOR => {
                if !json_nonneg_integer(value) {
                    return Err(metadata_json_error(
                        "con_spec_minor must be a JSON integer >= 0",
                    ));
                }
            }
            meta::SECTION_ARITY => validate_section_arity_value(value)?,
            meta::ENERGY
            | meta::TIME
            | meta::TIMESTEP
            | meta::CONVERGENCE_FMAX
            | meta::CONVERGENCE_ENERGY
            | meta::FMAX => validate_metadata_number(key, value)?,
            meta::FRAME_INDEX | meta::NEB_BEAD | meta::NEB_BAND => {
                validate_metadata_integer(key, value)?
            }
            meta::GENERATOR if !value.is_string() => {
                return Err(metadata_json_error("generator must be a string"));
            }
            meta::GENERATOR => {}
            meta::UNITS | meta::POTENTIAL if !value.is_object() => {
                return Err(metadata_json_error(format!("{key} must be an object")));
            }
            meta::POTENTIAL => {
                if let Some(potential_type) = value.get("type")
                    && !potential_type.is_string()
                {
                    return Err(metadata_json_error("potential.type must be a string"));
                }
            }
            meta::UNITS => {
                // Full v3 checks (required keys) run when con_spec_version >= 3.
            }
            meta::PBC => validate_pbc_metadata(value)?,
            meta::BONDS => validate_bonds_metadata(value)?,
            meta::LATTICE_VECTORS => validate_lattice_vectors_metadata(value)?,
            meta::CONVERGED if !value.is_boolean() => {
                return Err(metadata_json_error("converged must be a boolean"));
            }
            meta::CONVERGED => {}
            _ => {}
        }
    }

    Ok(())
}

fn validate_pbc_metadata(value: &Value) -> Result<(), ParseError> {
    let Some(values) = value.as_array() else {
        return Err(metadata_json_error("pbc must be a length-3 boolean array"));
    };
    if values.len() != 3 || values.iter().any(|entry| !entry.is_boolean()) {
        return Err(metadata_json_error("pbc must be a length-3 boolean array"));
    }
    Ok(())
}

/// Validate optional `bonds` frame topology metadata.
///
/// Each element is either a length-2 non-negative integer pair `[i, j]` or an
/// object `{"i": i, "j": j, "order"?: integer}`. Indices are 0-based into
/// `atom_data` order (parser does not yet know atom count at metadata time;
/// bounds are enforced when projecting to chemfiles / selection).
fn validate_bonds_metadata(value: &Value) -> Result<(), ParseError> {
    let Some(items) = value.as_array() else {
        return Err(metadata_json_error("bonds must be an array"));
    };
    for (idx, item) in items.iter().enumerate() {
        if let Some(pair) = item.as_array() {
            if pair.len() != 2 {
                return Err(metadata_json_error(format!(
                    "bonds[{idx}] pair must have exactly two indices"
                )));
            }
            for (k, entry) in pair.iter().enumerate() {
                let Some(n) = entry.as_u64() else {
                    return Err(metadata_json_error(format!(
                        "bonds[{idx}][{k}] must be a non-negative integer"
                    )));
                };
                if n > u32::MAX as u64 {
                    return Err(metadata_json_error(format!(
                        "bonds[{idx}][{k}] index exceeds u32"
                    )));
                }
            }
            continue;
        }
        if let Some(obj) = item.as_object() {
            for key in ["i", "j"] {
                let Some(entry) = obj.get(key) else {
                    return Err(metadata_json_error(format!(
                        "bonds[{idx}] object must include \"{key}\""
                    )));
                };
                let Some(n) = entry.as_u64() else {
                    return Err(metadata_json_error(format!(
                        "bonds[{idx}].{key} must be a non-negative integer"
                    )));
                };
                if n > u32::MAX as u64 {
                    return Err(metadata_json_error(format!(
                        "bonds[{idx}].{key} index exceeds u32"
                    )));
                }
            }
            if let Some(order) = obj.get("order")
                && order.as_i64().is_none()
            {
                return Err(metadata_json_error(format!(
                    "bonds[{idx}].order must be an integer when present"
                )));
            }
            continue;
        }
        return Err(metadata_json_error(format!(
            "bonds[{idx}] must be [i, j] or {{\"i\": i, \"j\": j, \"order\"?: ...}}"
        )));
    }
    Ok(())
}

fn validate_lattice_vectors_metadata(value: &Value) -> Result<(), ParseError> {
    let Some(rows) = value.as_array() else {
        return Err(metadata_json_error(
            "lattice_vectors must be a 3x3 numeric array",
        ));
    };
    if rows.len() != 3 {
        return Err(metadata_json_error(
            "lattice_vectors must be a 3x3 numeric array",
        ));
    }
    for row in rows {
        let Some(entries) = row.as_array() else {
            return Err(metadata_json_error(
                "lattice_vectors must be a 3x3 numeric array",
            ));
        };
        if entries.len() != 3 || entries.iter().any(|entry| entry.as_f64().is_none()) {
            return Err(metadata_json_error(
                "lattice_vectors must be a 3x3 numeric array",
            ));
        }
    }
    Ok(())
}

/// Parses a line of whitespace-separated values into a vector of a specific type.
///
/// This generic helper function takes a string slice, splits it by whitespace,
/// and attempts to parse each substring into the target type `T`. The type `T`
/// must implement `std::str::FromStr`.
///
/// # Arguments
///
/// * `line` - A string slice representing a single line of data.
/// * `n` - The exact number of values expected on the line.
///
/// # Errors
///
/// * `ParseError::InvalidVectorLength` if the number of parsed values is not equal to `n`.
/// * Propagates any error from the `parse()` method of the target type `T`.
///
/// # Example
///
/// ```
/// use readcon_core::parser::parse_line_of_n;
/// let line = "10.5 20.0 30.5";
/// let values: Vec<f64> = parse_line_of_n(line, 3).unwrap();
/// assert_eq!(values, vec![10.5, 20.0, 30.5]);
///
/// let result = parse_line_of_n::<i32>(line, 2);
/// assert!(result.is_err());
/// ```
pub fn parse_line_of_n<T: std::str::FromStr>(line: &str, n: usize) -> Result<Vec<T>, ParseError>
where
    ParseError: From<<T as std::str::FromStr>::Err>,
{
    let values: Vec<T> = line
        .split_whitespace()
        .map(|s| s.parse::<T>())
        .collect::<Result<_, _>>()?;

    if values.len() == n {
        Ok(values)
    } else {
        Err(ParseError::InvalidVectorLength {
            expected: n,
            found: values.len(),
        })
    }
}

/// Parses the 9-line header of a `.con` file frame from an iterator.
///
/// This function consumes the next 9 lines from the given line iterator to
/// construct a `FrameHeader`. The iterator is advanced by 9 lines on success.
///
/// # Arguments
///
/// * `lines` - A mutable reference to an iterator that yields string slices.
///
/// # Errors
///
/// * `ParseError::IncompleteHeader` if the iterator has fewer than 9 lines remaining.
/// * Propagates any errors from `parse_line_of_n` if the numeric data within
///   the header is malformed.
///
/// # Panics
///
/// This function will panic if the intermediate vectors for box dimensions or angles,
/// after being successfully parsed, cannot be converted into fixed-size arrays.
/// This should not happen if `parse_line_of_n` is used correctly with `n=3`.
pub fn parse_frame_header<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
) -> Result<FrameHeader, ParseError> {
    let prebox1 = lines
        .next()
        .ok_or(ParseError::IncompleteHeader)?
        .to_string();
    let prebox2_raw = lines.next().ok_or(ParseError::IncompleteHeader)?;

    // Line 2: if it starts with '{', parse as JSON metadata (spec v2+).
    // Otherwise treat as a legacy (pre-v2) file with spec_version = 1.
    // One JSON object and nothing else (CON-line2.1). serde_json::from_str
    // rejects trailing junk after the value.
    let trimmed = prebox2_raw.trim_matches([' ', '\t']);
    let (spec_version, metadata, sections, validate, sections_declared) =
        if trimmed.starts_with('{') {
            // A value that starts with '{' and parses is a JSON object.
            let json_obj: serde_json::Map<String, Value> = serde_json::from_str(trimmed)
                .map_err(|e| ParseError::InvalidMetadataJson(e.to_string()))?;
            let ver_value = json_obj
                .get(meta::CON_SPEC_VERSION)
                .ok_or(ParseError::MissingSpecVersion)?;
            let ver = if let Some(ver) = json_u64(ver_value) {
                if ver > u64::from(u32::MAX) {
                    return Err(ParseError::UnsupportedVersion(u32::MAX));
                }
                ver as u32
            } else if json_negative_integer(ver_value) {
                // A negative integer names no major (CON-line2.6).
                return Err(ParseError::UnsupportedVersion(0));
            } else {
                return Err(ParseError::InvalidMetadataJson(
                    "con_spec_version must be a JSON integer, the major version".into(),
                ));
            };
            // JSON majors 0 and 1 are UnsupportedVersion (CON-line2.5).
            // An unknown major is UnsupportedVersion (CON-line2.6).
            // Unknown minors of a known major are accepted (CON-line2.7, CON-status.1).
            if ver == 0 || ver == 1 || ver > crate::CON_SPEC_VERSION {
                return Err(ParseError::UnsupportedVersion(ver));
            }
            if let Some(time) = json_obj
                .get(meta::UNITS)
                .and_then(|units| units.get("time"))
                .and_then(|value| value.as_str())
            {
                crate::units::validate_unit_for_quantity("time", time).map_err(|err| {
                    ParseError::InvalidUnits(format!("units.time: {err}"))
                })?;
            }
            if ver >= 3 {
                match json_obj.get(meta::UNITS) {
                    Some(u) => crate::units::validate_v3_units_metadata(u)
                        .map_err(|e| ParseError::InvalidUnits(format!("v3 units: {e}")))?,
                    None => {
                        return Err(ParseError::InvalidUnits(
                        "con_spec_version >= 3 requires metadata \"units\" with length and energy"
                            .into(),
                    ));
                    }
                }
            }

            // Single pass over the JSON object: collect sections, capture the
            // validate flag, copy the rest into metadata. Folds the previous
            // pre-extract get(validate) + re-iterate pattern into one walk.
            let mut sections: Vec<String> = Vec::new();
            let mut metadata = BTreeMap::new();
            let mut sections_declared = false;
            let mut validate = false;
            for (k, v) in &json_obj {
                match k.as_str() {
                    meta::CON_SPEC_VERSION => {}
                    meta::CON_SPEC_MINOR => {
                        // 0.0 is a JSON number that is not an integer (CON-line2.9).
                        if !json_nonneg_integer(v) {
                            return Err(metadata_json_error(
                                "con_spec_minor must be a JSON integer >= 0",
                            ));
                        }
                        metadata.insert(k.clone(), v.clone());
                    }
                    meta::SECTION_ARITY => {
                        validate_section_arity_value(v)?;
                        metadata.insert(k.clone(), v.clone());
                    }
                    meta::SECTIONS => {
                        sections_declared = true;
                        let arr = v.as_array().ok_or_else(|| {
                            metadata_json_error("sections must be an array of strings")
                        })?;
                        sections.reserve(arr.len());
                        for entry in arr {
                            let s = entry.as_str().ok_or_else(|| {
                                metadata_json_error("sections must be an array of strings")
                            })?;
                            sections.push(s.to_string());
                        }
                    }
                    meta::VALIDATE => {
                        validate = match v {
                            Value::Bool(b) => *b,
                            _ => return Err(metadata_json_error("validate must be a boolean")),
                        };
                        metadata.insert(k.clone(), v.clone());
                    }
                    _ => {
                        metadata.insert(k.clone(), v.clone());
                    }
                }
            }

            // Strict-mode schema check fires only when the file requested
            // it. Hot-path parses (validate=false) skip the per-key match.
            // Bonds of an unknown shape stay in the metadata (CON-bonds.3).
            if let Some(pbc) = json_obj.get(meta::PBC) {
                validate_pbc_metadata(pbc)?;
            }
            if let Some(lattice) = json_obj.get(meta::LATTICE_VECTORS) {
                validate_lattice_vectors_metadata(lattice)?;
            }
            if let Some(energy) = json_obj.get(meta::ENERGY) {
                validate_metadata_number(meta::ENERGY, energy)?;
            }
            // Reserved numeric keys are typed in every mode (CON-line2.13).
            for key in [
                meta::TIME,
                meta::TIMESTEP,
                meta::CONVERGENCE_FMAX,
                meta::CONVERGENCE_ENERGY,
                meta::FMAX,
            ] {
                if let Some(value) = json_obj.get(key) {
                    validate_metadata_number(key, value)?;
                }
            }
            for key in [meta::FRAME_INDEX, meta::NEB_BEAD, meta::NEB_BAND] {
                if let Some(value) = json_obj.get(key) {
                    validate_metadata_integer(key, value)?;
                }
            }
            if validate {
                validate_metadata_schema(&json_obj)?;
            }

            (ver, metadata, sections, validate, sections_declared)
        } else {
            // Legacy file: no JSON metadata line.
            (1_u32, BTreeMap::new(), Vec::new(), false, false)
        };
    let prebox2 = prebox2_raw.to_string();
    let minor = metadata
        .get(meta::CON_SPEC_MINOR)
        .and_then(json_u64)
        .unwrap_or(0);
    reject_unknown_section_names(&sections, minor, &metadata)?;
    if spec_version >= 3
        && minor >= 1
        && sections.iter().any(|name| name == "velocities")
    {
        let time_ok = metadata
            .get(meta::UNITS)
            .and_then(|units| units.get("time"))
            .and_then(|value| value.as_str())
            .is_some_and(|unit| !unit.trim().is_empty());
        if !time_ok {
            return Err(ParseError::InvalidUnits(
                "minor >= 1 requires units.time when a velocities section is present".into(),
            ));
        }
    }

    let boxl_vec = parse_line_of_n_f64(lines.next().ok_or(ParseError::IncompleteHeader)?, 3)?;
    let angles_vec = parse_line_of_n_f64(lines.next().ok_or(ParseError::IncompleteHeader)?, 3)?;
    let boxl_arr: [f64; 3] = boxl_vec.as_slice().try_into().unwrap();
    let angles_arr: [f64; 3] = angles_vec.as_slice().try_into().unwrap();
    // Line 4 is checked before later lines (CON-error.4). A radicand at or
    // below -1e-12 is not a cell, with or without validate.
    crate::types::cell_vectors(boxl_arr, angles_arr)?;
    let postbox1 = lines
        .next()
        .ok_or(ParseError::IncompleteHeader)?
        .to_string();
    let postbox2 = lines
        .next()
        .ok_or(ParseError::IncompleteHeader)?
        .to_string();
    let natm_types_line = lines.next().ok_or(ParseError::IncompleteHeader)?;
    let natm_types = parse_count_line(natm_types_line, 1)?[0];
    // A type count of 0 is a zero-atom frame (CON-count.1, CON-count.4).
    // A per-type count of 0 is InvalidCount only when the type count is >= 1.
    let (natms_per_type, masses_per_type) = if natm_types == 0 {
        read_zero_atom_tail(|| lines.next())?;
        (Vec::new(), Vec::new())
    } else {
        let natms_per_type = parse_count_line(
            lines.next().ok_or(ParseError::IncompleteHeader)?,
            natm_types,
        )?;
        if natms_per_type.contains(&0) {
            return Err(ParseError::InvalidCount(
                "each per-type count must be >= 1".into(),
            ));
        }
        let masses_per_type = parse_line_of_n_f64(
            lines.next().ok_or(ParseError::IncompleteHeader)?,
            natm_types,
        )?;
        (natms_per_type, masses_per_type)
    };
    if validate {
        validate_header_geometry(&boxl_vec, &angles_vec)?;
        validate_masses(&masses_per_type)?;
    }
    let header = FrameHeader {
        prebox_header: PreboxHeader {
            user: prebox1,
            metadata_line: prebox2,
        },
        boxl: boxl_vec.try_into().unwrap(),
        angles: angles_vec.try_into().unwrap(),
        postbox_header: [postbox1, postbox2],
        natm_types,
        natms_per_type,
        masses_per_type,
        spec_version,
        metadata,
        sections,
        strict_validation: validate,
        sections_declared,
    };
    // Line 2 is before the cell and the coordinate block (CON-error.4).
    reject_unknown_sections(&header)?;
    reject_velocities_without_time(&header)?;
    Ok(header)
}

fn known_section(name: &str) -> bool {
    matches!(
        name,
        SECTION_VELOCITIES
            | SECTION_FORCES
            | SECTION_ENERGIES
            | SECTION_CHARGES
            | SECTION_SPINS
            | SECTION_MAGMOMS
            | SECTION_DISPLACEMENTS
            | SECTION_SPREADS
    )
}

/// An unknown section name is rejected on line 2, unless a newer minor
/// gives it an arity and the reader skips the block (CON-section.9).
fn reject_unknown_section_names(
    sections: &[String],
    minor: u64,
    metadata: &BTreeMap<String, Value>,
) -> Result<(), ParseError> {
    for name in sections {
        if known_section(name) {
            continue;
        }
        let arity = metadata
            .get(meta::SECTION_ARITY)
            .and_then(|value| value.as_object())
            .and_then(|obj| obj.get(name))
            .and_then(|value| value.as_u64())
            .filter(|n| *n >= 1);
        if minor > u64::from(crate::CON_SPEC_MINOR) && arity.is_some() {
            continue;
        }
        return Err(ParseError::UnknownSection(name.clone()));
    }
    Ok(())
}

fn reject_unknown_sections(header: &FrameHeader) -> Result<(), ParseError> {
    let minor = header
        .metadata
        .get(meta::CON_SPEC_MINOR)
        .and_then(json_u64)
        .unwrap_or(0);
    reject_unknown_section_names(&header.sections, minor, &header.metadata)
}

/// Parses a complete frame from a `.con` file, including its header and atomic data.
///
/// This function first parses the complete frame header and then uses the information within it
/// (specifically the number of atom types and atoms per type) to parse the subsequent
/// atom coordinate blocks.
///
/// # Arguments
///
/// * `lines` - A mutable reference to an iterator that yields string slices for the frame.
///
/// # Errors
///
/// * `ParseError::IncompleteFrame` if the iterator ends before all expected
///   atomic data has been read.
/// * Propagates any errors from the underlying calls to `parse_frame_header` and
///   `parse_line_of_n`.
///
/// # Example
///
/// ```
/// use readcon_core::parser::parse_single_frame;
///
/// let frame_text = r#"
///Generated by test
///{"con_spec_version":2}
///10.0 10.0 10.0
///90.0 90.0 90.0
///POSTBOX LINE 1
///POSTBOX LINE 2
///2
///1 1
///12.011 1.008
///C
///Coordinates of Component 1
///1.0 1.0 1.0 0 1
///H
///Coordinates of Component 2
///2.0 2.0 2.0 0 2
/// "#;
///
/// let mut lines = frame_text.trim().lines();
/// let con_frame = parse_single_frame(&mut lines).unwrap();
///
/// assert_eq!(con_frame.header.natm_types, 2);
/// assert_eq!(con_frame.atom_data.len(), 2);
/// assert_eq!(&*con_frame.atom_data[0].symbol, "C");
/// assert_eq!(con_frame.atom_data[1].atom_id, 2);
/// ```
pub fn parse_single_frame<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
) -> Result<ConFrame, ParseError> {
    // The caller owns the iterator and may not be able to clone it. Section
    // detection, which needs a lookahead clone, runs later on a `LineStream`
    // that can probe (`MemchrLines`, or a `Peekable` whose source is `Clone`).
    parse_single_frame_ls(&mut IterStream(lines))
}

/// Iterator adapter so [`parse_single_frame`] can satisfy [`LineStream`]
/// without requiring [`Clone`].
struct IterStream<I>(I);

impl<'a, I> Iterator for IterStream<I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.0.next()
    }
}

impl<'a, I> LineStream<'a> for IterStream<I>
where
    I: Iterator<Item = &'a str>,
{
    fn next_line(&mut self) -> Option<&'a str> {
        Iterator::next(self)
    }
    fn peek_line(&mut self) -> Option<&'a str> {
        None
    }
}

/// Frame parse over a [`LineStream`].
///
/// Coordinate rows go through [`LineStream::next_coord_row`] so the memchr
/// cursor can fuse the newline scan with numeric parse. Positions, ids, and
/// flags land in flat buffers; [`AtomDatum`] values are built from those
/// buffers afterwards (one `Vec` allocation, no per-atom heap traffic).
pub(crate) fn parse_single_frame_ls<'a, L>(lines: &mut L) -> Result<ConFrame, ParseError>
where
    L: LineStream<'a> + Iterator<Item = &'a str>,
{
    let header = parse_frame_header(lines)?;
    let total_atoms: usize = header.natms_per_type.iter().sum();
    let alloc_atoms = match lines.bytes_remaining() {
        Some(left) if total_atoms > left => left,
        _ => total_atoms,
    };
    use crate::storage_dtype::{ElementKind, FloatArray2, StorageDtypes};
    let dt = StorageDtypes::from_metadata(&header.metadata).unwrap_or_default();
    let f64_positions = dt.positions == ElementKind::Float64;
    let mut pos_flat = Vec::with_capacity(alloc_atoms.saturating_mul(3));
    let mut positions_other = if f64_positions {
        None
    } else {
        Some(FloatArray2::zeros(dt.positions, alloc_atoms, 3))
    };
    let mut ids = Vec::with_capacity(alloc_atoms);
    let mut atom_data: Vec<AtomDatum> = Vec::with_capacity(alloc_atoms);

    let mut global_atom_idx: u64 = 0;
    for (type_idx, num_atoms) in header.natms_per_type.iter().enumerate() {
        let symbol_line = lines.next().ok_or(ParseError::IncompleteFrame)?;
        let symbol: Arc<str> = Arc::from(symbol_line.trim());
        let coord_label = lines.next().ok_or(ParseError::IncompleteFrame)?;
        // The recorded reading checks a coordinate label in every mode.
        // The symbol check is CON-validate.4 and runs only in that mode.
        let expected_label = format!("Coordinates of Component {}", type_idx + 1);
        if !labels_equal(coord_label, &expected_label) {
            return Err(ParseError::ValidationError(format!(
                "expected coordinate label {expected_label:?}, found {coord_label:?}"
            )));
        }
        if header.strict_validation {
            validate_coordinate_component(type_idx, symbol.as_ref(), coord_label)?;
        }
        global_atom_idx = ingest_coord_rows(
            lines,
            *num_atoms,
            header.spec_version,
            global_atom_idx,
            &symbol,
            CoordSink {
                pos_flat: &mut pos_flat,
                ids: &mut ids,
                positions_other: &mut positions_other,
                atom_data: &mut atom_data,
            },
        )?;
    }
    let positions = if f64_positions {
        FloatArray2::from_f64_row_major(total_atoms, 3, pos_flat)
    } else {
        positions_other.expect("non-f64 positions allocated")
    };
    Ok(crate::types::con_frame_coords_only_with_ids(
        header, atom_data, positions, ids,
    ))
}

struct CoordSink<'a> {
    pos_flat: &'a mut Vec<f64>,
    ids: &'a mut Vec<u64>,
    positions_other: &'a mut Option<crate::storage_dtype::FloatArray2>,
    atom_data: &'a mut Vec<AtomDatum>,
}

/// Parse `n` coordinate rows into flat SoA buffers.
#[inline(always)]
fn ingest_coord_rows<'a, L>(
    lines: &mut L,
    n: usize,
    spec_version: u32,
    mut global_atom_idx: u64,
    symbol: &Arc<str>,
    sink: CoordSink<'_>,
) -> Result<u64, ParseError>
where
    L: LineStream<'a>,
{
    let CoordSink {
        pos_flat,
        ids,
        positions_other,
        atom_data,
    } = sink;
    for _ in 0..n {
        let (xyz, fixed, atom_id) = lines.next_coord_row(spec_version, global_atom_idx)?;
        if let Some(pos) = positions_other.as_mut() {
            pos.set_f64_row(ids.len(), xyz);
        }
        pos_flat.push(xyz[0]);
        pos_flat.push(xyz[1]);
        pos_flat.push(xyz[2]);
        ids.push(atom_id);
        atom_data.push(AtomDatum::coords(
            Arc::clone(symbol),
            xyz[0],
            xyz[1],
            xyz[2],
            fixed,
            atom_id,
        ));
        global_atom_idx += 1;
    }
    Ok(global_atom_idx)
}

fn validate_header_geometry(boxl: &[f64], angles: &[f64]) -> Result<(), ParseError> {
    if boxl
        .iter()
        .any(|length| !length.is_finite() || *length <= 0.0)
        || angles
            .iter()
            .any(|angle| !angle.is_finite() || *angle <= 0.0 || *angle >= 180.0)
    {
        return Err(ParseError::ValidationError(
            "cell geometry must have positive lengths and angles between 0 and 180 degrees"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_masses(masses_per_type: &[f64]) -> Result<(), ParseError> {
    if masses_per_type
        .iter()
        .any(|mass| !mass.is_finite() || *mass <= 0.0)
    {
        return Err(ParseError::ValidationError(
            "component masses must be positive".to_string(),
        ));
    }
    Ok(())
}

fn validate_coordinate_component(
    type_idx: usize,
    symbol: &str,
    label: &str,
) -> Result<(), ParseError> {
    let expected_label = format!("Coordinates of Component {}", type_idx + 1);
    if !labels_equal(label, &expected_label) {
        return Err(ParseError::ValidationError(format!(
            "expected coordinate label {expected_label:?}, found {label:?}"
        )));
    }
    if symbol != "X" && symbol_to_atomic_number(symbol) == 0 {
        return Err(ParseError::ValidationError(format!(
            "unknown component symbol {symbol}"
        )));
    }
    Ok(())
}

fn validate_section_component(
    section: &str,
    type_idx: usize,
    atom_idx: usize,
    symbol: &str,
    label: &str,
    header: &FrameHeader,
    atom_data: &[AtomDatum],
) -> Result<(), ParseError> {
    let expected_label = format!("{section} of Component {}", type_idx + 1);
    if !labels_equal(label, &expected_label) {
        return Err(ParseError::ValidationError(format!(
            "expected section label {expected_label:?}, found {label:?}"
        )));
    }

    if header.natms_per_type[type_idx] == 0 {
        return Ok(());
    }

    let expected_symbol = atom_data
        .get(atom_idx)
        .map(|atom| atom.symbol.as_ref())
        .ok_or_else(|| {
            ParseError::ValidationError(format!(
                "{section} component {} has no coordinate atom to validate against",
                type_idx + 1
            ))
        })?;
    if symbol != expected_symbol {
        return Err(ParseError::ValidationError(format!(
            "{section} component {} symbol mismatch: expected {expected_symbol}, found {symbol}",
            type_idx + 1
        )));
    }

    Ok(())
}

fn validate_section_atom_identity(
    section: &str,
    atom_idx: usize,
    fixed: [bool; 3],
    atom_id: u64,
    atom_data: &[AtomDatum],
) -> Result<(), ParseError> {
    let atom = atom_data.get(atom_idx).ok_or_else(|| {
        ParseError::ValidationError(format!(
            "{section} row {atom_idx} has no coordinate atom to validate against"
        ))
    })?;

    if atom.fixed != fixed {
        return Err(ParseError::ValidationError(format!(
            "{section} row {atom_idx} fixed mask mismatch for atom_id {}",
            atom.atom_id
        )));
    }
    if atom.atom_id != atom_id {
        return Err(ParseError::ValidationError(format!(
            "{section} row {atom_idx} atom_id mismatch: expected {}, found {atom_id}",
            atom.atom_id
        )));
    }

    Ok(())
}

/// Consume the single blank line that opens a section.
///
/// A blank line followed only by whitespace to EOF does not open a section
/// (CON-frame.5). The lines stay in the stream.
fn frame_minor(header: &FrameHeader) -> u64 {
    header
        .metadata
        .get(meta::CON_SPEC_MINOR)
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn section_row_arity(header: &FrameHeader, name: &str) -> Option<usize> {
    let obj = header.metadata.get(meta::SECTION_ARITY)?.as_object()?;
    let n = obj.get(name)?.as_u64()?;
    if n >= 1 { Some(n as usize) } else { None }
}

/// Skip one unknown section of a newer minor.
///
/// `arity` is the value-field count (CON-section.12). Each data row holds
/// that many values, then the flag, and an optional id, so the row has
/// `arity + 1` or `arity + 2` tokens (CON-section.14). The label is the
/// section name under CON-file.5, then `of Component N` (CON-section.15).
fn skip_unknown_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    name: &str,
    arity: usize,
) -> Result<(), ParseError> {
    if !open_section(lines)? {
        return Err(ParseError::IncompleteSection(name.to_string()));
    }
    let with_flag = arity.saturating_add(1);
    let with_id = arity.saturating_add(2);
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        if lines.next_line().is_none() {
            return Err(ParseError::IncompleteSection(name.to_string()));
        }
        let label = lines
            .next_line()
            .ok_or_else(|| ParseError::IncompleteSection(name.to_string()))?;
        let expected = format!("{name} of Component {}", type_idx + 1);
        if !labels_equal(label, &expected) {
            return Err(ParseError::IncompleteSection(name.to_string()));
        }
        for _ in 0..num_atoms {
            let row = lines
                .next_line()
                .ok_or_else(|| ParseError::IncompleteSection(name.to_string()))?;
            let found = token_count(row);
            if found != with_flag && found != with_id {
                let expected = if found < with_flag {
                    with_flag
                } else {
                    with_id
                };
                return Err(ParseError::InvalidVectorLength { expected, found });
            }
        }
    }
    Ok(())
}

fn open_section<'a>(lines: &mut impl LineStream<'a>) -> Result<bool, ParseError> {
    match lines.peek_line() {
        Some(line) if line.trim().is_empty() => {
            if lines.remainder_is_whitespace() {
                return Ok(false);
            }
            lines.next_line();
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Attempts to parse an optional velocity section following coordinate blocks.
///
/// In `.convel` files, after all coordinate blocks there is a blank separator line
/// followed by per-component velocity blocks with the same structure as coordinate
/// blocks (symbol line, "Velocities of Component N" line, then atom lines with
/// `vx vy vz fixed atomID`).
///
/// This function peeks at the next line. If it is blank (or contains only whitespace),
/// it consumes the blank line and parses velocity data into the existing `atom_data`.
/// If the next line is not blank (or is absent), no velocities are parsed.
///
/// Returns `Ok(true)` if velocities were found and parsed, `Ok(false)` otherwise.
pub fn parse_velocity_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    // A blank line followed only by whitespace to EOF is not a velocity
    // block (CON-frame.5, D2). Leave those lines for the iterator.
    match lines.peek_line() {
        Some(line) if line.trim().is_empty() => {
            if lines.remainder_is_whitespace() {
                return Ok(false);
            }
            // A declared section starts at its blank line even when the
            // label is wrong (CON-section.2). A legacy blank counts only
            // when it begins a complete velocity block (CON-frame.7).
            if !header.sections_declared && !lines.blank_line_opens_section() {
                return Ok(false);
            }
            lines.next_line();
        }
        Some(_) if header.sections_declared => {
            return Err(ParseError::IncompleteSection(SECTION_VELOCITIES.into()));
        }
        _ => return Ok(false),
    }

    parse_velocity_rows(lines, header, atom_data)?;
    Ok(true)
}

/// A velocity block the legacy opener has already accepted.
///
/// The opener is three checks on the peeked blank: `peek_line` is empty,
/// `blank_line_opens_section` sees a symbol and a velocity label, and
/// `legacy_velocity_rows_complete` sees five fields on every row. Absence
/// is not a result of this call. A row that does not parse is an error.
fn parse_opened_velocity_block<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<(), ParseError> {
    lines.next_line();
    parse_velocity_rows(lines, header, atom_data)
}

fn parse_velocity_rows<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<(), ParseError> {
    let validate = header.strict_validation;
    let mut atom_idx: usize = 0;
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        // Symbol line
        let symbol = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_VELOCITIES.into()))?
            .trim();
        if validate {
            let expected_symbol = atom_data
                .get(atom_idx)
                .map(|atom| atom.symbol.as_ref())
                .unwrap_or("");
            if symbol != expected_symbol {
                return Err(ParseError::ValidationError(format!(
                    "Velocities component {} symbol mismatch: expected {expected_symbol}, found {symbol}",
                    type_idx + 1
                )));
            }
        }

        // "Velocities of Component N" line
        let comp_line = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_VELOCITIES.into()))?;
        let expected = format!("Velocities of Component {}", type_idx + 1);
        if !labels_equal(comp_line, &expected) {
            if validate {
                return Err(ParseError::ValidationError(format!(
                    "expected section label {expected:?}, found {comp_line:?}"
                )));
            }
            return Err(ParseError::IncompleteSection(SECTION_VELOCITIES.into()));
        }
        if validate {
            validate_section_component(
                "Velocities",
                type_idx,
                atom_idx,
                symbol,
                comp_line,
                header,
                atom_data,
            )?;
        }

        for _ in 0..num_atoms {
            let vel_line = lines
                .next_line()
                .ok_or(ParseError::IncompleteSection(SECTION_VELOCITIES.into()))?;
            let (vals, fixed, atom_id) = parse_vector_section_row(vel_line, header.spec_version)?;
            if validate {
                validate_section_atom_identity("velocities", atom_idx, fixed, atom_id, atom_data)?;
            }
            if atom_idx < atom_data.len() {
                atom_data[atom_idx].set_velocity(Some(vals));
            }
            atom_idx += 1;
        }
    }

    Ok(())
}

/// Attempts to parse a force section following coordinate (and optional velocity) blocks.
///
/// Force sections mirror velocity sections: a blank separator line followed by per-component
/// force blocks (symbol line, "Forces of Component N" line, then atom lines with
/// `fx fy fz fixed_flag atom_id`).
///
/// Returns `Ok(true)` if forces were found and parsed, `Ok(false)` otherwise.
pub fn parse_force_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    let validate = header.strict_validation;
    if !open_section(lines)? {
        return Ok(false);
    }

    let mut atom_idx: usize = 0;
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        let symbol = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_FORCES.into()))?
            .trim();

        let comp_line = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_FORCES.into()))?;
        if !label_has_prefix(comp_line, "Forces of Component") {
            return Err(ParseError::IncompleteSection(SECTION_FORCES.into()));
        }
        if validate {
            validate_section_component(
                "Forces", type_idx, atom_idx, symbol, comp_line, header, atom_data,
            )?;
        }

        for _ in 0..num_atoms {
            let force_line = lines
                .next_line()
                .ok_or(ParseError::IncompleteSection(SECTION_FORCES.into()))?;
            let (vals, fixed, atom_id) = parse_vector_section_row(force_line, header.spec_version)?;
            if validate {
                validate_section_atom_identity("forces", atom_idx, fixed, atom_id, atom_data)?;
            }
            if atom_idx < atom_data.len() {
                atom_data[atom_idx].set_force(Some(vals));
            }
            atom_idx += 1;
        }
    }

    Ok(true)
}

/// Attempts to parse an energies section following coordinate (and optional
/// velocity / force) blocks.
///
/// Energy sections mirror force sections but with one scalar per atom:
/// blank separator, then per-component blocks of (symbol, "Energies of
/// Component N", and atom lines `e fixed_flag atom_id`). The two
/// trailing identity columns are optional and used only for strict
/// validation; in non-strict mode any whitespace after the energy is
/// ignored.
///
/// Returns `Ok(true)` if energies were found and parsed, `Ok(false)`
/// otherwise.
pub fn parse_energy_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    let validate = header.strict_validation;
    if !open_section(lines)? {
        return Ok(false);
    }

    let mut atom_idx: usize = 0;
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        let symbol = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_ENERGIES.into()))?
            .trim();

        let comp_line = lines
            .next_line()
            .ok_or(ParseError::IncompleteSection(SECTION_ENERGIES.into()))?;
        if !label_has_prefix(comp_line, "Energies of Component") {
            return Err(ParseError::IncompleteSection(SECTION_ENERGIES.into()));
        }
        if validate {
            validate_section_component(
                "Energies", type_idx, atom_idx, symbol, comp_line, header, atom_data,
            )?;
        }

        for _ in 0..num_atoms {
            let energy_line = lines
                .next_line()
                .ok_or(ParseError::IncompleteSection(SECTION_ENERGIES.into()))?;
            let (value, fixed, atom_id) =
                parse_scalar_section_row(energy_line, header.spec_version)?;
            if validate {
                validate_section_atom_identity("energies", atom_idx, fixed, atom_id, atom_data)?;
            }
            if atom_idx < atom_data.len() {
                atom_data[atom_idx].set_energy(Some(value));
            }
            atom_idx += 1;
        }
    }

    Ok(true)
}

/// Parses declared sections from a frame's header metadata.
///
/// If `header.sections` is non-empty (v2 file with `"sections"` key in JSON),
/// parses each declared section in order. Otherwise falls back to legacy
/// blank-separator velocity detection.
pub fn parse_declared_sections<'a>(
    lines: &mut impl LineStream<'a>,
    header: &mut FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<usize, ParseError> {
    let mut applied = 0usize;
    // A zero-atom frame has no coordinate block and no section block.
    // The legacy velocity fallback must not run (CON-count.1).
    if header.natm_types == 0 {
        if !header.sections.is_empty() {
            return Err(ParseError::InvalidCount(
                "a zero-atom frame cannot declare sections".into(),
            ));
        }
        return Ok(0);
    }
    if !header.sections_declared && header.sections.is_empty() {
        // Legacy: a blank line opens a velocity block only when every
        // label is `velocities of Component N` and every row has five
        // fields (CON-frame.5, CON-frame.7). Missing units.time is reported
        // on that block, before the row values (CON-units.11).
        let opens = lines
            .peek_line()
            .is_some_and(|line| line.trim().is_empty())
            && lines.blank_line_opens_section()
            && lines.legacy_velocity_rows_complete(&header.natms_per_type);
        if opens {
            if minor_velocities_need_time(header) {
                return Err(ParseError::InvalidUnits(
                    "legacy velocity block without units.time".into(),
                ));
            }
            parse_opened_velocity_block(lines, header, atom_data)?;
            header.sections.push(SECTION_VELOCITIES.into());
            applied = 1;
        }
    } else {
        let sections = std::mem::take(&mut header.sections);
        for section in &sections {
            match section.as_str() {
                SECTION_VELOCITIES => {
                    let found = parse_velocity_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_VELOCITIES.into()));
                    }
                    applied += 1;
                }
                SECTION_FORCES => {
                    let found = parse_force_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_FORCES.into()));
                    }
                    applied += 1;
                }
                SECTION_ENERGIES => {
                    let found = parse_energy_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_ENERGIES.into()));
                    }
                    applied += 1;
                }
                SECTION_CHARGES => {
                    let found = parse_charge_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_CHARGES.into()));
                    }
                    applied += 1;
                }
                SECTION_SPINS => {
                    let found = parse_spin_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_SPINS.into()));
                    }
                    applied += 1;
                }
                SECTION_MAGMOMS => {
                    let found = parse_magmom_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_MAGMOMS.into()));
                    }
                    applied += 1;
                }
                SECTION_DISPLACEMENTS => {
                    let found = parse_displacement_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_DISPLACEMENTS.into()));
                    }
                    applied += 1;
                }
                SECTION_SPREADS => {
                    let found = parse_spread_section(lines, header, atom_data)?;
                    if !found {
                        return Err(ParseError::IncompleteSection(SECTION_SPREADS.into()));
                    }
                    applied += 1;
                }
                other => {
                    if frame_minor(header) > u64::from(crate::CON_SPEC_MINOR) {
                        if let Some(arity) = section_row_arity(header, other) {
                            skip_unknown_section(lines, header, other, arity)?;
                            continue;
                        }
                    }
                    return Err(ParseError::UnknownSection(other.to_string()));
                }
            }
        }
        header.sections = sections;
    }
    reject_velocities_without_time(header)?;
    Ok(applied)
}

/// CON-units.11. Versions 1, 2, and 3.0 do not require `units.time`
/// (CON-units.8, CON-units.9). From minor 1 the key is required when a
/// velocities section is present.
fn minor_velocities_need_time(header: &FrameHeader) -> bool {
    if header.spec_version < 3 || frame_minor(header) < 1 {
        return false;
    }
    !header
        .metadata
        .get(meta::UNITS)
        .and_then(|units| units.get("time"))
        .and_then(|value| value.as_str())
        .is_some_and(|unit| !unit.trim().is_empty())
}

fn reject_velocities_without_time(header: &FrameHeader) -> Result<(), ParseError> {
    if header.spec_version < 3 || frame_minor(header) < 1 {
        return Ok(());
    }
    if !header
        .sections
        .iter()
        .any(|name| name == SECTION_VELOCITIES)
    {
        return Ok(());
    }
    let time_ok = header
        .metadata
        .get(meta::UNITS)
        .and_then(|units| units.get("time"))
        .and_then(|value| value.as_str())
        .is_some_and(|unit| !unit.trim().is_empty());
    if time_ok {
        Ok(())
    } else {
        Err(ParseError::InvalidUnits(
            "minor >= 1 requires units.time when a velocities section is present".into(),
        ))
    }
}

/// Scalar per-atom section: blank separator + per-type blocks
/// (`symbol`, `"X of Component N"`, lines `value fixed atom_id`).
fn parse_scalar_atom_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
    section_name: &str,
    component_label: &str,
    mut set_value: impl FnMut(&mut AtomDatum, f64),
) -> Result<bool, ParseError> {
    let validate = header.strict_validation;
    if !open_section(lines)? {
        return Ok(false);
    }

    let mut atom_idx: usize = 0;
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        let symbol = lines
            .next_line()
            .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?
            .trim();

        let comp_line = lines
            .next_line()
            .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?;
        if !label_has_prefix(comp_line, component_label) {
            return Err(ParseError::IncompleteSection(section_name.into()));
        }
        if validate {
            validate_section_component(
                component_label.trim_end_matches(" of Component").trim(),
                type_idx,
                atom_idx,
                symbol,
                comp_line,
                header,
                atom_data,
            )?;
        }

        for _ in 0..num_atoms {
            let data_line = lines
                .next_line()
                .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?;
            let (value, fixed, atom_id) = parse_scalar_section_row(data_line, header.spec_version)?;
            if validate {
                validate_section_atom_identity(section_name, atom_idx, fixed, atom_id, atom_data)?;
            }
            if atom_idx < atom_data.len() {
                set_value(&mut atom_data[atom_idx], value);
            }
            atom_idx += 1;
        }
    }
    Ok(true)
}

pub fn parse_charge_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    parse_scalar_atom_section(
        lines,
        header,
        atom_data,
        SECTION_CHARGES,
        "Charges of Component",
        |a, v| a.set_charge(Some(v)),
    )
}

pub fn parse_spin_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    parse_scalar_atom_section(
        lines,
        header,
        atom_data,
        SECTION_SPINS,
        "Spins of Component",
        |a, v| a.set_spin(Some(v)),
    )
}

fn parse_vector_named_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
    names: (&str, &str, &str),
    mut set_value: impl FnMut(&mut AtomDatum, [f64; 3]),
    mut check_row: impl FnMut(usize, [f64; 3]) -> Result<(), ParseError>,
) -> Result<bool, ParseError> {
    let (section_name, component_label, label_kind) = names;
    let validate = header.strict_validation;
    if !open_section(lines)? {
        return Ok(false);
    }
    let mut atom_idx: usize = 0;
    for (type_idx, &num_atoms) in header.natms_per_type.iter().enumerate() {
        let symbol = lines
            .next_line()
            .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?
            .trim();
        let comp_line = lines
            .next_line()
            .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?;
        if !label_has_prefix(comp_line, component_label) {
            return Err(ParseError::IncompleteSection(section_name.into()));
        }
        if validate {
            validate_section_component(
                label_kind, type_idx, atom_idx, symbol, comp_line, header, atom_data,
            )?;
        }
        for _ in 0..num_atoms {
            let row = lines
                .next_line()
                .ok_or_else(|| ParseError::IncompleteSection(section_name.into()))?;
            let (vals, fixed, atom_id) = parse_vector_section_row(row, header.spec_version)?;
            check_row(atom_idx, vals)?;
            if validate {
                validate_section_atom_identity(section_name, atom_idx, fixed, atom_id, atom_data)?;
            }
            if atom_idx < atom_data.len() {
                set_value(&mut atom_data[atom_idx], vals);
            }
            atom_idx += 1;
        }
    }
    Ok(true)
}

/// Magmoms: 3-vector per atom, same layout as velocities/forces.
pub fn parse_magmom_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    parse_vector_named_section(
        lines,
        header,
        atom_data,
        (SECTION_MAGMOMS, "Magmoms of Component", "Magmoms"),
        |a, v| a.set_magmom(Some(v)),
        |_, _| Ok(()),
    )
}

/// Displacements: 3-vector per atom (Angstrom), same layout as velocities/forces.
pub fn parse_displacement_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    parse_vector_named_section(
        lines,
        header,
        atom_data,
        (
            SECTION_DISPLACEMENTS,
            "Displacements of Component",
            "Displacements",
        ),
        |a, v| a.set_displacement(Some(v)),
        |_, _| Ok(()),
    )
}

/// Spreads: per-atom root-mean-square spread `[sx, sy, sz]` (Angstrom, each
/// non-negative), same layout as velocities/forces.
pub fn parse_spread_section<'a>(
    lines: &mut impl LineStream<'a>,
    header: &FrameHeader,
    atom_data: &mut [AtomDatum],
) -> Result<bool, ParseError> {
    parse_vector_named_section(
        lines,
        header,
        atom_data,
        (SECTION_SPREADS, "Spreads of Component", "Spreads"),
        |a, v| a.set_spread(Some(v)),
        |atom_idx, vals| {
            // Coordinate parsing already rejects a non-finite field.
            if vals.iter().any(|v| *v < 0.0) {
                return Err(crate::types::invalid_spread(atom_idx));
            }
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterators::ConFrameIterator;

    fn frame_with_count(count: &str, validate: bool) -> String {
        let validate = if validate { r#","validate":true"# } else { "" };
        format!(
            "user\n{{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{{\"length\":\"angstrom\",\"energy\":\"eV\"}},\"sections\":[]{validate}}}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n1\n{count}\n1.0\nH\nCoordinates of Component 1\n0.0 0.0 0.0 0 1\n"
        )
    }

    #[test]
    fn impossible_angles_are_rejected_with_and_without_validate() {
        for angles in ["10.0 10.0 170.0", "170.0 170.0 170.0"] {
            for validate in [false, true] {
                let text = frame_with_count("1", validate).replacen("90.0 90.0 90.0", angles, 1);
                let err = ConFrameIterator::new(&text).next().unwrap().unwrap_err();
                assert!(
                    matches!(err, ParseError::ValidationError(_)),
                    "{angles} validate={validate}: {err}"
                );
                assert!(err.to_string().contains("real cell"), "{err}");
            }
        }
    }

    #[test]
    fn an_empty_coordinate_line_is_the_wrong_number_of_fields() {
        let text = frame_with_count("2", false).replacen(
            "0.0 0.0 0.0 0 1\n",
            "\n0.0 0.0 0.0 0 1\n",
            1,
        );
        let err = ConFrameIterator::new(&text).next().unwrap().unwrap_err();
        assert!(
            matches!(
                err,
                ParseError::InvalidVectorLength {
                    expected: 5,
                    found: 0
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn a_short_legacy_velocity_row_is_not_a_velocity_block() {
        let text = "\
user
{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\",\"mass\":\"amu\"}}
10.0 10.0 10.0
90.0 90.0 90.0
0 0
0 0 0
1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1

H
Velocities of Component 1
0.1 0.0 0.0
";
        let err = crate::iterators::frames_from_text(text, Some(1)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let parallel = crate::iterators::frames_from_text(text, Some(2)).unwrap_err();
        assert!(matches!(parallel, ParseError::InvalidNumberFormat(_)));
    }

    fn class_at(text: &str, threads: usize) -> String {
        crate::iterators::frames_from_text(text, Some(threads))
            .unwrap_err()
            .class_name()
            .to_string()
    }

    #[test]
    fn agreed_error_class_for_each_corruption() {
        let minor = frame_with_count("1", false);
        let minor_float = minor.replacen("\"con_spec_minor\":1", "\"con_spec_minor\":0.0", 1);
        assert_eq!(class_at(&minor_float, 1), "InvalidMetadataJson");
        let huge = minor.replacen(
            "\"con_spec_minor\":1",
            "\"con_spec_minor\":18446744073709551616",
            1,
        );
        assert!(
            crate::iterators::frames_from_text(&huge, Some(1)).is_ok(),
            "2^64 is a legal minor"
        );
        let timed = minor.replacen("\"energy\":\"eV\"", "\"energy\":\"eV\",\"time\":\"fs\"", 1);
        let bad_time = timed.replacen("\"sections\":[]", "\"time\":\"fs\",\"sections\":[]", 1);
        assert_eq!(class_at(&bad_time, 1), "InvalidMetadataJson");
        let feed = minor.replacen("10.0 10.0 10.0", "10.0\u{c} 10.0 10.0", 1);
        assert_eq!(class_at(&feed, 1), "InvalidNumberFormat");
        let partial = minor.replacen("0.0 0.0 0.0 0 1", "NaN.5 0.0 0.0 0 1", 1);
        assert_eq!(class_at(&partial, 1), "InvalidNumberFormat");

        let legacy = "\
user
{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\",\"mass\":\"amu\"}}
10.0 10.0 10.0
90.0 90.0 90.0
0 0
0 0 0
1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1

H
Velocities of Component 1
0.1 0.0 0.0 0 e1
";
        assert_eq!(class_at(legacy, 1), "InvalidUnits");
        assert_eq!(class_at(legacy, 2), "InvalidUnits");
        let forces = legacy.replacen("Velocities of Component 1", "Forces of Component 1", 1);
        assert_eq!(class_at(&forces, 1), "InvalidNumberFormat");
        assert_eq!(class_at(&forces, 2), "InvalidNumberFormat");

        let declared = "\
user
{\"con_spec_version\":3,\"con_spec_minor\":1,\"units\":{\"length\":\"angstrom\",\"energy\":\"eV\",\"mass\":\"amu\",\"time\":\"fs\"},\"sections\":[\"velocities\"],\"validate\":true}
10.0 10.0 10.0
90.0 90.0 90.0
0 0
0 0 0
1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1
H
Velocity of Component 1
0.1 0.0 0.0 0 1
";
        assert_eq!(class_at(declared, 1), "IncompleteSection");
        assert_eq!(class_at(declared, 2), "IncompleteSection");
        let labelled = declared.replacen(
            "0.0 0.0 0.0 0 1\nH\n",
            "0.0 0.0 0.0 0 1\n\nH\n",
            1,
        );
        assert_eq!(class_at(&labelled, 1), "ValidationError");
        assert_eq!(class_at(&labelled, 2), "ValidationError");
    }

    #[test]
    fn a_declared_type_count_past_the_line_is_invalid_count() {
        let text = frame_with_count("1", false).replacen("\n1\n1\n1.0\n", "\n184467441616\n2\n195.084\n", 1);
        let err = ConFrameIterator::new(&text).next().unwrap().unwrap_err();
        assert!(
            matches!(err, ParseError::InvalidVectorLength { .. }),
            "{err}"
        );
    }

    #[test]
    fn a_declared_count_past_the_file_is_invalid_count() {
        let text = frame_with_count("1000000000000", false);
        let err = ConFrameIterator::new(&text).next().unwrap().unwrap_err();
        assert!(matches!(err, ParseError::IncompleteFrame), "{err}");
    }

    #[test]
    fn test_parse_line_of_n_success() {
        let line = "1.0 2.5 -3.0";
        let values = parse_line_of_n::<f64>(line, 3).unwrap();
        assert_eq!(values, vec![1.0, 2.5, -3.0]);
    }

    #[test]
    fn test_parse_line_of_n_too_short() {
        let line = "1.0 2.5";
        let result = parse_line_of_n::<f64>(line, 3);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidVectorLength {
                expected: 3,
                found: 2
            }
        ));
    }

    #[test]
    fn test_parse_line_of_n_too_long() {
        let line = "1.0 2.5 -3.0 4.0";
        let result = parse_line_of_n::<f64>(line, 3);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidVectorLength {
                expected: 3,
                found: 4
            }
        ));
    }

    #[test]
    fn test_parse_line_of_n_invalid_float() {
        let line = "1.0 abc -3.0";
        let result = parse_line_of_n::<f64>(line, 3);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidNumberFormat(_)
        ));
    }

    #[test]
    fn test_parse_frame_header_success() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let header = parse_frame_header(&mut line_it).expect("header");
        assert_eq!(header.prebox_header.user, "PREBOX1");
        assert_eq!(header.spec_version, 2);
        assert_eq!(header.boxl, [10.0, 20.0, 30.0]);
        assert_eq!(header.angles, [90.0, 90.0, 90.0]);
        assert_eq!(header.postbox_header, ["POSTBOX1", "POSTBOX2"]);
        assert_eq!(header.natm_types, 2);
        assert_eq!(header.natms_per_type, vec![1, 1]);
        assert_eq!(header.masses_per_type, vec![12.011, 1.008]);
    }

    #[test]
    fn test_parse_frame_header_missing_line() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            // Missing masses_per_type
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_frame_header(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::IncompleteHeader));
    }

    #[test]
    fn test_parse_frame_header_missing_spec_version() {
        let lines = [
            "PREBOX1",
            "{}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_frame_header(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::MissingSpecVersion
        ));
    }

    #[test]
    fn test_parse_frame_header_legacy_no_json() {
        // A non-JSON line 2 is treated as a legacy (v1) file.
        let lines = [
            "PREBOX1",
            "0.0000 TIME",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let header = parse_frame_header(&mut line_it).unwrap();
        assert_eq!(header.spec_version, 1);
        assert!(header.metadata.is_empty());
    }

    #[test]
    fn test_parse_frame_header_malformed_json() {
        // Line 2 starts with '{' but is not valid JSON -- this IS an error.
        let lines = [
            "PREBOX1",
            "{broken json",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_frame_header(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidMetadataJson(_)
        ));
    }

    #[test]
    fn test_parse_frame_header_unsupported_version() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":999}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_frame_header(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnsupportedVersion(999)
        ));
    }

    #[test]
    fn test_v3_missing_units_rejected() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":3}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "1.0",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("units"), "expected units error, got {msg}");
        assert!(msg.contains("units"), "expected units error, got {msg}");
    }

    #[test]
    fn test_v3_invalid_units_rejected() {
        let lines = [
            "PREBOX1",
            r#"{"con_spec_version":3,"units":{"length":"eV","energy":"angstrom"}}"#,
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "1.0",
        ];
        let mut line_it = lines.iter().copied();
        assert!(parse_frame_header(&mut line_it).is_err());
    }

    #[test]
    fn test_v3_valid_units_exposes_length_energy() {
        let lines = [
            "PREBOX1",
            r#"{"con_spec_version":3,"units":{"length":"angstrom","energy":"eV","mass":"amu","time":"fs"}}"#,
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "1.0",
        ];
        let mut line_it = lines.iter().copied();
        let header = parse_frame_header(&mut line_it).unwrap();
        assert_eq!(header.spec_version, 3);
        assert_eq!(header.length_unit(), Some("angstrom"));
        assert_eq!(header.energy_unit(), Some("eV"));
        let f = header.conversion_factor_to("length", "nm").unwrap();
        assert!((f - 0.1).abs() < 1e-12);
    }

    #[test]
    fn test_parse_frame_header_extra_metadata_preserved() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"generator\":\"test\"}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let header = parse_frame_header(&mut line_it).unwrap();
        assert_eq!(header.spec_version, 2);
        assert_eq!(
            header.metadata.get("generator"),
            Some(&serde_json::Value::String("test".to_string()))
        );
    }

    #[test]
    fn test_parse_frame_header_invalid_natms_per_type() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1 1", // 3 values, but natm_types is 2
            "12.011 1.008",
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_frame_header(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidVectorLength {
                expected: 2,
                found: 3
            }
        ));
    }

    #[test]
    fn test_parse_single_frame_success() {
        let lines = vec![
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "3 3",
            "12.011 1.008",
            "1",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 0 1",
            "1.0940 0.0 0.0 0 2",
            "-0.5470 0.9499 0.0 0 3",
            "2",
            "Coordinates of Component 2",
            "5.0 5.0 5.0 0 4",
            "6.0940 5.0 5.0 0 5",
            "5.5470 5.9499 5.0 0 6",
        ];
        let mut line_it = lines.iter().copied();
        let frame = parse_single_frame(&mut line_it).unwrap();

        assert_eq!(frame.header.natm_types, 2);
        assert_eq!(frame.header.natms_per_type, vec![3, 3]);
        assert_eq!(frame.header.masses_per_type, vec![12.011, 1.008]);
        assert_eq!(frame.atom_data.len(), 6);
        assert_eq!(&*frame.atom_data[0].symbol, "1");
        assert_eq!(frame.atom_data[0].atom_id, 1);
        assert_eq!(&*frame.atom_data[5].symbol, "2");
        assert_eq!(frame.atom_data[5].atom_id, 6);
    }

    #[test]
    fn test_column_4_value_1_is_x_only_on_spec_2() {
        let lines = [
            "Generated by probe",
            "{\"con_spec_version\":2}",
            "20.0 20.0 20.0",
            "90.0 90.0 90.0",
            "",
            "",
            "1",
            "1",
            "63.5",
            "Cu",
            "Coordinates of component   1",
            "1.0 2.0 3.0 1 1",
        ];
        let mut line_it = lines.iter().copied();
        let frame = parse_single_frame(&mut line_it).unwrap();
        assert_eq!(frame.header.spec_version, 2);
        assert_eq!(frame.atom_data[0].fixed, [true, false, false]);
    }

    #[test]
    fn test_column_4_value_1_is_all_fixed_on_spec_1() {
        let lines = [
            "Generated by probe",
            "0.0000 TIME",
            "20.0 20.0 20.0",
            "90.0 90.0 90.0",
            "",
            "",
            "1",
            "1",
            "63.5",
            "Cu",
            "Coordinates of component   1",
            "1.0 2.0 3.0 1 1",
        ];
        let mut line_it = lines.iter().copied();
        let frame = parse_single_frame(&mut line_it).unwrap();
        assert_eq!(frame.header.spec_version, 1);
        assert_eq!(frame.atom_data[0].fixed, [true, true, true]);
    }

    #[test]
    fn test_parse_single_frame_missing_line() {
        // With a valid header but truncated atom data, we get IncompleteFrame.
        let lines = vec![
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "3 3",
            "12.011 1.008",
            "1",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 0 1",
            "1.0940 0.0 0.0 0 2",
            "-0.5470 0.9499 0.0 0 3",
            // Missing Component 2 entirely
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_single_frame(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::IncompleteFrame));
    }

    #[test]
    fn test_parse_single_frame_missing_atom_index_defaults_sequential() {
        // Column 5 (atom_index) is optional; when absent, defaults to sequential.
        let lines = vec![
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "3 3",
            "12.011 1.008",
            "1",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 0 1",
            "1.0940 0.0 0.0 0 2",
            "-0.5470 0.9499 0.0 0 3",
            "2",
            "Coordinates of Component 2",
            "5.0 5.0 5.0 0",       // No atom_index: defaults to 3
            "6.0940 5.0 5.0 0 10", // Explicit atom_index: 10
            "5.5470 5.9499 5.0 0", // No atom_index: defaults to 5
        ];
        let mut line_it = lines.iter().copied();
        let frame = parse_single_frame(&mut line_it).unwrap();
        assert_eq!(frame.atom_data.len(), 6);
        // First type: explicit atom_index values
        assert_eq!(frame.atom_data[0].atom_id, 1);
        assert_eq!(frame.atom_data[1].atom_id, 2);
        assert_eq!(frame.atom_data[2].atom_id, 3);
        // Second type: mixed explicit and defaulted
        assert_eq!(frame.atom_data[3].atom_id, 3); // defaulted (global idx 3)
        assert_eq!(frame.atom_data[4].atom_id, 10); // explicit
        assert_eq!(frame.atom_data[5].atom_id, 5); // defaulted (global idx 5)
    }

    #[test]
    fn test_parse_single_frame_too_few_columns_fails() {
        // Only 3 columns (missing fixed_flag too) should still fail.
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
            "C",
            "Coordinates of Component 1",
            "0.0 0.0 0.0", // Only 3 values
        ];
        let mut line_it = lines.iter().copied();
        let result = parse_single_frame(&mut line_it);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidVectorLength {
                expected: 5,
                found: 3
            }
        ));
    }

    #[test]
    fn test_parse_velocity_section_present() {
        let lines = vec![
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "2",
            "1 1",
            "63.546 1.008",
            "Cu",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 1 0",
            "H",
            "Coordinates of Component 2",
            "1.0 2.0 3.0 0 1",
            "",
            "Cu",
            "Velocities of Component 1",
            "0.1 0.2 0.3 1 0",
            "H",
            "Velocities of Component 2",
            "0.4 0.5 0.6 0 1",
        ];
        let mut line_it = lines.iter().copied().peekable();
        // Parse the frame first (consuming 15 lines)
        let mut frame =
            parse_single_frame(&mut line_it).expect("coordinate parsing should succeed");
        assert!(!frame.has_velocities());

        // Now parse the velocity section
        let has_vel = parse_velocity_section(&mut line_it, &frame.header, &mut frame.atom_data)
            .expect("velocity parsing should succeed");
        assert!(has_vel);
        assert_eq!(frame.atom_data[0].velocity(), Some([0.1, 0.2, 0.3]));
        assert_eq!(frame.atom_data[1].velocity(), Some([0.4, 0.5, 0.6]));
    }

    #[test]
    fn test_validate_true_accepts_matching_section_identity() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":["velocities"],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
2
1 1
63.546 1.008
Cu
Coordinates of Component 1
0.0 0.0 0.0 5 0
H
Coordinates of Component 2
1.0 2.0 3.0 0 1

Cu
Velocities of Component 1
0.1 0.2 0.3 5 0
H
Velocities of Component 2
0.4 0.5 0.6 0 1
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let frame = iter.next().unwrap().unwrap();

        assert!(frame.has_velocities());
        assert_eq!(
            frame
                .header
                .metadata
                .get("validate")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_validate_true_rejects_section_atom_id_mismatch() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":["velocities"],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates of Component 1
0.0 0.0 0.0 5 0

Cu
Velocities of Component 1
0.1 0.2 0.3 5 99
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();

        assert!(matches!(err, ParseError::ValidationError(_)));
        assert!(err.to_string().contains("atom_id mismatch"));
    }

    #[test]
    fn test_validate_true_rejects_section_symbol_mismatch() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":["forces"],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates of Component 1
0.0 0.0 0.0 5 0

H
Forces of Component 1
0.1 0.2 0.3 5 0
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();

        assert!(matches!(err, ParseError::ValidationError(_)));
        assert!(err.to_string().contains("symbol mismatch"));
    }

    #[test]
    fn test_validate_absent_allows_legacy_duplicate_identity_mismatch() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":["velocities"]}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates of Component 1
0.0 0.0 0.0 5 0

Cu
Velocities of Component 1
0.1 0.2 0.3 0 99
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let frame = iter.next().unwrap().unwrap();

        assert!(frame.has_velocities());
        assert_eq!(frame.atom_data[0].atom_id, 0);
        assert_eq!(frame.atom_data[0].fixed, [true, false, true]);
    }

    #[test]
    fn test_validate_must_be_boolean_when_present() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"validate\":\"yes\",\"sections\":[]}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
        assert!(err.to_string().contains("validate"));
    }

    #[test]
    fn test_validate_true_requires_sections_key() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"validate\":true}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
        assert!(err.to_string().contains("sections"));
    }

    #[test]
    fn test_sections_must_be_string_array_when_present() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"sections\":[\"velocities\",7]}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
        assert!(err.to_string().contains("sections"));
    }

    #[test]
    fn test_validate_true_rejects_non_integer_coordinate_identity_columns() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":[],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates of Component 1
0.0 0.0 0.0 5.0 0
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();

        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));
        assert!(err.to_string().contains("5.0"));
    }

    #[test]
    fn test_validate_true_rejects_non_exact_coordinate_label() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":[],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates Component 1
0.0 0.0 0.0 5 0
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();

        assert!(matches!(err, ParseError::ValidationError(_)));
        assert!(err.to_string().contains("Coordinates of Component 1"));
    }

    #[test]
    fn test_validate_true_rejects_unknown_component_symbol() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":[],"validate":true}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Qq
Coordinates of Component 1
0.0 0.0 0.0 0 0
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();

        assert!(matches!(err, ParseError::ValidationError(_)));
        assert!(err.to_string().contains("symbol"));
    }

    #[test]
    fn test_declared_section_must_be_present() {
        let text = r#"
PREBOX1
{"con_spec_version":2,"sections":["velocities"]}
10.0 20.0 30.0
90.0 90.0 90.0
POSTBOX1
POSTBOX2
1
1
63.546
Cu
Coordinates of Component 1
0.0 0.0 0.0 0 0
"#;
        let mut iter = ConFrameIterator::new(text.trim());
        let err = iter.next().unwrap().unwrap_err();
        let forces = ParseError::IncompleteSection("forces".into());
        for (err, expect_velocities) in [(&err, true), (&forces, false)] {
            let velocities = matches!(
                err,
                ParseError::IncompleteSection(name) if name == "velocities"
            );
            assert_eq!(velocities, expect_velocities, "{err}");
        }
    }

    #[test]
    fn test_non_finite_cell_geometry_rejected_in_strict_mode() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"sections\":[],\"validate\":true}",
            "10.0 NaN 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::NonFiniteValue(_)));
        assert!(err.to_string().contains("NaN"));
    }

    #[test]
    fn test_validate_true_rejects_non_physical_cell_geometry() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"sections\":[],\"validate\":true}",
            "0.0 20.0 30.0",
            "90.0 180.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::ValidationError(_)));
        assert!(err.to_string().contains("cell"));
    }

    #[test]
    fn test_validate_true_rejects_reserved_metadata_type_mismatch() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"sections\":[],\"validate\":true,\"energy\":\"low\"}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();

        assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
        assert!(err.to_string().contains("energy"));
    }

    #[test]
    fn test_validate_true_rejects_malformed_bonds() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"sections\":[],\"validate\":true,\"bonds\":[[0]]}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let err = parse_frame_header(&mut line_it).unwrap_err();
        assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
        assert!(err.to_string().contains("bonds"));
    }

    #[test]
    fn test_bonds_metadata_round_trip_in_header() {
        use crate::types::{Bond, meta};
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2,\"bonds\":[[0,1],{\"i\":0,\"j\":2,\"order\":1}]}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
        ];
        let mut line_it = lines.iter().copied();
        let header = parse_frame_header(&mut line_it).expect("header");
        let bonds = header.bonds();
        assert_eq!(bonds.len(), 2);
        assert_eq!(bonds[0], Bond::new(0, 1));
        assert_eq!(bonds[1].i, 0);
        assert_eq!(bonds[1].j, 2);
        assert_eq!(bonds[1].order, Some(1));
        assert!(header.metadata.contains_key(meta::BONDS));
    }

    #[test]
    fn test_parse_velocity_section_absent() {
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 20.0 30.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "1",
            "12.011",
            "C",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 0 1",
        ];
        let mut line_it = lines.iter().copied().peekable();
        let mut frame = parse_single_frame(&mut line_it).expect("parse should succeed");
        let has_vel = parse_velocity_section(&mut line_it, &frame.header, &mut frame.atom_data)
            .expect("should succeed with no velocities");
        assert!(!has_vel);
        assert_eq!(frame.atom_data[0].velocity(), None);
    }

    #[test]
    fn test_parse_line_of_range_f64_exact() {
        let vals = parse_line_of_range_f64("1.0 2.0 3.0 0.0 42", 4, 5, &[0.0; 5]).unwrap();
        assert_eq!(vals, vec![1.0, 2.0, 3.0, 0.0, 42.0]);
    }

    #[test]
    fn test_byte_scan_stack_parses_realistic_coord_line() {
        // Typical CON atom line: x y z fixed_mask [atom_id]
        let line = "   0.63939999999999997    0.90449999999999997   -0.00009999999999977 1    0";
        let defaults = [0.0, 0.0, 0.0, 0.0, 99.0];
        let mut buf = [0.0f64; 5];
        let n = parse_line_of_range_f64_stack(line, 4, 5, &defaults, &mut buf).unwrap();
        assert_eq!(n, 5);
        assert!((buf[0] - 0.6394).abs() < 1e-4);
        assert!((buf[1] - 0.9045).abs() < 1e-4);
        assert_eq!(buf[3] as u8, 1);
        assert_eq!(buf[4] as u64, 0);
        // trailing junk must error
        let bad = "1.0 2.0 3.0 1 0 EXTRA";
        assert!(parse_line_of_range_f64_stack(bad, 4, 5, &defaults, &mut buf).is_err());
        // partial non-boundary token must error
        let glued = "1.0 2.0 3.0abc 1 0";
        assert!(parse_line_of_range_f64_stack(glued, 4, 5, &defaults, &mut buf).is_err());
    }

    #[test]
    fn test_parse_line_of_range_f64_stack_matches_vec_api() {
        let defaults = [0.0, 0.0, 0.0, 0.0, 7.0];
        let line = "1.0 2.0 3.0 0.0";
        let mut buf = [0.0f64; 5];
        parse_line_of_range_f64_stack(line, 4, 5, &defaults, &mut buf).unwrap();
        let via_vec = parse_line_of_range_f64(line, 4, 5, &defaults).unwrap();
        assert_eq!(&buf[..5], via_vec.as_slice());
        assert_eq!(buf[4], 7.0);
    }

    #[test]
    fn test_parse_line_of_range_f64_padded() {
        let defaults = [0.0, 0.0, 0.0, 0.0, 99.0];
        let vals = parse_line_of_range_f64("1.0 2.0 3.0 0.0", 4, 5, &defaults).unwrap();
        assert_eq!(vals, vec![1.0, 2.0, 3.0, 0.0, 99.0]);
    }

    #[test]
    fn test_parse_line_of_range_f64_too_few() {
        let result = parse_line_of_range_f64("1.0 2.0 3.0", 4, 5, &[0.0; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_line_of_range_f64_too_many() {
        let result = parse_line_of_range_f64("1.0 2.0 3.0 0.0 5.0 6.0", 4, 5, &[0.0; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_all_four_column_lines() {
        // All atom lines have only 4 columns; atom_index defaults to sequential.
        let lines = [
            "PREBOX1",
            "{\"con_spec_version\":2}",
            "10.0 10.0 10.0",
            "90.0 90.0 90.0",
            "POSTBOX1",
            "POSTBOX2",
            "1",
            "3",
            "12.011",
            "C",
            "Coordinates of Component 1",
            "0.0 0.0 0.0 0",
            "1.0 0.0 0.0 0",
            "2.0 0.0 0.0 1",
        ];
        let mut line_it = lines.iter().copied();
        let frame = parse_single_frame(&mut line_it).unwrap();
        assert_eq!(frame.atom_data[0].atom_id, 0);
        assert_eq!(frame.atom_data[1].atom_id, 1);
        assert_eq!(frame.atom_data[2].atom_id, 2);
        assert!(frame.atom_data[2].is_fixed());
    }

    #[test]
    fn coord_row_and_fixed_flag_reject_the_short_forms() {
        let err = parse_coord_row_at(b"", 0, 2, 0).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteFrame));
        let err = parse_fixed_flag("", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));
        let err = parse_fixed_flag("-1", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));
        let err = parse_fixed_flag("8", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));
        let err = parse_fixed_flag("2", 1).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));
        assert_eq!(parse_fixed_flag("0", 1).unwrap(), [false, false, false]);
        assert_eq!(parse_fixed_flag("1", 1).unwrap(), [true, true, true]);
        assert_eq!(parse_fixed_flag("5", 2).unwrap(), [true, false, true]);
        let err = parse_scalar_section_row("1.0", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }));
        let err = parse_scalar_section_row("1.0 0", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }));
        let err = parse_scalar_section_row("nope 0 1", 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let err = parse_scalar_section_row("inf 0 1", 2).unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)));
        let err = parse_scalar_section_row("1.5 1 7 extra", 2).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InvalidVectorLength { expected: 3, .. }
        ));
        let (value, fixed, id) = parse_scalar_section_row("1.5 1 7", 2).unwrap();
        assert_eq!(value, 1.5);
        assert_eq!(fixed, [true, false, false]);
        assert_eq!(id, 7);
    }

    #[test]
    fn bonds_metadata_rejects_each_bad_shape() {
        let cases = [
            (r#""no""#, "array"),
            (r#"[[1]]"#, "exactly two"),
            (r#"[[-1,0]]"#, "non-negative"),
            (r#"[[4294967296,0]]"#, "exceeds u32"),
            (r#"[{"i":1}]"#, "include \"j\""),
            (r#"[{"i":"x","j":0}]"#, "non-negative"),
            (r#"[{"i":4294967296,"j":0}]"#, "exceeds u32"),
            (r#"[{"i":0,"j":1,"order":"no"}]"#, "order must be an integer"),
            (r#"[true]"#, "[i, j]"),
        ];
        for (raw, needle) in cases {
            let value: serde_json::Value = serde_json::from_str(raw).unwrap();
            let err = super::validate_bonds_metadata(&value).expect_err(raw);
            let text = err.to_string();
            assert!(text.contains(needle), "{raw}: {text}");
        }
        let ok: serde_json::Value =
            serde_json::from_str(r#"[[0,0],{"i":0,"j":0,"order":1}]"#).unwrap();
        super::validate_bonds_metadata(&ok).unwrap();
    }

    struct BareStream;
    impl<'a> super::LineStream<'a> for BareStream {
        fn next_line(&mut self) -> Option<&'a str> {
            None
        }
        fn peek_line(&mut self) -> Option<&'a str> {
            None
        }
    }

    #[test]
    fn a_stream_without_lookahead_uses_the_trait_defaults() {
        let mut bare = BareStream;
        assert!(!bare.remainder_is_whitespace());
        assert!(!bare.blank_line_opens_section());
        assert!(bare.blank_opens_velocity_block());
        assert!(bare.legacy_velocity_rows_complete(&[5, 5]));
        assert!(bare.next_line().is_none());
        assert!(bare.peek_line().is_none());
        assert!(bare.bytes_remaining().is_none());
        assert!(matches!(
            bare.next_coord_row(2, 0),
            Err(ParseError::IncompleteFrame)
        ));
    }

    #[test]
    fn validators_reject_bad_tokens_labels_and_metadata() {
        assert_eq!(super::count_tokens_on_line(b"  a\tb c\n", 0), 3);
        assert_eq!(super::count_tokens_on_line(b" \n", 0), 0);
        let err = super::parse_finite_at("", 0).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }));
        let err = super::parse_finite_at("nan", 0).unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)));
        let err = super::parse_finite_at("1.0xyz", 0).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let err = super::parse_atom_id_token("no").unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let err = super::parse_flag_at(b"2\n", 0, 1).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)));

        let frames = crate::iterators::frames_from_text(
            &std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap(),
            Some(1),
        )
        .unwrap();
        let frame = &frames[0];
        let err = super::validate_section_component(
            "velocities",
            0,
            0,
            "Cu",
            "not a label",
            &frame.header,
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("expected section label"));
        let err = super::validate_section_component(
            "velocities",
            0,
            99,
            "Cu",
            "velocities of Component 1",
            &frame.header,
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("no coordinate atom"));
        let err = super::validate_section_component(
            "velocities",
            0,
            0,
            "Nope",
            "velocities of Component 1",
            &frame.header,
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("symbol mismatch"));
        let mut header = frame.header.clone();
        header.natms_per_type[0] = 0;
        super::validate_section_component(
            "velocities",
            0,
            0,
            "Cu",
            "velocities of Component 1",
            &header,
            &frame.atom_data,
        )
        .unwrap();

        let err = super::validate_section_atom_identity(
            "forces",
            99,
            [false; 3],
            0,
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("no coordinate atom"));
        let atom = &frame.atom_data[0];
        let mut other = atom.fixed;
        other[0] = !other[0];
        let err = super::validate_section_atom_identity(
            "forces",
            0,
            other,
            atom.atom_id,
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("fixed mask"));
        let err = super::validate_section_atom_identity(
            "forces",
            0,
            atom.fixed,
            atom.atom_id.wrapping_add(1),
            &frame.atom_data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("atom_id mismatch"));

        let err = super::validate_section_arity_value(&serde_json::json!("no")).unwrap_err();
        assert!(err.to_string().contains("section_arity"));
        let err = super::validate_section_arity_value(&serde_json::json!({"velocities": 0}))
            .unwrap_err();
        assert!(matches!(err, ParseError::InvalidCount(_)));
        let err = super::validate_metadata_integer("frame_index", &serde_json::json!("x"))
            .unwrap_err();
        assert!(err.to_string().contains("non-negative integer"));
        let mut obj = serde_json::Map::new();
        obj.insert("validate".into(), serde_json::json!(true));
        let err = super::validate_metadata_schema(&obj).unwrap_err();
        assert!(err.to_string().contains("sections"));
        let err = super::validate_lattice_vectors_metadata(&serde_json::json!("no")).unwrap_err();
        assert!(err.to_string().contains("lattice_vectors"));
        let err = super::validate_masses(&[-1.0]).unwrap_err();
        assert!(matches!(err, ParseError::ValidationError(_)));

        let err = super::parse_f64_at(b"\n", 0, 0).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }));
        let err = super::parse_flag_at(b"0\n", 0, 1).unwrap();
        assert_eq!(err.0, [false, false, false]);
        let err = super::parse_flag_at(b"1\n", 0, 1).unwrap();
        assert_eq!(err.0, [true, true, true]);
        let mut schema = serde_json::Map::new();
        schema.insert("validate".into(), serde_json::json!("no"));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("validate".into(), serde_json::json!(false));
        schema.insert("sections".into(), serde_json::json!("no"));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("sections".into(), serde_json::json!([1]));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("sections".into(), serde_json::json!([]));
        schema.insert("con_spec_minor".into(), serde_json::json!("x"));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("con_spec_minor".into(), serde_json::json!(0));
        schema.insert("generator".into(), serde_json::json!(1));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("generator".into(), serde_json::json!("t"));
        schema.insert("potential".into(), serde_json::json!("no"));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("potential".into(), serde_json::json!({"type": 1}));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("potential".into(), serde_json::json!({"type": "EMT"}));
        schema.insert("converged".into(), serde_json::json!("no"));
        assert!(super::validate_metadata_schema(&schema).is_err());
        schema.insert("converged".into(), serde_json::json!(true));
        assert!(super::validate_metadata_schema(&schema).is_ok());

        let mut header = frame.header.clone();
        header.spec_version = 3;
        header.metadata.insert("con_spec_minor".into(), serde_json::json!(1));
        header.sections = vec!["velocities".into()];
        assert!(super::reject_velocities_without_time(&header).is_err());
        let err = super::validate_coordinate_component(0, "Cu", "not a label").unwrap_err();
        assert!(err.to_string().contains("coordinate label"));
    }

    #[test]
    fn a_peekable_reports_the_bytes_still_ahead() {
        use super::LineStream;
        let lines = ["a", "bb", ""];
        let mut peek = lines.iter().copied().peekable();
        assert_eq!(peek.bytes_remaining(), Some(1 + 1 + 2 + 1 + 0 + 1));
        assert!(peek.blank_opens_velocity_block());
        let mut empty = std::iter::empty().peekable();
        assert_eq!(empty.bytes_remaining(), Some(0));
        assert!(empty.blank_opens_velocity_block());
    }

    #[test]
    fn declared_sections_and_nonfinite_numbers_fail() {
        use super::LineStream;
        let mut bare = super::IterStream(std::iter::empty::<&str>());
        assert!(bare.peek_line().is_none());
        assert!(bare.next_line().is_none());
        let err = super::parse_finite_at("1e999", 0).unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)));
        let err = super::parse_atom_id_token("18446744073709551616").unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));

        let frames = crate::iterators::frames_from_text(
            &std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap(),
            Some(1),
        )
        .unwrap();
        let mut frame = frames.into_iter().next().unwrap();
        frame.header.strict_validation = true;
        for name in [
            "forces",
            "energies",
            "charges",
            "spins",
            "magmoms",
            "displacements",
            "spreads",
        ] {
            frame.header.sections = vec![name.to_string()];
            let mut lines = std::iter::empty().peekable();
            let err = super::parse_declared_sections(
                &mut lines,
                &mut frame.header,
                &mut frame.atom_data,
            )
            .unwrap_err();
            assert!(matches!(err, ParseError::IncompleteSection(_)), "{name} {err}");
        }
        frame.header.sections = vec!["custom".into()];
        let mut lines = std::iter::empty().peekable();
        let err =
            super::parse_declared_sections(&mut lines, &mut frame.header, &mut frame.atom_data)
                .unwrap_err();
        assert!(matches!(err, ParseError::UnknownSection(_)));

        let body = "\nCu\nEnergies of Component 1\n1.0 0 0\n1.0 0 1\nH\nEnergies of Component 2\n0.5 0 2\n0.5 0 3\n";
        let mut energy_lines = body.lines().peekable();
        let energy = super::parse_energy_section(
            &mut energy_lines,
            &frame.header,
            &mut frame.atom_data,
        );
        assert!(energy.is_err());

        let spreads = "\nCu\nSpreads of Component 1\nnan nan nan 0 0\n0 0 0 0 1\nH\nSpreads of Component 2\n0 0 0 0 2\n0 0 0 0 3\n";
        let mut spread_lines = spreads.lines().peekable();
        let err = super::parse_spread_section(
            &mut spread_lines,
            &frame.header,
            &mut frame.atom_data,
        )
        .unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)), "{err}");
    }

    #[test]
    fn cold_parser_branches() {
        assert!(matches!(
            parse_finite_at("", 0),
            Err(ParseError::InvalidVectorLength {
                expected: 3,
                found: 0
            })
        ));
        assert!(matches!(
            parse_finite_at("inf", 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_finite_at("infinity", 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_finite_at("nan", 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_finite_at("1.0x", 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_finite_at("nope", 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_finite_at("1e9999", 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_fixed_flag("", 2),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert!(matches!(
            parse_fixed_flag("1.5", 2),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert_eq!(parse_fixed_flag("1", 1).unwrap(), [true, true, true]);
        assert!(matches!(
            parse_fixed_flag("2", 1),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert!(matches!(
            parse_fixed_flag("8", 2),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert_eq!(parse_fixed_flag("5", 2).unwrap(), [true, false, true]);
        assert!(matches!(
            parse_atom_id_token(""),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_atom_id_token("-3"),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_atom_id_token(&"9".repeat(40)),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_coord_row("1 2", 2, 0),
            Err(ParseError::InvalidVectorLength { .. })
        ));
        assert!(matches!(
            parse_coord_row("1 2 3 0 1 extra", 2, 0),
            Err(ParseError::InvalidVectorLength { expected: 5, .. })
        ));
        assert!(matches!(
            parse_vector_section_row("0 0 0 0", 2),
            Err(ParseError::InvalidVectorLength { expected: 5, .. })
        ));
        assert_eq!(count_tokens_on_line(b"  a   b\t", 0), 2);
        assert_eq!(count_tokens_on_line(b"", 0), 0);
        assert!(matches!(
            parse_count_line("-1 2", 2),
            Err(ParseError::InvalidCount(_))
        ));
        assert!(matches!(
            parse_count_line(&"9".repeat(30), 1),
            Err(ParseError::InvalidCount(_))
        ));
        assert!(matches!(
            parse_frame_header(&mut ["only-one"].into_iter()),
            Err(ParseError::IncompleteHeader)
        ));
        let array_header = parse_frame_header(&mut ["line", "[1]"].into_iter());
        assert!(
            matches!(array_header, Err(ParseError::IncompleteHeader)),
            "{array_header:?}"
        );
        assert!(matches!(
            parse_frame_header(&mut ["line", "{\"con_spec_version\":\"no\"}"].into_iter()),
            Err(ParseError::InvalidMetadataJson(_))
        ));
        assert!(matches!(
            parse_frame_header(&mut ["line", "{}"].into_iter()),
            Err(ParseError::MissingSpecVersion)
        ));
        assert!(matches!(
            parse_frame_header(&mut ["\u{feff}line", "{}"].into_iter()),
            Err(ParseError::MissingSpecVersion)
        ));
        assert!(matches!(
            parse_f64_at(b"", 0, 0),
            Err(ParseError::InvalidVectorLength { .. })
        ));
        assert!(matches!(
            parse_f64_at(b"\xff rest", 0, 0),
            Err(ParseError::ValidationError(_))
        ));
        assert!(matches!(
            parse_f64_at(b"inf", 0, 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_f64_at(b"1.0x", 0, 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_f64_at(b"abc", 0, 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_f64_at(b"1e9999", 0, 0),
            Err(ParseError::NonFiniteValue(_))
        ));
        assert!(matches!(
            parse_coord_fields_detailed(b"\n", 0, 2, None),
            Err(ParseError::InvalidVectorLength { .. })
        ));
        assert!(parse_coord_fields_detailed(b"1.0 2.0 3.0 0 4\n", 0, 2, Some(0)).is_ok());
        assert!(matches!(
            parse_coord_fields_detailed(b"1.0 2.0 3.0 no\n", 0, 2, None),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert!(matches!(
            parse_flag_at(b"9\n", 0, 2),
            Err(ParseError::InvalidFixedFlag(_))
        ));
        assert_eq!(parse_flag_at(b"1\n", 0, 1).unwrap().0, [true, true, true]);
        assert!(matches!(
            parse_id_at(b"", 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_id_at(b"12x", 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));
        assert!(matches!(
            parse_id_at(&b"9".repeat(40), 0),
            Err(ParseError::InvalidNumberFormat(_))
        ));

        let good = [
            "user",
            "{\"con_spec_version\":2}",
            "10 10 10",
            "90 90 90",
            "post1",
            "post2",
            "1",
            "1",
            "1.0",
            "H",
            "Coordinates of Component 1",
            "0 0 0 0 1",
        ];
        let frame = parse_single_frame(&mut good.into_iter()).unwrap();
        let mut header = frame.header.clone();
        let mut atoms = frame.atom_data.clone();
        for name in [
            "velocities",
            "forces",
            "energies",
            "charges",
            "spins",
            "magmoms",
            "displacements",
            "spreads",
        ] {
            header.sections = vec![name.to_string()];
            let mut empty = std::iter::empty::<&str>().peekable();
            assert!(matches!(
                parse_declared_sections(&mut empty, &mut header, &mut atoms),
                Err(ParseError::IncompleteSection(_))
            ));
        }
        assert!(matches!(
            validate_section_component("Energies", 0, 0, "H", "wrong label", &header, &atoms),
            Err(ParseError::ValidationError(_))
        ));
        header.natms_per_type[0] = 0;
        assert!(
            validate_section_component(
                "Energies",
                0,
                0,
                "H",
                "Energies of Component 1",
                &header,
                &atoms,
            )
            .is_ok()
        );
        header.natms_per_type[0] = 1;
        assert!(matches!(
            validate_section_atom_identity("energies", 9, [false, false, false], 0, &atoms),
            Err(ParseError::ValidationError(_))
        ));
        assert!(matches!(
            validate_section_atom_identity("energies", 0, [true, false, false], 1, &atoms),
            Err(ParseError::ValidationError(_))
        ));
        assert!(matches!(
            validate_section_atom_identity("energies", 0, atoms[0].fixed, 99, &atoms),
            Err(ParseError::ValidationError(_))
        ));

        let mut energy_lines = ["", "H", "not a label"].into_iter().peekable();
        assert!(matches!(
            parse_energy_section(&mut energy_lines, &header, &mut atoms),
            Err(ParseError::IncompleteSection(_))
        ));

        use serde_json::{Map, Value, json};
        let shapes = [
            json!({"bonds": "no"}),
            json!({"bonds": [[1]]}),
            json!({"bonds": [["x", 1]]}),
            json!({"bonds": [[u64::MAX, 1]]}),
            json!({"bonds": [{"j": 1}]}),
            json!({"bonds": [{"i": "x", "j": 1}]}),
            json!({"bonds": [{"i": u64::MAX, "j": 1}]}),
            json!({"bonds": [{"i": 0, "j": 1, "order": "no"}]}),
            json!({"bonds": ["no"]}),
            json!({"lattice_vectors": "no"}),
            json!({"lattice_vectors": [[1.0, 0.0, 0.0]]}),
            json!({"lattice_vectors": [1.0, [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]}),
            json!({"lattice_vectors": [[1.0, 0.0, "a"], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]}),
            json!({"section_arity": "no"}),
            json!({"section_arity": {"velocities": "x"}}),
            json!({"section_arity": {"velocities": 0}}),
            json!({"section_arity": {"energies": 3}}),
            json!({"potential": {"type": 1}}),
            json!({"units": 1}),
            json!({"pbc": [true]}),
            json!({"energy": "hot"}),
            json!(1),
        ];
        let mut skipped = 0usize;
        for value in shapes {
            let Value::Object(map) = value else {
                skipped += 1;
                continue;
            };
            assert!(validate_metadata_schema(&map).is_err(), "{map:?}");
        }
        assert_eq!(skipped, 1);
        let good_meta = [
            json!({"bonds": [[0, 1], {"i": 0, "j": 1, "order": 2}]}),
            json!({"lattice_vectors": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]}),
            json!({"converged": true}),
            json!(1),
        ];
        let mut skipped = 0usize;
        for value in good_meta {
            let Value::Object(map) = value else {
                skipped += 1;
                continue;
            };
            assert!(validate_metadata_schema(&map).is_ok(), "{map:?}");
        }
        assert_eq!(skipped, 1);
        let mut ok = Map::new();
        ok.insert("converged".into(), Value::Bool(true));
        assert!(validate_metadata_schema(&ok).is_ok());
    }

    #[test]
    fn cold_header_and_row_arms_name_their_errors() {
        let angles = [90.0, 90.0, 90.0];
        let lengths = [1.0, 1.0, 1.0];
        let bad_geom = [
            ([f64::NAN, 1.0, 1.0], angles),
            ([0.0, 1.0, 1.0], angles),
            ([-1.0, 1.0, 1.0], angles),
            (lengths, [f64::NAN, 90.0, 90.0]),
            (lengths, [0.0, 90.0, 90.0]),
            (lengths, [180.0, 90.0, 90.0]),
        ];
        for (boxl, ang) in bad_geom {
            let err = validate_header_geometry(&boxl, &ang).unwrap_err();
            assert!(
                matches!(err, ParseError::ValidationError(_)),
                "{boxl:?} {ang:?} {err}"
            );
        }
        assert!(validate_header_geometry(&lengths, &angles).is_ok());
        assert!(matches!(
            validate_masses(&[f64::NAN]).unwrap_err(),
            ParseError::ValidationError(_)
        ));
        assert!(matches!(
            validate_masses(&[0.0]).unwrap_err(),
            ParseError::ValidationError(_)
        ));
        assert!(validate_masses(&[1.0]).is_ok());
        assert!(
            validate_coordinate_component(0, "X", "Coordinates of Component 1").is_ok()
        );
        assert!(
            validate_coordinate_component(0, "H", "Coordinates of Component 1").is_ok()
        );
        assert!(matches!(
            validate_coordinate_component(0, "Zz", "Coordinates of Component 1").unwrap_err(),
            ParseError::ValidationError(_)
        ));

        let huge = "user\n{\"con_spec_version\":4294967296}\n";
        let err = parse_frame_header(&mut huge.lines()).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(_)), "{err}");
        let neg = "user\n{\"con_spec_version\":-1}\n";
        let err = parse_frame_header(&mut neg.lines()).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(0)), "{err}");
        let frac = "user\n{\"con_spec_version\":1.5}\n";
        let err = parse_frame_header(&mut frac.lines()).unwrap_err();
        assert!(matches!(err, ParseError::InvalidMetadataJson(_)), "{err}");
        assert_eq!(json_u64(&serde_json::json!(3.0)), Some(3));
        assert_eq!(json_u64(&serde_json::json!(1.5)), None);
        assert_eq!(json_u64(&serde_json::json!(-2)), None);
        assert!(json_negative_integer(&serde_json::json!(-2)));
        assert!(json_negative_integer(&serde_json::json!(-2.0)));
        assert!(!json_negative_integer(&serde_json::json!(-1.5)));
        assert!(!json_negative_integer(&serde_json::json!("no")));
        assert!(json_nonneg_integer(&serde_json::json!(4)));
        assert!(json_nonneg_integer(
            &serde_json::json!(18446744073709551616.0)
        ));
        assert!(!json_nonneg_integer(&serde_json::json!(1.5)));
        assert!(!json_nonneg_integer(&serde_json::json!("no")));

        let mut out = [0.0; 5];
        let err = parse_line_of_range_f64_stack("1e999", 1, 1, &[0.0], &mut out).unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)), "{err}");
        let err = parse_line_of_range_f64_stack("1.2abc", 1, 1, &[0.0], &mut out).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)), "{err}");
        let err = parse_line_of_range_f64_stack("1", 2, 3, &[0.0, 0.0, 0.0], &mut out).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InvalidVectorLength { found: 1, .. }
        ));
        let n = parse_line_of_range_f64_stack("1 2", 1, 3, &[9.0, 8.0, 7.0], &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out[0], 1.0);
        assert_eq!(out[2], 7.0);
        let err = parse_line_of_range_f64_stack("1 2 3 4 5 6", 1, 5, &[0.0; 5], &mut out).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InvalidVectorLength { expected: 5, .. }
        ));

        let row = parse_coord_fields(b"1 2 3 1 4\n", 0, 2, Some(9)).unwrap();
        assert_eq!(row.0.0, [1.0, 2.0, 3.0]);
        assert_eq!(row.0.1, [true, false, false]);
        assert_eq!(row.0.2, 4);
        let row = parse_coord_fields(b"1 2 3 1\n", 0, 1, Some(9)).unwrap();
        assert_eq!(row.0.1, [true, true, true]);
        assert_eq!(row.0.2, 9);
        let err = parse_coord_fields(b"1 2 3 1\n", 0, 2, None).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InvalidVectorLength { found: 4, .. }
        ));
        let err = parse_coord_fields(b"1 2 3 8 1\n", 0, 2, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)), "{err}");
        let err = parse_coord_fields(&[0xff, b'\n'], 0, 2, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::ValidationError(_)), "{err}");
        let wide = parse_coord_fields(b"1 2 3 0 1234567890123456789\n", 0, 2, Some(0)).unwrap();
        assert_eq!(wide.0.2, 1234567890123456789);
        let err = parse_coord_fields(b"1 2 3 0 123456789012345678901\n", 0, 2, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)), "{err}");
        let err = parse_coord_fields(b"1 2 3 0 1 extra\n", 0, 2, Some(0)).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InvalidVectorLength { found: 6, .. }
        ));

        let mut lines = ["", "H"].into_iter().peekable();
        assert!(!section_follows(&mut lines));
        let mut lines = ["{meta", "nope"].into_iter().peekable();
        assert!(!section_follows(&mut lines));
        let mut lines = ["H"].into_iter().peekable();
        assert!(!section_follows(&mut lines));
        let mut lines = ["Cu", "velocities of Component 1"].into_iter().peekable();
        assert!(section_follows(&mut lines));
        let mut probe = ["", "Cu", "velocities of Component 1"]
            .into_iter()
            .peekable();
        assert!(probe.blank_line_opens_section());
        let mut bare = ["not blank"].into_iter().peekable();
        assert!(bare.blank_opens_velocity_block());
    }

    #[test]
    fn truncated_and_illegal_text_is_an_error() {
        assert!(ConFrameIterator::new("").next().is_none());
        let err = ConFrameIterator::new("only\n").next().unwrap().unwrap_err();
        assert!(matches!(err, ParseError::IncompleteHeader), "{err}");
        let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
        let err = ConFrameIterator::new(&text[..40]).next().unwrap().unwrap_err();
        assert!(!err.to_string().is_empty(), "{err}");
        let mut bytes = text.into_bytes();
        bytes[20] = 0xff;
        let err = ConFrameIterator::from_bytes(&bytes).next().unwrap().unwrap_err();
        let plain = ParseError::ValidationError("plain".into());
        for err in [&err, &plain] {
            assert!(
                err.to_string().to_ascii_lowercase().contains("utf-8")
                    || matches!(err, ParseError::ValidationError(_)),
                "{err}"
            );
        }
        let mut empty = std::iter::empty::<&str>().peekable();
        assert!(!empty.blank_line_opens_section());
        let mut junk = ["xyz"].into_iter().peekable();
        assert!(!junk.blank_line_opens_section());
        let counted = parse_line_of_range_f64("1 2 3", 3, 3, &[0.0, 0.0, 0.0]).unwrap();
        assert_eq!(counted, vec![1.0, 2.0, 3.0]);
        let err = parse_line_of_range_f64("nan", 1, 1, &[0.0]).unwrap_err();
        assert!(matches!(err, ParseError::NonFiniteValue(_)), "{err}");
    }

    #[test]
    fn numeric_edges_and_section_gaps_name_the_failure() {
        let mut empty = std::iter::empty::<&str>().peekable();
        assert!(!section_follows(&mut empty));
        let mut blank = ["", "H", "velocities of Component 1"].into_iter().peekable();
        assert!(blank.blank_opens_velocity_block());
        let mut words = ["not blank"].into_iter().peekable();
        assert!(words.blank_opens_velocity_block());

        let err = parse_id_at(b"\n", 0).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let err = parse_id_at(b"", 0).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)));
        let err = parse_id_at(b"x", 0).unwrap_err();
        assert!(err.to_string().contains("invalid integer"));
        assert_eq!(parse_id_at(b"12 ", 0).unwrap(), (12, 2));

        let err = parse_flag_at(b"\n", 0, 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { found: 3, .. }));
        let err = parse_flag_at(b"", 0, 2).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { found: 3, .. }));
        let err = parse_coord_fields(b"1 2\n", 0, 2, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }), "{err}");
        let err = parse_coord_fields(b"1 2 3 2\n", 0, 1, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidFixedFlag(_)), "{err}");
        let err = parse_coord_fields("é 0 0 0 1".as_bytes(), 0, 2, Some(0)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumberFormat(_)), "{err}");
        let err = parse_coord_fields(b"1 2 3\n0 4", 0, 2, Some(9)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { .. }), "{err}");
        let row = parse_coord_fields(b"1 2 3 0 4\n", 0, 2, Some(9)).unwrap();
        assert_eq!(row.0.2, 4);
        assert_eq!(row.1, b"1 2 3 0 4\n".len());

        assert!(json_u64(&serde_json::json!(1e20)).is_none());
        assert_eq!(json_u64(&serde_json::json!(3.0)), Some(3));
        assert!(!json_negative_integer(&serde_json::json!(1.5)));
        assert!(json_negative_integer(&serde_json::json!(-2.0)));
        assert!(json_nonneg_integer(&serde_json::json!(1e20)));
        assert!(!json_nonneg_integer(&serde_json::json!(-2.0)));

        let lines = ["user", "[1]", "1 1 1", "90 90 90", "", "", "1", "1", "1.0"];
        let header = parse_frame_header(&mut lines.into_iter()).unwrap();
        assert_eq!(header.spec_version, 1);
        assert_eq!(header.prebox_header.metadata_line(), "[1]");

        let mut map = serde_json::Map::new();
        map.insert("potential".into(), serde_json::json!({"params": 1}));
        validate_metadata_schema(&map).unwrap();
        map.clear();
        map.insert(
            "lattice_vectors".into(),
            serde_json::json!([[1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        );
        assert!(validate_metadata_schema(&map).is_err());
        map.clear();
        map.insert("bonds".into(), serde_json::json!([{"i": 0, "j": 1}]));
        validate_metadata_schema(&map).unwrap();

        let text = frame_with_count("1", false);
        let frame = ConFrameIterator::new(&text).next().unwrap().unwrap();
        let mut header = frame.header.clone();
        header.metadata.insert(
            meta::SECTION_ARITY.into(),
            serde_json::json!({"widgets": 0}),
        );
        assert!(section_row_arity(&header, "widgets").is_none());
        header.metadata.insert(
            meta::SECTION_ARITY.into(),
            serde_json::json!({"widgets": 2}),
        );
        assert_eq!(section_row_arity(&header, "widgets"), Some(2));
        header.metadata.insert(
            meta::CON_SPEC_MINOR.into(),
            serde_json::json!(2),
        );
        header.metadata.insert(
            meta::SECTION_ARITY.into(),
            serde_json::json!({"widgets": 0}),
        );
        header.sections = vec!["widgets".into()];
        header.sections_declared = true;
        let mut lines = ["\n"].into_iter().peekable();
        let err = parse_declared_sections(&mut lines, &mut header, &mut []).unwrap_err();
        assert!(matches!(err, ParseError::UnknownSection(ref name) if name == "widgets"));

        let mut ws = crate::iterators::MemchrLines::new("   \n  \n");
        assert!(!open_section(&mut ws).unwrap());
        let mut declared = frame.header.clone();
        declared.sections_declared = true;
        let mut follow = ["Cu"].into_iter().peekable();
        let err = parse_velocity_section(&mut follow, &declared, &mut []).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));

        let mut velocity = [
            "",
            "H",
            "Velocities of Component 1",
            "0.1 0.0 0.0 0 1",
        ]
        .into_iter()
        .peekable();
        assert!(parse_velocity_section(&mut velocity, &frame.header, &mut []).unwrap());
        let mut forces = ["Cu"].into_iter().peekable();
        assert!(!parse_force_section(&mut forces, &frame.header, &mut []).unwrap());

        let mut bad_energy = ["", "H", "Not energies", "1 0 1"].into_iter().peekable();
        let err = parse_energy_section(&mut bad_energy, &frame.header, &mut []).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));

        let mut spread = [
            "",
            "H",
            "Spreads of Component 1",
            "-1 0 0 0 1",
        ]
        .into_iter()
        .peekable();
        let err = parse_spread_section(&mut spread, &frame.header, &mut frame.atom_data.clone())
            .unwrap_err();
        assert!(err.to_string().contains("negative"), "{err}");
        let mut unlabeled = ["", "H", "Not spreads", "0 0 0 0 1"].into_iter().peekable();
        let err = parse_spread_section(&mut unlabeled, &frame.header, &mut []).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));

        let mut missing = std::iter::empty::<&str>().peekable();
        let err = skip_unknown_section(&mut missing, &frame.header, "widgets", 1).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));

        let float32 = text.replacen(
            "\"sections\":[]",
            "\"sections\":[],\"storage_dtypes\":{\"positions\":\"float32\"}",
            1,
        );
        let stored = ConFrameIterator::new(&float32).next().unwrap().unwrap();
        assert!(stored.positions.f64_slice().is_none());
        assert_eq!(stored.positions.as_f64_row(0), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn declared_sections_take_the_remaining_arms() {
        let text = frame_with_count("1", false);
        let frame = ConFrameIterator::new(&text).next().unwrap().unwrap();
        let mut header = frame.header.clone();
        header.strict_validation = false;
        let mut atoms = frame.atom_data.clone();
        let row = "\nCu\nVelocities of Component 1\n0.1 0.2 0.3 0 1\n";

        let six = parse_line_of_range_f64("1 2 3 4 5 6", 6, 6, &[0.0; 6]).unwrap();
        assert_eq!(six, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let mut blanks = ["", "Cu", "Velocities of Component 1"]
            .into_iter()
            .peekable();
        assert!(blanks.blank_opens_velocity_block());

        let mut words = crate::iterators::MemchrLines::new("Cu\n");
        assert_eq!(open_section(&mut words).unwrap(), false);

        header.sections_declared = false;
        header.sections.clear();
        let mut only_ws = crate::iterators::MemchrLines::new("\n  \n");
        assert_eq!(
            parse_velocity_section(&mut only_ws, &header, &mut atoms).unwrap(),
            false
        );
        let mut not_blank = crate::iterators::MemchrLines::new("Cu\n");
        assert_eq!(
            parse_velocity_section(&mut not_blank, &header, &mut atoms).unwrap(),
            false
        );
        let mut not_a_block = crate::iterators::MemchrLines::new("\nnot a label\n");
        assert_eq!(
            parse_velocity_section(&mut not_a_block, &header, &mut atoms).unwrap(),
            false
        );

        header.sections_declared = true;
        let mut declared = crate::iterators::MemchrLines::new("Cu\n");
        let err = parse_velocity_section(&mut declared, &header, &mut []).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));
        let mut short_atoms = crate::iterators::MemchrLines::new(row);
        assert_eq!(
            parse_velocity_section(&mut short_atoms, &header, &mut []).unwrap(),
            true
        );
        let force = "\nH\nForces of Component 1\n0.1 0.2 0.3 0 1\n";
        let mut forces = crate::iterators::MemchrLines::new(force);
        assert_eq!(parse_force_section(&mut forces, &header, &mut []).unwrap(), true);
        let mut no_blank = crate::iterators::MemchrLines::new("H\n");
        assert_eq!(
            parse_force_section(&mut no_blank, &header, &mut atoms).unwrap(),
            false
        );
        header.strict_validation = true;
        let mut checked = crate::iterators::MemchrLines::new(force);
        assert_eq!(
            parse_force_section(&mut checked, &header, &mut atoms).unwrap(),
            true
        );
        header.strict_validation = false;
        let energy = "\nCu\nEnergies of Component 1\n4.5 0 1\n";
        let mut energies = crate::iterators::MemchrLines::new(energy);
        assert_eq!(
            parse_energy_section(&mut energies, &header, &mut []).unwrap(),
            true
        );
        let charge = "\nCu\nCharges of Component 1\n0.2 0 1\n";
        let mut charges = crate::iterators::MemchrLines::new(charge);
        assert_eq!(
            parse_charge_section(&mut charges, &header, &mut []).unwrap(),
            true
        );
        let disp = "\nCu\nDisplacements of Component 1\n0.1 0.2 0.3 0 1\n";
        let mut displacements = crate::iterators::MemchrLines::new(disp);
        assert_eq!(
            parse_displacement_section(&mut displacements, &header, &mut []).unwrap(),
            true
        );

        let mut stored = header.clone();
        stored.sections_declared = false;
        stored.sections = vec!["velocities".into()];
        stored
            .metadata
            .insert(meta::CON_SPEC_MINOR.into(), serde_json::json!(0));
        let mut body = crate::iterators::MemchrLines::new(row);
        assert_eq!(
            parse_declared_sections(&mut body, &mut stored, &mut atoms).unwrap(),
            1
        );

        let mut old = header.clone();
        old.sections_declared = true;
        old.sections = vec!["widgets".into()];
        old.metadata
            .insert(meta::CON_SPEC_MINOR.into(), serde_json::json!(0));
        let mut none = crate::iterators::MemchrLines::new("\n");
        let err = parse_declared_sections(&mut none, &mut old, &mut atoms).unwrap_err();
        assert!(matches!(err, ParseError::UnknownSection(_)));

        let mut newer = header.clone();
        newer.sections_declared = true;
        newer.sections = vec!["widgets".into()];
        newer
            .metadata
            .insert(meta::CON_SPEC_MINOR.into(), serde_json::json!(2));
        newer.metadata.remove(meta::SECTION_ARITY);
        let mut missing_arity = crate::iterators::MemchrLines::new("\n");
        let err = parse_declared_sections(&mut missing_arity, &mut newer, &mut atoms).unwrap_err();
        assert!(matches!(err, ParseError::UnknownSection(_)));

        newer.metadata.insert(
            meta::SECTION_ARITY.into(),
            serde_json::json!({"widgets": 1}),
        );
        newer.sections = vec!["widgets".into()];
        let mut cut = crate::iterators::MemchrLines::new("\n");
        let err = parse_declared_sections(&mut cut, &mut newer, &mut atoms).unwrap_err();
        assert!(matches!(err, ParseError::IncompleteSection(_)));
    }

    #[test]
    fn memchr_header_and_labels_take_each_rejected_value() {
        for (text, needle) in [
            (
                "user\n{\"con_spec_version\":4294967296}\n",
                "unsupported con_spec_version",
            ),
            ("user\n{\"con_spec_version\":0}\n", "unsupported con_spec_version"),
            ("user\n{\"con_spec_version\":1}\n", "unsupported con_spec_version"),
            ("user\n{\"con_spec_version\":9}\n", "unsupported con_spec_version"),
            ("user\n{\"con_spec_version\":-3}\n", "unsupported con_spec_version"),
            (
                "user\n{\"con_spec_version\":2,\"section_arity\":{\"widgets\":0}}\n",
                ">= 1",
            ),
            (
                "user\n{\"con_spec_version\":2,\"section_arity\":{\"velocities\":1}}\n",
                "expected 3",
            ),
        ] {
            let err = ConFrameIterator::new(text).next().unwrap().unwrap_err();
            assert!(err.to_string().contains(needle), "{text} -> {err}");
        }
        assert!(!line_has_component_label("", 1));
        assert!(!line_has_component_label("speed of component 1", 1));
        assert!(!line_has_component_label("velocities on component 1", 1));
        assert!(!line_has_component_label("velocities of part 1", 1));
        assert!(!line_has_component_label("velocities of component 2", 1));
        assert!(!line_has_component_label("velocities of component 1 extra", 1));
        assert!(line_has_component_label("Velocities of Component 1", 1));

        let mut labelled = ["Cu", "Velocities of Component 1"].into_iter().peekable();
        assert!(section_follows(&mut labelled));
        assert!(!section_follows(&mut labelled));
        let mut short = ["only-symbol", "ignored"].into_iter().peekable();
        assert!(short.next().is_some());
        assert!(!section_follows(&mut short));
        let blank_lines = ["", "Cu", "Velocities of Component 1"];
        let mut blanks = blank_lines.iter().copied().peekable();
        assert!(blanks.blank_line_opens_section());
        let words = ["not blank", "Cu"];
        let mut words = words.iter().copied().peekable();
        assert!(!words.blank_line_opens_section());
        let none: &[&str] = &[];
        let mut none = none.iter().copied().peekable();
        assert!(!none.blank_line_opens_section());
        assert!(none.blank_opens_velocity_block());

        let three = parse_line_of_range_f64("1 2 3", 3, 3, &[0.0; 3]).unwrap();
        assert_eq!(three, vec![1.0, 2.0, 3.0]);
        let six = parse_line_of_range_f64("1 2 3 4 5 6", 6, 6, &[0.0; 6]).unwrap();
        assert_eq!(six, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let short = b"";
        let err = parse_coord_fields_detailed(short, 0, 2, Some(1)).unwrap_err();
        assert!(matches!(err, ParseError::InvalidVectorLength { found: 0, .. }), "{err}");
    }

}
