//! CON iterators behind the C ABI.
//!
//! A path iterator parses through [`crate::stream::ConFrameReader`], so the
//! buffer stays near one frame plus one read. A string or byte-buffer
//! iterator copies the text the caller already owns.

use crate::iterators::ConFrameIterator;
use crate::stream::ConFrameReader;
use crate::types::ConFrame;
use std::io::Read;
use std::path::Path;
use std::ptr;

struct TextCursor {
    contents: *mut String,
    iter: *mut ConFrameIterator<'static>,
}

impl TextCursor {
    fn new(text: String) -> Self {
        let contents = Box::into_raw(Box::new(text));
        let iter = Box::into_raw(Box::new(ConFrameIterator::new(unsafe { &*contents })));
        Self { contents, iter }
    }

    fn iter(&mut self) -> &mut ConFrameIterator<'static> {
        unsafe { &mut *self.iter }
    }
}

impl Drop for TextCursor {
    fn drop(&mut self) {
        if !self.iter.is_null() {
            drop(unsafe { Box::from_raw(self.iter) });
            self.iter = ptr::null_mut();
        }
        if !self.contents.is_null() {
            drop(unsafe { Box::from_raw(self.contents) });
            self.contents = ptr::null_mut();
        }
    }
}

enum Body {
    Text(TextCursor),
    File(ConFrameReader<Box<dyn Read + Send>>),
}

/// Owned iterator state. The C struct holds one pointer to this value.
pub struct Cursor {
    body: Body,
}

impl Cursor {
    pub fn from_text(text: String) -> Self {
        Self {
            body: Body::Text(TextCursor::new(text)),
        }
    }

    pub fn open_file(path: &Path, read_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let source = crate::compression::open_decoded(path)?;
        Ok(Self {
            body: Body::File(ConFrameReader::with_read_size(source, read_size.max(1))),
        })
    }

    pub fn next_frame(&mut self) -> Option<Result<ConFrame, String>> {
        match &mut self.body {
            Body::Text(text) => text
                .iter()
                .next()
                .map(|item| item.map_err(|err| err.to_string())),
            Body::File(reader) => reader
                .next()
                .map(|item| item.map_err(|err| err.to_string())),
        }
    }

    pub fn forward(&mut self) -> Option<Result<(), String>> {
        match &mut self.body {
            Body::Text(text) => text
                .iter()
                .forward()
                .map(|item| item.map_err(|err| err.to_string())),
            Body::File(reader) => reader
                .forward()
                .map(|item| item.map_err(|err| err.to_string())),
        }
    }

    /// How many frames `forward` skipped. `Err` is a parse error after `Ok` skips.
    pub fn skip(&mut self, n: usize) -> Result<usize, String> {
        let mut skipped = 0usize;
        for _ in 0..n {
            match self.forward() {
                Some(Ok(())) => skipped += 1,
                Some(Err(err)) => return Err(err),
                None => break,
            }
        }
        Ok(skipped)
    }

