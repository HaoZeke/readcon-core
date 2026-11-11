// Copyright (c) readcon-core developers
//! Sequential extended-XYZ scan.
//!
//! One byte buffer, fixed token walks, and plain arrays. The comment
//! line supplies the lattice and the column layout.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct XyzColumn {
    pub name: String,
    pub kind: u8,
    pub width: usize,
    /// Text values for an `S` column. Empty for a numeric column.
    pub text: Vec<String>,
    /// Numeric values for an `R`, `I`, or `L` column, in column-major
    /// order within each atom. Empty for a text column.
    pub real: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct XyzTables {
    pub n_frames: usize,
    pub n_atoms: usize,
    pub positions: Vec<f64>,
    pub masses: Vec<f64>,
    pub cell: Vec<f64>,
    /// One symbol per atom, frame by frame. Empty strings when the file
    /// has no `species` column.
    pub species: Vec<String>,
    /// Every per-atom column named by `Properties`, including species,
    /// positions, masses and forces.
    pub columns: Vec<XyzColumn>,
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

struct ColumnSpec {
    name: String,
    kind: u8,
    width: usize,
    offset: usize,
}

fn property_columns(properties: &[u8]) -> Result<Vec<ColumnSpec>, String> {
    let mut offset = 0;
    let mut columns = Vec::new();
    let mut rest = properties;
    let mut saw_pos = false;
    while !rest.is_empty() {
        let name_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .ok_or("properties")?;
        let name = std::str::from_utf8(&rest[..name_end]).map_err(|_| "properties")?;
        rest = &rest[name_end + 1..];
        let kind_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .ok_or("properties")?;
        if kind_end != 1 {
            return Err("properties".to_string());
        }
        let kind = rest[0];
        if !matches!(kind, b'S' | b'R' | b'I' | b'L') {
            return Err("properties".to_string());
        }
        rest = &rest[kind_end + 1..];
        let count_end = rest
            .iter()
            .position(|byte| *byte == b':')
            .unwrap_or(rest.len());
        let count = parse_f64(&rest[..count_end])? as usize;
        if count == 0 {
            return Err("properties".to_string());
        }
        if name == "pos" {
            saw_pos = true;
            if count != 3 {
                return Err("pos".to_string());
            }
        }
        columns.push(ColumnSpec {
            name: name.to_string(),
            kind,
            width: count,
            offset,
        });
        offset += count;
        if count_end == rest.len() {
            break;
        }
        rest = &rest[count_end + 1..];
    }
    if !saw_pos {
        return Err("pos".to_string());
    }
    Ok(columns)
}

fn logical_value(token: &[u8]) -> f64 {
    match token {
        b"T" | b"t" | b"1" | b"true" | b"True" | b"TRUE" => 1.0,
        _ => 0.0,
    }
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

fn tokens(line: &[u8]) -> Vec<&[u8]> {
    let mut found = Vec::new();
    let mut index = 0;
    while let Some((start, end)) = next_token(line, index) {
        found.push(&line[start..end]);
        index = end;
    }
    found
}

fn push_atom(line: &[u8], specs: &[ColumnSpec], columns: &mut [XyzColumn]) -> Result<(), String> {
    let words = tokens(line);
    let need = specs
        .last()
        .map(|spec| spec.offset + spec.width)
        .unwrap_or(0);
    if words.len() < need {
        return Err("atom".to_string());
    }
    for (spec, column) in specs.iter().zip(columns.iter_mut()) {
        for slot in 0..spec.width {
            let token = words[spec.offset + slot];
            match spec.kind {
                b'S' => column
                    .text
                    .push(String::from_utf8_lossy(token).into_owned()),
                b'L' => column.real.push(logical_value(token)),
                _ => column.real.push(parse_f64(token)?),
            }
        }
    }
    Ok(())
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
    let mut cell = Vec::new();
    let mut columns: Vec<XyzColumn> = Vec::new();
    let mut specs: Vec<ColumnSpec> = Vec::new();
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
        let frame_specs = property_columns(properties)?;
        let frame_cell = lattice_cell(lattice)?;
        if n_frames == 0 {
            columns = frame_specs
                .iter()
                .map(|spec| XyzColumn {
                    name: spec.name.clone(),
                    kind: spec.kind,
                    width: spec.width,
                    text: Vec::new(),
                    real: Vec::new(),
                })
                .collect();
            specs = frame_specs;
            n_atoms = count;
        } else if count != n_atoms
            || frame_specs.len() != specs.len()
            || frame_specs.iter().zip(specs.iter()).any(|(left, right)| {
                left.name != right.name || left.kind != right.kind || left.width != right.width
            })
        {
            return Err("properties".to_string());
        }
        mark += 2;
        if mark + count > lines.len() {
            return Err("atoms".to_string());
        }
        for line in &lines[mark..mark + count] {
            push_atom(line, &specs, &mut columns)?;
        }
        cell.extend_from_slice(&frame_cell);
        mark += count;
        n_frames += 1;
    }
    if n_frames == 0 {
        return Err("no frames".to_string());
    }
    let mut positions = Vec::new();
    let mut masses = vec![f64::NAN; n_frames * n_atoms];
    let mut species = vec![String::new(); n_frames * n_atoms];
    for column in &columns {
        if column.name == "pos" {
            positions.clone_from(&column.real);
        } else if column.name == "masses" || column.name == "mass" {
            if column.width == 1 {
                masses.clone_from(&column.real);
            }
        } else if column.name == "species" && column.width == 1 {
            species.clone_from(&column.text);
        }
    }
    if positions.len() != n_frames * n_atoms * 3 {
        return Err("pos".to_string());
    }
    Ok(XyzTables {
        n_frames,
        n_atoms,
        positions,
        masses,
        cell,
        species,
        columns,
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
        assert!(property_columns(b"pos").is_err());
        let columns = property_columns(b"species:S:1:pos:R:3:masses:R:1").unwrap();
        assert_eq!(columns[0].name, "species");
        assert_eq!(columns[1].offset, 1);
        assert_eq!(columns[2].offset, 4);
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
        assert_eq!(tables.species, vec!["Cu".to_string(), "Cu".to_string()]);
        assert!(tables.columns.iter().any(|column| column.name == "species"));
        assert!((tables.cell[0] - 10.0).abs() < 1e-12);
        assert!((tables.cell[3] - 90.0).abs() < 1e-8);
    }

    #[test]
    fn forces_and_species_round_trip_from_the_small_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bench/h2h/fixtures/small-1.extxyz");
        let tables = tables_from_extxyz_path(&path).unwrap();
        assert_eq!(tables.n_frames, 1);
        assert_eq!(tables.n_atoms, 10);
        assert!(tables.species.iter().all(|symbol| symbol == "Cu"));
        let forces = tables
            .columns
            .iter()
            .find(|column| column.name == "forces")
            .unwrap();
        assert_eq!(forces.width, 3);
        assert!((forces.real[3] - 0.25).abs() < 1e-12);
        assert!((forces.real[4] - 0.5).abs() < 1e-12);
        assert!((forces.real[5] - 1.0).abs() < 1e-12);
        let names: Vec<_> = tables
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();
        assert_eq!(names, vec!["species", "pos", "masses", "forces"]);
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

        let no_newline =
            b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nH 0 1 2";
        let one = tables_from_extxyz_bytes(no_newline).unwrap();
        assert_eq!(one.positions, vec![0.0, 1.0, 2.0]);
        assert!(one.masses[0].is_nan());

        assert_eq!(tables_from_extxyz_bytes(b"\n\n").unwrap_err(), "no frames");
        assert_eq!(tables_from_extxyz_bytes(b"1\n").unwrap_err(), "comment");
        assert_eq!(
            tables_from_extxyz_bytes(
                b"2\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos:R:3\nH 0 0 0\n"
            )
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
            tables_from_extxyz_bytes(b"1\nLattice=\"1 0 0\" Properties=pos:R:3\nH 0 0 0\n")
                .unwrap_err(),
            "lattice"
        );
        assert_eq!(
            tables_from_extxyz_bytes(b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=pos\nH 0 0 0\n")
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
            tables_from_extxyz_bytes(b"1\nLattice=\"1 0 0 Properties=pos:R:3\nH 0 0 0\n")
                .unwrap_err(),
            "lattice"
        );
        assert_eq!(
            tables_from_extxyz_bytes(b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=\nH 0 0 0\n")
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
