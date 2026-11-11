// Copyright (c) readcon-core developers
//! Time `tables_from_extxyz_path` and, in check mode, dump the arrays it returns.
//!
//! `extxyz_time time PATH REPEATS` warms up once, then prints one JSON object.
//! `extxyz_time check PATH DUMP` writes the arrays from one read.
//! `extxyz_time once PATH` reads once and prints the high-water mark.
//! `extxyz_time serve` reads line commands so one process can time single
//! reads and writes between other arms.

use readcon_core::extxyz_scan::{XyzTables, tables_from_extxyz_path, write_extxyz_path};
use std::env;
use std::fs::File;
use std::hint::black_box;
use std::io::{BufRead, Write};
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
    if mode == "serve" {
        serve();
        return;
    }
    let path = args.next().expect("path");
    match mode.as_str() {
        "time" => time(
            &path,
            args.next().expect("repeats").parse().expect("repeats"),
        ),
        "check" => check(&path, &args.next().expect("dump")),
        "write" => write_timed(
            &path,
            &args.next().expect("out"),
            args.next().expect("repeats").parse().expect("repeats"),
        ),
        "write-once" => {
            let tables = tables_from_extxyz_path(Path::new(&path)).expect("read");
            let out = args.next().expect("out");
            write_extxyz_path(Path::new(&out), &tables).expect("write");
            black_box(&tables);
            println!("{{\"hwm_kib\":{}}}", hwm_kib());
        }
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

fn reply(line: &str) {
    println!("{line}");
    let _ = std::io::stdout().flush();
}

/// One warm process. Each `read` or `write` line is one timed call.
///
/// `warmup-write SRC DST` reads `SRC` outside the clock. Later `write` lines
/// for that same `SRC` time only the write.
fn serve() {
    let stdin = std::io::stdin();
    let mut tables: Option<XyzTables> = None;
    let mut loaded = String::new();
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let command = parts.next().expect("command");
        match command {
            "exit" => {
                reply("ok");
                break;
            }
            "warmup-read" | "read" => {
                let path = parts.next().expect("path");
                let started = Instant::now();
                let parsed = tables_from_extxyz_path(Path::new(path)).expect("read");
                let elapsed = started.elapsed().as_secs_f64();
                black_box(&parsed);
                if command == "read" {
                    reply(&format!("{elapsed:.9}"));
                } else {
                    reply("ok");
                }
            }
            "warmup-write" | "write" => {
                let src = parts.next().expect("src");
                let dest = parts.next().expect("dest");
                if loaded != src || tables.is_none() {
                    tables = Some(tables_from_extxyz_path(Path::new(src)).expect("read"));
                    loaded = src.to_string();
                }
                let parsed = tables.as_ref().expect("tables");
                let started = Instant::now();
                write_extxyz_path(Path::new(dest), parsed).expect("write");
                let elapsed = started.elapsed().as_secs_f64();
                black_box(parsed);
                if command == "write" {
                    reply(&format!("{elapsed:.9}"));
                } else {
                    reply("ok");
                }
            }
            other => panic!("unknown serve command {other}"),
        }
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
    let sum_forces: f64 = last
        .columns
        .iter()
        .find(|column| column.name == "forces")
        .map(|column| column.real.iter().sum())
        .unwrap_or(0.0);
    let species_bytes: usize = last.species.iter().map(|symbol| symbol.len()).sum();
    black_box(sum_forces);
    black_box(species_bytes);
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
        "],\"n_frames\":{},\"n_atoms\":{},\"sum_pos\":{sum_pos:.17},\"sum_mass\":{sum_mass:.17},\"sum_forces\":{sum_forces:.17},\"species_bytes\":{species_bytes},\"cell0\":[{cell}],\"hwm_kib\":{}}}",
        last.n_frames,
        last.n_atoms,
        hwm_kib()
    );
}

fn write_timed(src: &str, out: &str, repeats: usize) {
    let tables = tables_from_extxyz_path(Path::new(src)).expect("read");
    let dest = Path::new(out);
    write_extxyz_path(dest, &tables).expect("warmup");
    let mut samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let started = Instant::now();
        write_extxyz_path(dest, &tables).expect("write");
        samples.push(started.elapsed().as_secs_f64());
    }
    black_box(&tables);
    print!("{{\"samples\":[");
    for (i, sample) in samples.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("{sample:.9}");
    }
    println!("],\"hwm_kib\":{}}}", hwm_kib());
}

fn check(path: &str, dump: &str) {
    let tables = tables_from_extxyz_path(Path::new(path)).expect("read");
    let mut out = File::create(dump).expect("dump");
    out.write_all(b"RCX2").unwrap();
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
    out.write_all(&(tables.species.len() as u32).to_le_bytes())
        .unwrap();
    for symbol in &tables.species {
        let bytes = symbol.as_bytes();
        out.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        out.write_all(bytes).unwrap();
    }
    let forces = tables.columns.iter().find(|column| column.name == "forces");
    out.write_all(&(u32::from(forces.is_some())).to_le_bytes())
        .unwrap();
    if let Some(column) = forces {
        for value in &column.real {
            out.write_all(&value.to_le_bytes()).unwrap();
        }
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
