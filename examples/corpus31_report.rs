// Copyright (c) readcon developers
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
            line = it.lines_read();
            break;
        }
    }
    if line == 0 {
        line = it.lines_read();
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
    // The read position is the last line that was present. A missing
    // coordinate row is the next line (CON-frame.4).
    if err.class_name() == "IncompleteFrame" && line as usize == rows.len() {
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
    match frames_from_bytes(&bytes, threads) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use readcon_core::error::ParseError;

    #[test]
    fn line_of_reports_the_class_and_the_bytes() {
        assert_eq!(line_of(b"a\n", &ParseError::MissingSpecVersion), 2);
        assert_eq!(line_of(b"a\n", &ParseError::UnknownSection("q".into())), 2);
        let bad = b"ab\xff";
        assert_eq!(line_of(bad, &ParseError::IncompleteHeader), 1);
        let units = b"header\nvelocities stay\n";
        assert_eq!(
            line_of(units, &ParseError::InvalidUnits("unknown unit furlong".into())),
            2
        );
        let timed = b"h\nvelocities stay\n\n";
        assert_eq!(
            line_of(timed, &ParseError::ValidationError("need units.time".into())),
            2
        );
        let blank = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n\nrest\n";
        let at = line_of(
            blank.as_bytes(),
            &ParseError::ValidationError("missing units.time".into()),
        );
        assert!(at >= 11);
        let section = b"\n\nbody\n";
        assert_eq!(
            line_of(section, &ParseError::IncompleteSection("forces".into())),
            3
        );
        let ended = b"only\n";
        assert_eq!(line_of(ended, &ParseError::IncompleteHeader), 2);
        assert_eq!(line_of(b"only", &ParseError::IncompleteFrame), 2);
        let walked = b"not a frame\nstill not\n";
        let at = line_of(walked, &ParseError::ValidationError("later".into()));
        assert!(at >= 1);
        let text = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
        let counted = line_of(&text, &ParseError::ValidationError("later".into()));
        assert!(counted >= 1);
        let blanks = b"row\n\n\n";
        let section = line_of(blanks, &ParseError::IncompleteSection("forces".into()));
        assert!(section >= 1);
    }

    #[test]
    fn one_rejects_a_missing_file_and_accepts_a_fixture() {
        let missing = one(Path::new("resources/test/no-such-file.con"), None);
        assert_eq!(missing["outcome"], "reject");
        assert_eq!(missing["class"], "IoError");
        let path = Path::new("resources/test/tiny_cuh2.con");
        let accepted = one(path, Some(1));
        assert_eq!(accepted["outcome"], "accept");
        assert!(accepted["frames"].as_u64().unwrap() >= 1);
    }
}
