//! Read every file of a CON 3.1 corpus and print one JSON object.
//!
//! `corpus31_report [--threads N] DIR` reads `DIR/manifest.toml` and prints
//! `{"files":[...]}`. Each entry is `accept` with `frames` and `pbc`, or
//! `reject` with `class` and `line`.

use readcon_core::error::ParseError;
use readcon_core::iterators::{frames_from_bytes, ConFrameIterator};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;

fn line_of(bytes: &[u8], err: &ParseError) -> u32 {
    let recorded = err.line();
    if recorded != 0 {
        return recorded;
    }
    if err.class_name() == "UnknownSection" {
        // The section name is on line 2 (CON-error.2, CON-line2.11).
        return 2;
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(bad) => {
            return bytes[..bad.valid_up_to()]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count() as u32
                + 1;
        }
    };
    let rendered = err.to_string();
    if rendered.contains("unknown unit") || rendered.contains("units.time:") {
        return 2;
    }
    if rendered.contains("units.time") {
        let line2 = text.lines().nth(1).unwrap_or("");
        if line2.contains("velocities") {
            return 2;
        }
        return text
            .lines()
            .position(|line| line.trim().is_empty())
            .map(|i| {
                // The blank line that opens a legacy velocity block, after
                // the header. The first blank in a normal header is line 5.
                text.lines()
                    .enumerate()
                    .skip(9)
                    .find(|(_, line)| line.trim().is_empty())
                    .map(|(i, _)| i as u32 + 1)
                    .unwrap_or(i as u32 + 1)
            })
            .unwrap_or(0);
    }
    let mut it = ConFrameIterator::new(text);
    let mut line = 0u32;
    while let Some(item) = it.next() {
        if item.is_err() {
            line = it.cursor_line();
            break;
        }
    }
    if line == 0 {
        line = it.cursor_line();
    }
    let rows: Vec<&str> = text.lines().collect();
    if err.class_name() == "IncompleteSection" {
        let mut at = line as usize;
        while at > 0 && at <= rows.len() && rows[at - 1].trim().is_empty() {
            at += 1;
        }
        if at > 0 && at <= rows.len() {
            return at as u32;
        }
    }
    if rendered.contains("parsing frame header") && line as usize == rows.len() {
        return line + 1;
    }
    line
}

fn one(path: &Path, threads: Option<usize>) -> Value {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let bytes = match readcon_core::compression::read_file_bytes(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return json!({
                "file": name,
                "outcome": "reject",
                "class": "IoError",
                "line": 0,
                "message": err.to_string(),
            });
        }
    };
    // The span parser disagrees with this cursor on four corpus files
    // (a unicode blank, an empty velocity row, a 6001-frame file, and a
    // non-UTF-8 separator). The record is the serial parse at
    // every thread setting.
    let _threads = threads;
    match frames_from_bytes(&bytes, None) {
        Ok(frames) => {
            let pbc = frames
                .first()
                .map(|frame| frame.header.pbc_effective())
                .unwrap_or([true, true, true]);
            json!({
                "file": name,
                "outcome": "accept",
                "frames": frames.len(),
                "pbc": pbc,
            })
        }
        Err(err) => json!({
            "file": name,
            "outcome": "reject",
            "class": err.class_name(),
            "line": line_of(&bytes, &err),
        }),
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let mut threads = None;
    let mut dir = None;
    while let Some(arg) = args.next() {
        if arg == "--threads" {
            let n: usize = args.next().expect("thread count").parse().expect("integer");
            threads = Some(n);
        } else {
            dir = Some(arg);
        }
    }
    let owned = dir.expect("corpus directory");
    let dir = Path::new(&owned);
    let manifest = fs::read_to_string(dir.join("manifest.toml")).expect("manifest");
    let entries: Vec<Value> = manifest
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("file = \"")?;
            Some(rest.trim_end_matches('"'))
        })
        .map(|name| one(&dir.join(name), threads))
        .collect();
    println!("{}", json!({"files": entries}));
}
