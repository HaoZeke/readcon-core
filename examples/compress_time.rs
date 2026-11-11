// Copyright (c) readcon-core developers
//! Time compressed CON reads and writes.
//!
//! Codecs come from `detect_compression_from_extension`. `serve` keeps one
//! process warm: a write loads the source outside the clock.

use readcon_core::compression::{Compression, detect_compression_from_extension};
use readcon_core::iterators::read_all_frames;
use readcon_core::types::ConFrame;
use readcon_core::writer::ConFrameWriter;
use std::env;
use std::ffi::CString;
use std::hint::black_box;
use std::io::{self, BufRead, Write};
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

fn codec_name(codec: Compression) -> Option<&'static str> {
    match codec {
        Compression::Gzip => Some("gzip"),
        Compression::Zstd => Some("zstd"),
        Compression::None => None,
    }
}

fn codecs() {
    let extensions = ["gz", "zst", "bz2", "xz", "lz4", "br", "lzma", "Z"];
    let mut found = Vec::new();
    for extension in extensions {
        let name_on_disk = format!("a.con.{extension}");
        let path = Path::new(&name_on_disk);
        if let Some(name) = codec_name(detect_compression_from_extension(path))
            && !found.contains(&name)
        {
            found.push(name);
        }
    }
    print!("{{\"codecs\":[");
    for (index, name) in found.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!("\"{name}\"");
    }
    println!("]}}");
}

fn write_frames(path: &Path, frames: &[ConFrame]) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    match detect_compression_from_extension(path) {
        Compression::Gzip => {
            let mut writer = ConFrameWriter::from_path_gzip(path)?;
            writer.extend(frames.iter())?;
            Ok(())
        }
        Compression::Zstd => {
            #[cfg(feature = "zstd")]
            {
                let mut writer = ConFrameWriter::from_path_zstd(path)?;
                writer.extend(frames.iter())?;
                Ok(())
            }
            #[cfg(not(feature = "zstd"))]
            {
                let _ = frames;
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "zstd compression requires readcon-core built with the zstd feature",
                ))
            }
        }
        Compression::None => {
            let mut writer = ConFrameWriter::from_path(path)?;
            writer.extend(frames.iter())?;
            Ok(())
        }
    }
}

fn read_timed(path: &str, repeats: usize) {
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
    print_samples(&samples, last.len(), hwm_kib());
}

fn print_samples(samples: &[f64], n_frames: usize, hwm: u64) {
    print!("{{\"samples\":[");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!("{sample:.9}");
    }
    println!("],\"n_frames\":{n_frames},\"hwm_kib\":{hwm}}}");
}

fn reply(line: &str) {
    println!("{line}");
    let _ = io::stdout().flush();
}

fn serve() {
    let stdin = io::stdin();
    let mut loaded_src = String::new();
    let mut loaded: Option<Vec<ConFrame>> = None;
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
            "codecs" => {
                let extensions = ["gz", "zst", "bz2", "xz", "lz4", "br", "lzma", "Z"];
                let mut found = Vec::new();
                for extension in extensions {
                    let name_on_disk = format!("a.con.{extension}");
                    let path = Path::new(&name_on_disk);
                    if let Some(name) = codec_name(detect_compression_from_extension(path))
                        && !found.contains(&name)
                    {
                        found.push(name);
                    }
                }
                reply(&found.join(","));
            }
            "warmup-read" | "read" => {
                let path = parts.next().expect("path");
                let started = Instant::now();
                let frames = read_all_frames(Path::new(path)).expect("read");
                let elapsed = started.elapsed().as_secs_f64();
                black_box(&frames);
                if command == "read" {
                    reply(&format!("{elapsed:.9}"));
                } else {
                    reply("ok");
                }
            }
            "warmup-write" | "write" => {
                let src = parts.next().expect("src");
                let dest = parts.next().expect("dest");
                if loaded_src != src || loaded.is_none() {
                    loaded = Some(read_all_frames(Path::new(src)).expect("load"));
                    loaded_src = src.to_string();
                }
                let frames = loaded.as_ref().expect("frames");
                let started = Instant::now();
                write_frames(Path::new(dest), frames).expect("write");
                let elapsed = started.elapsed().as_secs_f64();
                black_box(frames);
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

fn write_measured(dest: &str, library: &str, marker: &str, frames: &[ConFrame]) -> i64 {
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
    write_frames(Path::new(dest), frames).expect("write");
    unsafe { stop() }
}

fn main() {
    let mut args = env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "codecs".to_string());
    match mode.as_str() {
        "codecs" => codecs(),
        "serve" => serve(),
        "read" => read_timed(
            &args.next().expect("path"),
            args.next().expect("repeats").parse().expect("repeats"),
        ),
        "once" => {
            let path = args.next().expect("path");
            let frames = read_all_frames(Path::new(&path)).expect("read");
            black_box(&frames);
            println!(
                "{{\"hwm_kib\":{},\"n_frames\":{},\"n_atoms\":{}}}",
                hwm_kib(),
                frames.len(),
                frames.first().map(|frame| frame.atom_data.len()).unwrap_or(0)
            );
        }
        "write-once" => {
            let src = args.next().expect("src");
            let dest = args.next().expect("dest");
            let library = args.next().expect("vgmark");
            let marker = args.next().expect("marker");
            let frames = read_all_frames(Path::new(&src)).expect("read");
            let delta = write_measured(&dest, &library, &marker, &frames);
            black_box(&frames);
            println!("{{\"rss_kib\":{delta}}}");
        }
        other => panic!("unknown mode {other}"),
    }
}
