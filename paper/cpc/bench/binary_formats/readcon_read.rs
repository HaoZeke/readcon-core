//! The readcon-core cells of the binary-format campaign and the CON and `.rcbin`
//! files they read.
//!
//! ```text
//! readcon_read write-con REFERENCE BOX OUT roundtrip|DECIMALS
//! readcon_read write-rcbin CON OUT
//! readcon_read time frames|all|columns|image THREADS PATH
//! readcon_read dump frames|all|columns|image THREADS PATH POSITIONS BOX
//! ```
//!
//! `write-con` writes the frames of `reference.f64` and `reference_box.f64` of
//! `gen.c` with the library writer, in shortest round-trip binary64 (its
//! default) or with DECIMALS decimals: atom `i` is Cu in the first half and H
//! in the second, with fixed flags 0 and id `i`, in a box with 90 degree
//! angles.
//! `write-rcbin` writes the positions image of a CON file: every frame's
//! binary64 coordinates and its three box lengths. This tree has no
//! `soa_traj` module, so the image format lives in this driver.
//!
//! The modes are the readers: `frames` iterates `ConFrameIterator` over the
//! `read_file_contents` of the file, one frame at a time; `all` is
//! `read_all_frames_with_threads`; `columns` keeps the coordinates and the
//! box lengths from one pass of the same iterator; `image` reads the file
//! `write-rcbin` wrote.  Only `all` takes a thread count.  `time` and `dump`
//! print and write what the native drivers of `bf_driver.h` do.

use std::error::Error;
use std::fs::File;
use std::hint::black_box;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;

use readcon_core::compression::read_file_contents;
use readcon_core::iterators::{read_all_frames_with_threads, ConFrameIterator};
use readcon_core::types::ConFrameBuilder;
use readcon_core::writer::ConFrameWriter;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Default)]
struct Fold {
    frames: usize,
    atoms: usize,
    checksum: f64,
    dump: Option<(BufWriter<File>, BufWriter<File>)>,
}

impl Fold {
    fn frame(&mut self, positions: &[f64], cell: &[f64]) -> Result<()> {
        let atoms = positions.len() / 3;
        if self.frames == 0 {
            self.atoms = atoms;
        }
        if atoms == 0 || atoms != self.atoms {
            return Err("the atom count changed between frames".into());
        }
        self.checksum += positions[0] + positions[3 * (atoms - 1)] + cell[0];
        self.frames += 1;
        if let Some((pos, bx)) = self.dump.as_mut() {
            for value in positions {
                pos.write_all(&value.to_le_bytes())?;
            }
            for value in &cell[..3] {
                bx.write_all(&value.to_le_bytes())?;
            }
        }
        Ok(())
    }

    fn columns(&mut self, positions: &[f64], boxes: &[f64], n_atoms: usize) -> Result<()> {
        if n_atoms == 0 || positions.len() % (3 * n_atoms) != 0 {
            return Err("the frames hold different atom counts".into());
        }
        let width = 3 * n_atoms;
        let n_frames = positions.len() / width;
        if boxes.len() != n_frames * 3 {
            return Err("the box count does not match the frames".into());
        }
        for f in 0..n_frames {
            self.frame(&positions[f * width..(f + 1) * width], &boxes[3 * f..3 * f + 3])?;
        }
        Ok(())
    }
}

fn read(mode: &str, threads: usize, path: &Path, fold: &mut Fold) -> Result<()> {
    if mode != "all" && threads != 1 {
        return Err(format!("{mode} takes no thread count").into());
    }
    match mode {
        "frames" => {
            let contents = read_file_contents(path)?;
            for frame in ConFrameIterator::from_bytes(contents.as_bytes()) {
                let frame = frame?;
                let positions = frame.positions.f64_slice().ok_or("positions are not binary64")?;
                fold.frame(positions, &frame.header.boxl)?;
                black_box(&frame);
            }
        }
        "all" => {
            let frames = read_all_frames_with_threads(path, Some(threads))?;
            for frame in &frames {
                let positions = frame.positions.f64_slice().ok_or("positions are not binary64")?;
                fold.frame(positions, &frame.header.boxl)?;
            }
            black_box(&frames);
        }
        "columns" => {
            // One pass. The position block does not carry the box, and a
            // second full read made this the longest one-thread cell.
            let contents = read_file_contents(path)?;
            let mut positions = Vec::new();
            let mut boxes = Vec::new();
            let mut n_atoms = 0usize;
            for frame in ConFrameIterator::from_bytes(contents.as_bytes()) {
                let frame = frame?;
                let row = frame.positions.f64_slice().ok_or("positions are not binary64")?;
                if n_atoms == 0 {
                    n_atoms = row.len() / 3;
                }
                positions.extend_from_slice(row);
                boxes.extend_from_slice(&frame.header.boxl);
            }
            fold.columns(&positions, &boxes, n_atoms)?;
            black_box((&positions, &boxes));
        }
        "image" => {
            let (positions, boxes, n_atoms) = read_image(path)?;
            fold.columns(&positions, &boxes, n_atoms)?;
            black_box((&positions, &boxes));
        }
        _ => return Err("mode is frames, all, columns or image".into()),
    }
    Ok(())
}

