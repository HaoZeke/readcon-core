// Copyright (c) readcon developers
//! Sequential extended-XYZ scan.
//!
//! One byte buffer, fixed token walks, and plain arrays. The comment
//! line supplies the lattice and the column layout.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct XyzTables {
    pub n_frames: usize,
    pub n_atoms: usize,
    pub positions: Vec<f64>,
    pub masses: Vec<f64>,
    pub cell: Vec<f64>,
}

fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn next_token(line: &[u8], mut index: usize) -> Option<(usize, usize)> {
    while index < line.len() && line[index].is_ascii_whitespace() {
        index += 1;
    }
    if index >= line.len() {
        return None;
    }
    let start = index;
    while index < line.len() && !line[index].is_ascii_whitespace() {
        index += 1;
    }
    Some((start, index))
}

fn parse_f64(bytes: &[u8]) -> Result<f64, String> {
    let (value, _consumed) =
        fast_float2::parse_partial::<f64, _>(bytes).map_err(|_| "float".to_string())?;
    // fast_float2 only returns Ok after consuming at least one digit.
    Ok(value)
}

fn quoted_after<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut index = 0;
    while index + key.len() <= line.len() {
        if &line[index..index + key.len()] == key {
            let mut mark = index + key.len();
            if mark < line.len() && line[mark] == b'"' {
                mark += 1;
                let start = mark;
                while mark < line.len() && line[mark] != b'"' {
                    mark += 1;
                }
                return Some(&line[start..mark]);
            }
            let start = mark;
            while mark < line.len() && !line[mark].is_ascii_whitespace() {
                mark += 1;
            }
            return Some(&line[start..mark]);
        }
        index += 1;
    }
    None
}

fn column_layout(properties: &[u8]) -> Result<(usize, Option<usize>), String> {
    let mut offset = 0;
    let mut pos = None;
    let mut mass = None;
    let mut rest = properties;
    while !rest.is_empty() {
        let name_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .ok_or("properties")?;
        let name = &rest[..name_end];
        rest = &rest[name_end + 1..];
        let kind_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .ok_or("properties")?;
        rest = &rest[kind_end + 1..];
        let count_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .unwrap_or(rest.len());
        let count = parse_f64(&rest[..count_end])? as usize;
        if name == b"pos" {
            pos = Some(offset);
        } else if name == b"masses" {
            mass = Some(offset);
        }
        offset += count;
        if count_end == rest.len() {
            break;
        }
        rest = &rest[count_end + 1..];
    }
    Ok((pos.ok_or("pos")?, mass))
}

fn lattice_cell(lattice: &[u8]) -> Result<[f64; 6], String> {
    let mut values = [0.0; 9];
    let mut rest = trim(lattice);
    for slot in values.iter_mut() {
        rest = trim(rest);
        let (value, consumed) =
            fast_float2::parse_partial::<f64, _>(rest).map_err(|_| "lattice".to_string())?;
        *slot = value;
        rest = rest.get(consumed..).unwrap_or(b"");
    }
    let length = |i: usize| {
        (values[i] * values[i] + values[i + 1] * values[i + 1] + values[i + 2] * values[i + 2])
            .sqrt()
    };
    let la = length(0);
    let lb = length(3);
    let lc = length(6);
    let dot = |i: usize, j: usize| {
        values[i] * values[j] + values[i + 1] * values[j + 1] + values[i + 2] * values[j + 2]
    };
    let angle = |cos_v: f64| cos_v.clamp(-1.0, 1.0).acos().to_degrees();
    let gamma = angle(dot(0, 3) / (la * lb));
    let beta = angle(dot(0, 6) / (la * lc));
    let alpha = angle(dot(3, 6) / (lb * lc));
    Ok([la, lb, lc, alpha, beta, gamma])
}

fn atom_values(
    line: &[u8],
    pos_col: usize,
    mass_col: Option<usize>,
) -> Result<(f64, f64, f64, f64), String> {
    let mut index = 0;
    let mut column = 0;
    let mut coords = [0.0; 3];
    let mut mass = f64::NAN;
    let mut got = 0;
    while let Some((start, end)) = next_token(line, index) {
        if column >= pos_col && column < pos_col + 3 {
            coords[column - pos_col] = parse_f64(&line[start..end])?;
            got += 1;
        } else if Some(column) == mass_col {
            mass = parse_f64(&line[start..end])?;
        }
        column += 1;
        index = end;
        if got == 3 && mass_col.map(|item| column > item).unwrap_or(true) {
            break;
        }
    }
    if got != 3 {
        return Err("atom".to_string());
    }
    Ok((coords[0], coords[1], coords[2], mass))
}

pub fn tables_from_extxyz_bytes(bytes: &[u8]) -> Result<XyzTables, String> {
    let mut lines = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && bytes[index] != b'\n' {
            index += 1;
        }
        let mut line = &bytes[start..index];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        lines.push(line);
        if index < bytes.len() {
            index += 1;
        }
    }
    let mut positions = Vec::new();
    let mut masses = Vec::new();
    let mut cell = Vec::new();
    let mut mark = 0;
    let mut n_atoms = 0;
    let mut n_frames = 0;
    while mark < lines.len() {
        let count_line = trim(lines[mark]);
        if count_line.is_empty() {
            mark += 1;
            continue;
        }
        let count = parse_f64(count_line)? as usize;
        if mark + 1 >= lines.len() {
            return Err("comment".to_string());
        }
        let comment = lines[mark + 1];
        let lattice = quoted_after(comment, b"Lattice=").ok_or("lattice")?;
        let properties = quoted_after(comment, b"Properties=").ok_or("properties")?;
        let (pos_col, mass_col) = column_layout(properties)?;
        let frame_cell = lattice_cell(lattice)?;
        mark += 2;
        if mark + count > lines.len() {
            return Err("atoms".to_string());
        }
        for line in &lines[mark..mark + count] {
            let (x, y, z, mass) = atom_values(line, pos_col, mass_col)?;
            positions.push(x);
            positions.push(y);
            positions.push(z);
            masses.push(mass);
        }
        cell.extend_from_slice(&frame_cell);
        mark += count;
        n_frames += 1;
        n_atoms = count;
    }
    if n_frames == 0 {
        return Err("no frames".to_string());
    }
    Ok(XyzTables {
        n_frames,
        n_atoms,
        positions,
        masses,
        cell,
    })
}

