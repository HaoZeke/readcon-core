// Copyright (c) readcon-core developers
//! Time `tables_from_extxyz_path` and, in check mode, dump the arrays it returns.
//!
//! `extxyz_time time PATH REPEATS` warms up once, then prints one JSON object.
//! `extxyz_time check PATH DUMP` writes the arrays from one read.
//! `extxyz_time once PATH` reads once and prints the high-water mark.

use readcon_core::extxyz_scan::tables_from_extxyz_path;
use std::env;
use std::fs::File;
use std::hint::black_box;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn hwm_kib() -> u64 {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            return rest
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
        }
    }
    0
}

fn main() {
    let mut args = env::args().skip(1);
    let mode = args.next().expect("mode");
    let path = args.next().expect("path");
    match mode.as_str() {
        "time" => time(
            &path,
            args.next().expect("repeats").parse().expect("repeats"),
        ),
        "check" => check(&path, &args.next().expect("dump")),
        "once" => {
            let tables = tables_from_extxyz_path(Path::new(&path)).expect("read");
            black_box(&tables);
            println!(
                "{{\"hwm_kib\":{},\"n_frames\":{},\"n_atoms\":{}}}",
                hwm_kib(),
                tables.n_frames,
                tables.n_atoms
            );
        }
        other => panic!("unknown mode {other}"),
    }
}

fn time(path: &str, repeats: usize) {
    let warm = tables_from_extxyz_path(Path::new(path)).expect("warmup");
    black_box(&warm);
    let mut samples = Vec::with_capacity(repeats);
    let mut last = warm;
    for _ in 0..repeats {
        let started = Instant::now();
        last = tables_from_extxyz_path(Path::new(path)).expect("read");
        let elapsed = started.elapsed().as_secs_f64();
        samples.push(elapsed);
    }
    black_box(&last);
    let sum_pos: f64 = last.positions.iter().sum();
    let sum_mass: f64 = last.masses.iter().copied().filter(|v| !v.is_nan()).sum();
    let cell = last
        .cell
        .iter()
        .take(6)
        .map(|v| format!("{v:.17}"))
        .collect::<Vec<_>>()
        .join(",");
    print!("{{\"samples\":[");
    for (i, sample) in samples.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("{sample:.9}");
    }
    println!(
        "],\"n_frames\":{},\"n_atoms\":{},\"sum_pos\":{sum_pos:.17},\"sum_mass\":{sum_mass:.17},\"cell0\":[{cell}],\"hwm_kib\":{}}}",
        last.n_frames,
        last.n_atoms,
        hwm_kib()
    );
}

fn check(path: &str, dump: &str) {
    let tables = tables_from_extxyz_path(Path::new(path)).expect("read");
    let mut out = File::create(dump).expect("dump");
    out.write_all(b"RCX1").unwrap();
    out.write_all(&(tables.n_frames as u32).to_le_bytes())
        .unwrap();
    out.write_all(&(tables.n_atoms as u32).to_le_bytes())
        .unwrap();
    for value in &tables.positions {
        out.write_all(&value.to_le_bytes()).unwrap();
    }
    for value in &tables.masses {
        out.write_all(&value.to_le_bytes()).unwrap();
    }
    for value in &tables.cell {
        out.write_all(&value.to_le_bytes()).unwrap();
    }
    let name = Path::new(dump)
        .file_name()
        .map(|item| item.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dump".to_string());
    println!(
        "{{\"n_frames\":{},\"n_atoms\":{},\"dump\":{name:?}}}",
        tables.n_frames, tables.n_atoms
    );
}
