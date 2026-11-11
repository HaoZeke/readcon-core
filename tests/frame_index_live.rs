// Copyright (c) readcon developers
//! Sidecar checks through the library copy used by integration tests.

use std::fs;
use std::io;

use readcon_core::frame_index::{read_sidecar, sidecar_path, write_sidecar, FrameIndex};

#[test]
fn a_two_frame_sidecar_matches_the_scan_and_a_directory_errors() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("frames.con");
    let one = fs::read("resources/test/tiny_cuh2.con").unwrap();
    let bytes = [one.clone(), one].concat();
    fs::write(&data, &bytes).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let idx = FrameIndex::scan(text).unwrap();
    assert!(idx.len() >= 2, "{}", idx.len());
    assert_eq!(idx.offsets[0], 0);
    assert!(idx.offsets[1] > 0);
    assert_eq!(text.as_bytes()[idx.offsets[1] as usize - 1], b'\n');
    write_sidecar(&data, &idx).unwrap();
    let loaded = read_sidecar(&data).unwrap().unwrap();
    assert_eq!(loaded.offsets, idx.offsets);

    let missing = dir.path().join("absent.con");
    fs::write(&missing, &bytes).unwrap();
    assert!(read_sidecar(&missing).unwrap().is_none());

    let side = sidecar_path(&data);
    fs::remove_file(&side).unwrap();
    fs::create_dir(&side).unwrap();
    let err = read_sidecar(&data).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::IsADirectory);
    fs::remove_dir(&side).unwrap();

    let bytes_txt = text.as_bytes();
    let mut mid = idx.clone();
    let second = idx.offsets[1] as usize;
    assert_eq!(bytes_txt[second - 1], b'\n');
    mid.offsets[1] = (second + 1) as u64;
    write_sidecar(&data, &mid).unwrap();
    assert!(read_sidecar(&data).unwrap().is_none());

    let mut extra = bytes_txt.to_vec();
    extra.push(b'\n');
    fs::write(&data, &extra).unwrap();
    let text2 = std::str::from_utf8(&extra).unwrap();
    let scanned = FrameIndex::scan(text2).unwrap();
    let tail = extra
        .iter()
        .enumerate()
        .skip(1)
        .find(|(i, _)| extra[i - 1] == b'\n' && extra[*i..].iter().all(u8::is_ascii_whitespace))
        .map(|(i, _)| i as u64)
        .expect("a newline with only whitespace after it");
    assert!(tail < extra.len() as u64);
    let mut blank = scanned.clone();
    blank.offsets = vec![scanned.offsets[0], tail];
    write_sidecar(&data, &blank).unwrap();
    assert!(read_sidecar(&data).unwrap().is_none());
}
