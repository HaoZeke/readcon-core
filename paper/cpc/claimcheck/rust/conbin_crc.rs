//! Write and read a CON-bin image for the claim check of the binary image.
//!
//! `conbin_crc write CON OUT` writes `OUT` with `write_from_con` and prints
//! one JSON object: frame count, atom count, payload offset and the sum of
//! the positions `map_file` returns.
//! `conbin_crc read OUT` prints the same fields, or the error text.

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

fn report(path: &Path) -> String {
    match readcon_core::conbin::map_file(path, 1) {
        Ok(mapped) => {
            let sum: f64 = mapped
                .positions_f64()
                .expect("positions")
                .iter()
                .copied()
                .sum();
            format!(
                "{{\"ok\":true,\"n_frames\":{},\"n_atoms\":{},\"payload_off\":{},\"position_sum\":{:?}}}",
                mapped.header.n_frames, mapped.header.n_atoms, mapped.payload_off, sum
            )
        }
        Err(err) => format!("{{\"ok\":false,\"error\":{}}}", quote(&err.to_string())),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("write") if args.len() == 4 => {
            readcon_core::conbin::write_from_con(Path::new(&args[2]), Path::new(&args[3]), false)
                .expect("write the CON-bin image");
            println!("{}", report(Path::new(&args[3])));
        }
        Some("read") if args.len() == 3 => println!("{}", report(Path::new(&args[2]))),
        _ => {
            eprintln!("usage: conbin_crc write CON OUT | conbin_crc read OUT");
            std::process::exit(2);
        }
    }
}