    pub fn buffer_capacity(&self) -> Option<usize> {
        match &self.body {
            Body::File(reader) => Some(reader.buffer_capacity()),
            Body::Text(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::{
        RKRStatus, con_frame_iterator_forward, con_frame_iterator_next, con_frame_iterator_nth,
        con_frame_iterator_skip, free_con_frame_iterator, invalid_c_iterator,
        open_file_iterator_sized, read_con_file_iterator,
    };
    use crate::iterators::read_all_frames;
    use std::ffi::CString;
    use std::io::Write;
    use std::path::Path;

    fn cu_frame(n: usize, x0: f64, side: f64) -> String {
        let mut text = format!(
            "user\n{{\"con_spec_version\":2}}\n{side} {side} {side}\n90.0 90.0 90.0\n0 0\n0 0\n1\n{n}\n63.546\nCu\nCoordinates of Component 1\n"
        );
        for index in 0..n {
            let x = x0 + index as f64;
            text.push_str(&format!("{x} 0.0 0.0 0 {index}\n"));
        }
        text
    }

    fn write_con(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create");
        let wrote = file.write(text.as_bytes()).expect("write");
        assert_eq!(wrote, text.len());
        path
    }

    fn assert_same(left: &ConFrame, right: &ConFrame) {
        assert_eq!(left.header.boxl, right.header.boxl);
        assert_eq!(left.header.angles, right.header.angles);
        assert_eq!(left.atom_data.len(), right.atom_data.len());
        let mut max_dx = 0.0_f64;
        for (a, b) in left.atom_data.iter().zip(right.atom_data.iter()) {
            assert_eq!(a.symbol.as_ref(), b.symbol.as_ref());
            max_dx = max_dx
                .max((a.x - b.x).abs())
                .max((a.y - b.y).abs())
                .max((a.z - b.z).abs());
        }
        assert_eq!(max_dx, 0.0);
    }

    fn take_frames(iterator: *mut crate::ffi::CConFrameIterator) -> Vec<ConFrame> {
        let mut frames = Vec::new();
        loop {
            let handle = unsafe { con_frame_iterator_next(iterator) };
            if handle.is_null() {
                break;
            }
            let frame = unsafe { *Box::from_raw(handle as *mut ConFrame) };
            frames.push(frame);
        }
        frames
    }

    #[test]
    fn file_iterator_matches_read_all_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = format!("{}{}", cu_frame(4, 1.0, 12.0), cu_frame(4, 2.0, 14.0));
        let path = write_con(dir.path(), "two.con", &text);
        let expected = read_all_frames(&path).expect("read_all_frames");
        let c_path = CString::new(path.to_str().expect("utf8")).expect("cstring");
        let iterator = unsafe { read_con_file_iterator(c_path.as_ptr()) };
        assert!(!iterator.is_null());
        let got = take_frames(iterator);
        unsafe { free_con_frame_iterator(iterator) };
        assert_eq!(got.len(), expected.len());
        for (left, right) in got.iter().zip(expected.iter()) {
            assert_same(left, right);
        }
        let again = unsafe { read_con_file_iterator(c_path.as_ptr()) };
        assert!(!again.is_null());
        let drained = take_frames(again);
        assert_eq!(drained.len(), expected.len());
        let status = unsafe { con_frame_iterator_forward(again) };
        assert_eq!(status, RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS);
        let extra = unsafe { con_frame_iterator_next(again) };
        assert!(extra.is_null());
        unsafe { free_con_frame_iterator(again) };
    }

    #[test]
    fn tiny_buffer_splits_frames_and_stays_below_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = format!(
            "{}{}{}",
            cu_frame(3, 0.0, 10.0),
            cu_frame(3, 5.0, 11.0),
            cu_frame(3, 9.0, 12.0)
        );
        let path = write_con(dir.path(), "split.con", &text);
        let file_len = std::fs::metadata(&path).expect("meta").len() as usize;
        let expected = read_all_frames(&path).expect("read_all_frames");
        let iterator = open_file_iterator_sized(&path, 8);
        assert!(!iterator.is_null());
        let got = take_frames(iterator);
        unsafe { free_con_frame_iterator(iterator) };
        assert_eq!(got.len(), expected.len());
        for (left, right) in got.iter().zip(expected.iter()) {
            assert_same(left, right);
        }
        let mut cursor = Cursor::open_file(&path, 8).expect("cursor");
        let first = cursor.next_frame().expect("frame").expect("parse");
        assert_eq!(first.atom_data.len(), 3);
        let capacity = cursor.buffer_capacity().expect("file cursor");
        assert!(capacity > 0);
        assert!(capacity < file_len, "capacity {capacity} file {file_len}");
    }

    #[test]
    fn truncated_last_frame_is_an_error_after_the_good_frame() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = cu_frame(2, 1.0, 10.0);
        let mut broken = format!("{good}{}", cu_frame(2, 3.0, 10.0));
        broken.truncate(broken.len() - 12);
        let path = write_con(dir.path(), "cut.con", &broken);
        let only = write_con(dir.path(), "only.con", &good);
        let expected = read_all_frames(&only).expect("good frame");
        assert_eq!(expected.len(), 1);
        let iterator = open_file_iterator_sized(&path, 64);
        assert!(!iterator.is_null());
        let first = unsafe { con_frame_iterator_next(iterator) };
        assert!(!first.is_null());
        let frame = unsafe { *Box::from_raw(first as *mut ConFrame) };
        assert_same(&frame, &expected[0]);
        let second = unsafe { con_frame_iterator_next(iterator) };
        assert!(second.is_null());
        unsafe { free_con_frame_iterator(iterator) };
        let partial = write_con(
            dir.path(),
            "partial.con",
            "user\n{\"con_spec_version\":2}\n10.0\n",
        );
        let forward_it = open_file_iterator_sized(&partial, 4);
        assert!(!forward_it.is_null());
        let failed = unsafe { con_frame_iterator_forward(forward_it) };
        assert_eq!(failed, RKRStatus::RKR_STATUS_IO_ERROR);
        let after = unsafe { con_frame_iterator_next(forward_it) };
        assert!(after.is_null());
        unsafe { free_con_frame_iterator(forward_it) };
        let skip_it = open_file_iterator_sized(&partial, 4);
        assert!(!skip_it.is_null());
        let skipped = unsafe { con_frame_iterator_skip(skip_it, 1) };
        assert_eq!(skipped, usize::MAX);
        unsafe { free_con_frame_iterator(skip_it) };
        let nth_it = open_file_iterator_sized(&partial, 4);
        assert!(!nth_it.is_null());
        let nth = unsafe { con_frame_iterator_nth(nth_it, 0) };
        assert!(nth.is_null());
        unsafe { free_con_frame_iterator(nth_it) };
    }

