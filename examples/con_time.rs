// Copyright (c) readcon-core developers
//! Time CON and convel reads and writes.
//!
//! `con_time time PATH REPEATS` warms up once, then prints one JSON object.
//! `con_time once PATH` reads once and prints the high-water mark.
//! `con_time write SRC OUT REPEATS` loads once, warms up one write, then times.
//! `con_time write-once SRC OUT LIBRARY MARKER` loads, then reports the
//! resident increase of one write.

use readcon_core::iterators::read_all_frames;
use readcon_core::writer::ConFrameWriter;
use std::env;
use std::ffi::CString;
use std::hint::black_box;
use std::os::raw::{c_char, c_int, c_long, c_void};
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
        "once" => {
            let frames = read_all_frames(Path::new(&path)).expect("read");
            black_box(&frames);
            println!(
                "{{\"hwm_kib\":{},\"n_frames\":{},\"n_atoms\":{}}}",
                hwm_kib(),
                frames.len(),
                frames.first().map(|frame| frame.atom_data.len()).unwrap_or(0)
            );
        }
        "write" => write_timed(
            &path,
            &args.next().expect("out"),
            args.next().expect("repeats").parse().expect("repeats"),
        ),
        "write-once" => {
            let out = args.next().expect("out");
            let library = args.next().expect("vgmark");
            let marker = args.next().expect("marker");
            let frames = read_all_frames(Path::new(&path)).expect("read");
            let delta = write_measured(&out, &library, &marker, &frames);
            black_box(&frames);
            println!("{{\"rss_kib\":{delta}}}");
        }
        other => panic!("unknown mode {other}"),
    }
}

fn time(path: &str, repeats: usize) {
    let warm = read_all_frames(Path::new(path)).expect("warmup");
    black_box(&warm);
    let mut samples = Vec::with_capacity(repeats);
    let mut last = warm;
    for _ in 0..repeats {
        let started = Instant::now();
        last = read_all_frames(Path::new(path)).expect("read");
        samples.push(started.elapsed().as_secs_f64());
    }
    black_box(&last);
    let sum: f64 = last
        .iter()
        .flat_map(|frame| frame.atom_data.iter())
        .map(|atom| atom.x + atom.y + atom.z)
        .sum();
    print!("{{\"samples\":[");
    for (i, sample) in samples.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("{sample:.9}");
    }
    println!(
        "],\"n_frames\":{},\"n_atoms\":{},\"sum_pos\":{sum:.17},\"hwm_kib\":{}}}",
        last.len(),
        last.first().map(|frame| frame.atom_data.len()).unwrap_or(0),
        hwm_kib()
    );
}

fn write_all(path: &str, frames: &[readcon_core::types::ConFrame]) {
    let mut writer = ConFrameWriter::from_path(path).expect("writer");
    writer.extend(frames.iter()).expect("write");
}

fn write_timed(src: &str, out: &str, repeats: usize) {
    let frames = read_all_frames(Path::new(src)).expect("read");
    write_all(out, &frames);
    let mut samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let started = Instant::now();
        write_all(out, &frames);
        samples.push(started.elapsed().as_secs_f64());
    }
    black_box(&frames);
    print!("{{\"samples\":[");
    for (i, sample) in samples.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("{sample:.9}");
    }
    println!("],\"hwm_kib\":{}}}", hwm_kib());
}

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

fn symbol<T>(handle: *mut c_void, name: &str) -> T {
    let c_name = CString::new(name).expect("symbol");
    let pointer = unsafe { dlsym(handle, c_name.as_ptr()) };
    if pointer.is_null() {
        panic!("symbol {name}");
    }
    unsafe { std::mem::transmute_copy(&pointer) }
}

fn write_measured(
    dest: &str,
    library: &str,
    marker: &str,
    frames: &[readcon_core::types::ConFrame],
) -> i64 {
    let path = CString::new(library).expect("library");
    let handle = unsafe { dlopen(path.as_ptr(), 2) };
    if handle.is_null() {
        panic!("vgmark");
    }
    let start: unsafe extern "C" fn() = symbol(handle, "watch_start");
    let stop: unsafe extern "C" fn() -> c_long = symbol(handle, "watch_delta_kib");
    let monitor: unsafe extern "C" fn(*const c_char) -> c_int = symbol(handle, "vg_monitor");
    let command = CString::new(format!("detailed_snapshot {marker}")).expect("marker");
    unsafe { start() };
    if unsafe { monitor(command.as_ptr()) } != 0 {
        panic!("snapshot");
    }
    write_all(dest, frames);
    unsafe { stop() }
}
