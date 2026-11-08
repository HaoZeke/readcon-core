//! Read each file named on the command line with `read_all_frames` of readcon-core, and through the stream
//! that `open_decoded` gives, for `claimcheck/envelopes.py`.
//!
//! Each file gets one JSON line: the frame count and the sum of every coordinate of `read_all_frames`, and
//! the number of bytes `open_decoded` yields, or the error text of each.

use std::io::Read;
use std::path::Path;

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    for file in std::env::args().skip(1) {
        let path = Path::new(&file);
        let frames = match readcon_core::iterators::read_all_frames(path) {
            Ok(frames) => {
                let sum: f64 = frames
                    .iter()
                    .flat_map(|f| f.atom_data.iter())
                    .map(|a| a.x + a.y + a.z)
                    .sum();
                format!(
                    "{{\"frames\": {}, \"coordinate_sum\": {:?}}}",
                    frames.len(),
                    sum
                )
            }
            Err(e) => format!("{{\"error\": {}}}", quote(&e.to_string())),
        };
        let stream = match readcon_core::compression::open_decoded(path) {
            Ok(mut reader) => {
                let mut bytes = Vec::new();
                match reader.read_to_end(&mut bytes) {
                    Ok(n) => format!("{{\"bytes\": {n}}}"),
                    Err(e) => format!("{{\"error\": {}}}", quote(&e.to_string())),
                }
            }
            Err(e) => format!("{{\"error\": {}}}", quote(&e.to_string())),
        };
        println!(
            "{{\"file\": {}, \"read_all_frames\": {frames}, \"open_decoded\": {stream}}}",
            quote(&file)
        );
    }
}