    #[test]
    fn null_and_invalid_handles_do_not_crash() {
        let null = ptr::null_mut();
        let missing = unsafe { con_frame_iterator_next(null) };
        assert!(missing.is_null());
        let status = unsafe { con_frame_iterator_forward(null) };
        assert_eq!(status, RKRStatus::RKR_STATUS_NULL_POINTER);
        let skipped = unsafe { con_frame_iterator_skip(null, 1) };
        assert_eq!(skipped, usize::MAX);
        let nth = unsafe { con_frame_iterator_nth(null, 0) };
        assert!(nth.is_null());
        unsafe { free_con_frame_iterator(null) };
        let name = unsafe { read_con_file_iterator(ptr::null()) };
        assert!(name.is_null());
        let bad_name = b"\xff.con\0";
        let bad_path = unsafe { read_con_file_iterator(bad_name.as_ptr().cast()) };
        assert!(bad_path.is_null());
        let missing = CString::new("no-such-readcon-iterator.con").expect("cstring");
        let missing_it = unsafe { read_con_file_iterator(missing.as_ptr()) };
        assert!(missing_it.is_null());
        let bad_text = b"\xff\0";
        let bad_string = unsafe { crate::ffi::read_con_string_iterator(bad_text.as_ptr().cast()) };
        assert!(bad_string.is_null());
        let empty_string = unsafe { crate::ffi::read_con_string_iterator(ptr::null()) };
        assert!(empty_string.is_null());
        let bad_buf = unsafe { crate::ffi::read_con_buffer_iterator(bad_text.as_ptr(), 1) };
        assert!(bad_buf.is_null());
        let null_buf = unsafe { crate::ffi::read_con_buffer_iterator(ptr::null(), 1) };
        assert!(null_buf.is_null());
        let empty_buf = unsafe { crate::ffi::read_con_buffer_iterator(ptr::null(), 0) };
        assert!(!empty_buf.is_null());
        let empty_next = unsafe { con_frame_iterator_next(empty_buf) };
        assert!(empty_next.is_null());
        unsafe { free_con_frame_iterator(empty_buf) };
        let invalid = invalid_c_iterator();
        assert!(!invalid.is_null());
        let bad_next = unsafe { con_frame_iterator_next(invalid) };
        assert!(bad_next.is_null());
        let bad_forward = unsafe { con_frame_iterator_forward(invalid) };
        assert_eq!(bad_forward, RKRStatus::RKR_STATUS_NULL_POINTER);
        let bad_skip = unsafe { con_frame_iterator_skip(invalid, 2) };
        assert_eq!(bad_skip, usize::MAX);
        let bad_nth = unsafe { con_frame_iterator_nth(invalid, 0) };
        assert!(bad_nth.is_null());
        unsafe { free_con_frame_iterator(invalid) };
    }