pub fn tables_from_extxyz_path(path: &Path) -> Result<XyzTables, String> {
    let bytes = fs::read(path).map_err(|_| "read".to_string())?;
    tables_from_extxyz_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_value_and_a_bare_value_both_return_the_bytes() {
        assert_eq!(quoted_after(b"Lattice=10 0", b"Lattice="), Some(&b"10"[..]));
        assert_eq!(
            quoted_after(b"Lattice=\"10 0\" tail", b"Lattice="),
            Some(&b"10 0"[..])
        );
        assert_eq!(quoted_after(b"no key", b"Lattice="), None);
        assert!(column_layout(b"pos").is_err());
        let (pos, mass) = column_layout(b"species:S:1:pos:R:3:masses:R:1").unwrap();
        assert_eq!(pos, 1);
        assert_eq!(mass, Some(4));
    }

    #[test]
    fn one_frame_keeps_lattice_position_and_mass() {
        let text = "\
2
Lattice=\"10 0 0 0 12 0 0 0 14\" Properties=species:S:1:pos:R:3:masses:R:1 pbc=\"T T T\"
Cu 1.0 2.0 3.0 63.546
Cu 4.0 5.0 6.0 63.546
";
        let tables = tables_from_extxyz_bytes(text.as_bytes()).unwrap();
        assert_eq!(tables.n_frames, 1);
        assert_eq!(tables.n_atoms, 2);
        assert_eq!(tables.positions, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(tables.masses, vec![63.546, 63.546]);
        assert!((tables.cell[0] - 10.0).abs() < 1e-12);
        assert!((tables.cell[3] - 90.0).abs() < 1e-8);
    }

    #[test]
    fn frames_cover_quotes_angles_blank_lines_and_errors() {
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos:R:3\nCu nope 0 0\n"
            )
            .unwrap_err(),
            "float"
        );
        let text = "\r\n2\r\nLattice=\"10 0 0 0 12 0 0 0 14\" Properties=species:S:1:pos:R:3:masses:R:1\r\nCu 1 2 3 63.546\r\nCu 4 5 6 63.546\r\n";
        let tables = tables_from_extxyz_bytes(text.as_bytes()).unwrap();
        assert_eq!(tables.n_frames, 1);
        assert_eq!(tables.n_atoms, 2);
        assert_eq!(tables.positions.len(), 6);
        assert_eq!(tables.masses[0], 63.546);
        assert!((tables.cell[0] - 10.0).abs() < 1e-9);

        let no_newline = b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nH 0 1 2";
        let one = tables_from_extxyz_bytes(no_newline).unwrap();
        assert_eq!(one.positions, vec![0.0, 1.0, 2.0]);
        assert!(one.masses[0].is_nan());

        assert_eq!(tables_from_extxyz_bytes(b"\n\n").unwrap_err(), "no frames");
        assert_eq!(tables_from_extxyz_bytes(b"1\n").unwrap_err(), "comment");
        assert_eq!(
            tables_from_extxyz_bytes(b"2\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos:R:3\nH 0 0 0\n")
                .unwrap_err(),
            "atoms"
        );
        assert_eq!(
            tables_from_extxyz_bytes(b"1\nnote\nH 0 0 0\n").unwrap_err(),
            "lattice"
        );
        assert_eq!(
            tables_from_extxyz_bytes(b"1\nLattice=\"1 0 0 0 1 0 0 0 1\"\nH 0 0 0\n").unwrap_err(),
            "properties"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=masses:R:1\nH 1\n"
            )
            .unwrap_err(),
            "pos"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0\" Properties=pos:R:3\nH 0 0 0\n"
            )
            .unwrap_err(),
            "lattice"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos\nH 0 0 0\n"
            )
            .unwrap_err(),
            "properties"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos:R:x\nH 0 0 0\n"
            )
            .unwrap_err(),
            "float"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos:R:3\nH 0 nope 2\n"
            )
            .unwrap_err(),
            "float"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"nope 0 0 0 1 0 0 0 1\" Properties=pos:R:3\nH 0 0 0\n"
            )
            .unwrap_err(),
            "lattice"
        );
        assert_eq!(tables_from_extxyz_bytes(b"abc\n").unwrap_err(), "float");
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 Properties=pos:R:3\nH 0 0 0\n"
            )
            .unwrap_err(),
            "lattice"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=\nH 0 0 0\n"
            )
            .unwrap_err(),
            "pos"
        );
        assert_eq!(
            tables_from_extxyz_bytes(
                b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\n\n"
            )
            .unwrap_err(),
            "atom"
        );

        let missing = tables_from_extxyz_path(std::path::Path::new("/no/such/extxyz")).unwrap_err();
        assert_eq!(missing, "read");
        let path = std::env::temp_dir().join(format!("readcon-extxyz-{}.xyz", std::process::id()));
        std::fs::write(&path, no_newline).unwrap();
        let from_path = tables_from_extxyz_path(&path).unwrap();
        assert_eq!(from_path.n_frames, 1);
        std::fs::remove_file(&path).unwrap();
    }
}