fn read_f64(path: &Path) -> Result<Vec<f64>> {
    let bytes = std::fs::read(path)?;
    Ok(bytes.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())).collect())
}

fn write_con(reference: &Path, boxes: &Path, out: &Path, format: &str) -> Result<()> {
    let lengths = read_f64(boxes)?;
    let frames = lengths.len() / 3;
    let mut file = File::open(reference)?;
    let atoms = file.metadata()?.len() as usize / (24 * frames);
    let mut writer = match format {
        "roundtrip" => ConFrameWriter::from_path(out)?,
        decimals => ConFrameWriter::from_path_with_precision(out, decimals.parse()?)?,
    };
    let mut bytes = vec![0u8; 24 * atoms];
    for f in 0..frames {
        file.read_exact(&mut bytes)?;
        let value = |k: usize| f64::from_le_bytes(bytes[8 * k..8 * k + 8].try_into().unwrap());
        let mut builder = ConFrameBuilder::new([lengths[3 * f], lengths[3 * f + 1], lengths[3 * f + 2]], [90.0; 3]);
        for i in 0..atoms {
            let (symbol, mass) = if i < atoms / 2 { ("Cu", 63.546) } else { ("H", 1.00794) };
            builder.add_atom(symbol, value(3 * i), value(3 * i + 1), value(3 * i + 2), [false; 3], i as u64, mass);
        }
        writer.write_frame(&builder.build()?)?;
    }
    writer.flush()?;
    Ok(())
}

fn write_image(con: &Path, out: &Path) -> Result<()> {
    let frames = read_all_frames_with_threads(con, Some(1))?;
    let n_atoms = frames.first().map(|frame| frame.positions.nrows()).unwrap_or(0);
    let mut file = File::create(out)?;
    file.write_all(&(frames.len() as u64).to_le_bytes())?;
    file.write_all(&(n_atoms as u64).to_le_bytes())?;
    for frame in &frames {
        let positions = frame.positions.f64_slice().ok_or("positions are not binary64")?;
        if positions.len() != n_atoms * 3 {
            return Err("the atom count changed between frames".into());
        }
        for value in positions {
            file.write_all(&value.to_le_bytes())?;
        }
        for value in &frame.header.boxl {
            file.write_all(&value.to_le_bytes())?;
        }
    }
    Ok(())
}

fn read_image(path: &Path) -> Result<(Vec<f64>, Vec<f64>, usize)> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 16 {
        return Err("the image is shorter than its header".into());
    }
    let n_frames = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
    let n_atoms = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    let per = n_atoms * 3 + 3;
    let need = 16 + n_frames * per * 8;
    if bytes.len() != need {
        return Err("the image length does not match its header".into());
    }
    let mut positions = Vec::with_capacity(n_frames * n_atoms * 3);
    let mut boxes = Vec::with_capacity(n_frames * 3);
    let mut offset = 16;
    for _ in 0..n_frames {
        for _ in 0..n_atoms * 3 {
            let value = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            positions.push(value);
            offset += 8;
        }
        for _ in 0..3 {
            let value = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            boxes.push(value);
            offset += 8;
        }
    }
    Ok((positions, boxes, n_atoms))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: readcon_read write-con REFERENCE BOX OUT roundtrip|DECIMALS | write-rcbin CON OUT | \
                 time MODE THREADS PATH | dump MODE THREADS PATH POSITIONS BOX";
    match args.get(1).map(String::as_str) {
        Some("write-con") if args.len() == 6 => {
            write_con(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), &args[5])
        }
        Some("write-rcbin") if args.len() == 4 => write_image(Path::new(&args[2]), Path::new(&args[3])),
        Some(command @ ("time" | "dump")) if args.len() == if command == "dump" { 7 } else { 5 } => {
            let threads: usize = args[3].parse()?;
            let mut fold = Fold::default();
            if command == "dump" {
                fold.dump = Some((BufWriter::new(File::create(&args[5])?), BufWriter::new(File::create(&args[6])?)));
            }
            let start = Instant::now();
            read(&args[2], threads, Path::new(&args[4]), &mut fold)?;
            let seconds = start.elapsed().as_secs_f64();
            if let Some((mut pos, mut bx)) = fold.dump.take() {
                pos.flush()?;
                bx.flush()?;
            }
            println!("seconds {seconds:.9}\nframes {}\natoms {}\nchecksum {:?}", fold.frames, fold.atoms, fold.checksum);
            Ok(())
        }
        _ => Err(usage.into()),
    }
}
