//! The readcon-core cells of the cited binary-format check.
//!
//! ```text
//! readcon_cited write-con REFERENCE BOX OUT roundtrip|DECIMALS
//! readcon_cited write-conbin CON OUT
//! readcon_cited time all|image THREADS PATH
//! readcon_cited dump all|image THREADS PATH POSITIONS BOX
//! ```
//!
//! `write-con` writes the frames of the reference and box files with the
//! library writer. `write-conbin` writes the CON-bin image of a CON file
//! with `write_from_con`. `all` reads the CON file with
//! `read_all_frames_with_threads`. `image` reads the position run of a
//! CON-bin file with `map_file`. The checksum is the first position plus
//! the last position of each frame, so a CON file and the image of that
//! file print the same checksum when the positions agree.

use std::error::Error;
use std::fs::File;
use std::hint::black_box;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;

use readcon_core::iterators::read_all_frames_with_threads;
use readcon_core::types::ConFrameBuilder;
use readcon_core::writer::ConFrameWriter;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

unsafe extern "C" {
    fn snprintf(buf: *mut i8, n: usize, fmt: *const i8, ...) -> i32;
}

/// Seventeen significant digits, the same text `printf("%.17g")` writes.
fn g17(value: f64) -> String {
    let mut buf = [0u8; 64];
    let n = unsafe { snprintf(buf.as_mut_ptr().cast(), buf.len(), c"%.17g".as_ptr(), value) };
    if n <= 0 {
        return format!("{value:.16e}");
    }
    let n = (n as usize).min(buf.len());
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

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
        self.checksum += positions[0] + positions[3 * (atoms - 1)];
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
}

fn read(mode: &str, threads: usize, path: &Path, fold: &mut Fold) -> Result<()> {
    if mode != "all" && threads != 1 {
        return Err(format!("{mode} takes no thread count").into());
    }
    match mode {
        "all" => {
            let frames = read_all_frames_with_threads(path, Some(threads))?;
            for frame in &frames {
                let positions = frame
                    .positions
                    .f64_slice()
                    .ok_or("positions are not binary64")?;
                fold.frame(positions, &frame.header.boxl)?;
            }
            black_box(&frames);
        }
        "image" => {
            let mapped = readcon_core::conbin::map_file(path, 1)?;
            let positions = mapped.positions_f64()?;
            let atoms = mapped.header.n_atoms as usize;
            let frames = mapped.header.n_frames as usize;
            let width = 3 * atoms;
            for index in 0..frames {
                fold.frame(&positions[index * width..(index + 1) * width], &[0.0; 3])?;
            }
            black_box(positions);
        }
        _ => return Err("mode is all or image".into()),
    }
    Ok(())
}

fn read_f64(path: &Path) -> Result<Vec<f64>> {
    let bytes = std::fs::read(path)?;
    Ok(bytes
        .chunks_exact(8)
        .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
        .collect())
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
        let mut builder = ConFrameBuilder::new(
            [lengths[3 * f], lengths[3 * f + 1], lengths[3 * f + 2]],
            [90.0; 3],
        );
        for i in 0..atoms {
            let (symbol, mass) = if i < atoms / 2 {
                ("Cu", 63.546)
            } else {
                ("H", 1.00794)
            };
            builder.add_atom(
                symbol,
                value(3 * i),
                value(3 * i + 1),
                value(3 * i + 2),
                [false; 3],
                i as u64,
                mass,
            );
        }
        writer.write_frame(&builder.build()?)?;
    }
    drop(writer);
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: readcon_cited write-con REFERENCE BOX OUT roundtrip|DECIMALS | \
                 write-conbin CON OUT | time MODE THREADS PATH | \
                 dump MODE THREADS PATH POSITIONS BOX";
    match args.get(1).map(String::as_str) {
        Some("write-con") if args.len() == 6 => write_con(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
            &args[5],
        ),
        Some("write-conbin") if args.len() == 4 => {
            readcon_core::conbin::write_from_con(Path::new(&args[2]), Path::new(&args[3]), false)?;
            Ok(())
        }
        Some(command @ ("time" | "dump"))
            if args.len() == if command == "dump" { 7 } else { 5 } =>
        {
            let threads: usize = args[3].parse()?;
            let mut fold = Fold::default();
            if command == "dump" {
                fold.dump = Some((
                    BufWriter::new(File::create(&args[5])?),
                    BufWriter::new(File::create(&args[6])?),
                ));
            }
            let start = Instant::now();
            read(&args[2], threads, Path::new(&args[4]), &mut fold)?;
            let seconds = start.elapsed().as_secs_f64();
            if let Some((mut pos, mut bx)) = fold.dump.take() {
                pos.flush()?;
                bx.flush()?;
            }
            println!(
                "seconds {}\nframes {}\natoms {}\nchecksum {:?}",
                g17(seconds),
                fold.frames,
                fold.atoms,
                fold.checksum
            );
            Ok(())
        }
        _ => Err(usage.into()),
    }
}