    #[test]
    fn nth_and_skip_match_read_all_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = format!(
            "{}{}{}",
            cu_frame(2, 0.0, 8.0),
            cu_frame(2, 1.0, 9.0),
            cu_frame(2, 2.0, 10.0)
        );
        let path = write_con(dir.path(), "three.con", &text);
        let expected = read_all_frames(&path).expect("read_all_frames");
        let iterator = open_file_iterator_sized(&path, 32);
        assert!(!iterator.is_null());
        let skipped = unsafe { con_frame_iterator_skip(iterator, 1) };
        assert_eq!(skipped, 1);
        let handle = unsafe { con_frame_iterator_next(iterator) };
        assert!(!handle.is_null());
        let frame = unsafe { *Box::from_raw(handle as *mut ConFrame) };
        assert_same(&frame, &expected[1]);
        unsafe { free_con_frame_iterator(iterator) };
        let nth_it = open_file_iterator_sized(&path, 32);
        assert!(!nth_it.is_null());
        let nth = unsafe { con_frame_iterator_nth(nth_it, 2) };
        assert!(!nth.is_null());
        let third = unsafe { *Box::from_raw(nth as *mut ConFrame) };
        assert_same(&third, &expected[2]);
        unsafe { free_con_frame_iterator(nth_it) };
        let past_it = open_file_iterator_sized(&path, 32);
        assert!(!past_it.is_null());
        let past = unsafe { con_frame_iterator_nth(past_it, 5) };
        assert!(past.is_null());
        unsafe { free_con_frame_iterator(past_it) };
    }

    #[test]
    fn text_iterator_forward_and_null_drop() {
        let text = cu_frame(1, 0.0, 10.0);
        let c_text = CString::new(text).expect("cstring");
        let iterator = unsafe { crate::ffi::read_con_string_iterator(c_text.as_ptr()) };
        assert!(!iterator.is_null());
        let status = unsafe { con_frame_iterator_forward(iterator) };
        assert_eq!(status, RKRStatus::RKR_STATUS_SUCCESS);
        let eof = unsafe { con_frame_iterator_forward(iterator) };
        assert_eq!(eof, RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS);
        unsafe { free_con_frame_iterator(iterator) };
        let bad = CString::new("user\n{\"con_spec_version\":2}\n10.0\n").expect("cstring");
        let broken = unsafe { crate::ffi::read_con_string_iterator(bad.as_ptr()) };
        assert!(!broken.is_null());
        let failed = unsafe { con_frame_iterator_forward(broken) };
        assert_eq!(failed, RKRStatus::RKR_STATUS_IO_ERROR);
        unsafe { free_con_frame_iterator(broken) };
        drop(TextCursor {
            contents: std::ptr::null_mut(),
            iter: std::ptr::null_mut(),
        });
    }

    #[test]
    fn text_cursor_and_skip_error_are_reported() {
        let mut empty = Cursor::from_text(String::new());
        assert!(empty.buffer_capacity().is_none());
        let skipped = empty.skip(3).expect("skip empty");
        assert_eq!(skipped, 0);
        let missing = Cursor::open_file(Path::new("no-such-readcon-iterator.con"), 8);
        assert!(missing.is_err());
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_con(
            dir.path(),
            "partial.con",
            "user\n{\"con_spec_version\":2}\n10.0\n",
        );
        let mut cursor = Cursor::open_file(&path, 4).expect("open partial");
        let failed = cursor.skip(1);
        assert!(failed.is_err());
        let text = Cursor::from_text("user\n{\"con_spec_version\":2}\n10.0\n".to_string());
        assert!(text.buffer_capacity().is_none());
        let mut bad = Cursor::from_text("user\n{\"con_spec_version\":2}\n10.0\n".to_string());
        let frame = bad.next_frame().expect("error result");
        assert!(frame.is_err());
    }
}
