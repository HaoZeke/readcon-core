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
    /// Atomic numbers, one per atom, frame-major. The lookup is
    /// [`crate::helpers::symbol_to_atomic_number`].
    pub types: Vec<u64>,
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
    let (value, consumed) =
        fast_float2::parse_partial::<f64, _>(bytes).map_err(|_| "float".to_string())?;
    if consumed == 0 {
        return Err("float".to_string());
    }
    Ok(value)
}

fn quoted_after<'a>(line: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut index = 0;
    while index + key.len() <= line.len() {
        if &line[index..index + key.len()] == key {
            let mut cursor = index + key.len();
            if cursor < line.len() && line[cursor] == b'"' {
                cursor += 1;
                let start = cursor;
                while cursor < line.len() && line[cursor] != b'"' {
                    cursor += 1;
                }
                return Some(&line[start..cursor]);
            }
            let start = cursor;
            while cursor < line.len() && !line[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            return Some(&line[start..cursor]);
        }
        index += 1;
    }
    None
}

fn column_layout(properties: &[u8]) -> Result<(usize, Option<usize>, usize), String> {
    let mut offset = 0;
    let mut pos = None;
    let mut mass = None;
    let mut species = None;
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
        } else if name == b"species" {
            species = Some(offset);
        }
        offset += count;
        if count_end == rest.len() {
            break;
        }
        rest = &rest[count_end + 1..];
    }
    Ok((pos.ok_or("pos")?, mass, species.ok_or("species")?))
}

fn lattice_cell(lattice: &[u8]) -> Result<[f64; 6], String> {
    let mut values = [0.0; 9];
    let mut rest = trim(lattice);
    for slot in values.iter_mut() {
        rest = trim(rest);
        let (value, consumed) =
            fast_float2::parse_partial::<f64, _>(rest).map_err(|_| "lattice".to_string())?;
        if consumed == 0 {
            return Err("lattice".to_string());
        }
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

fn atomic_number_at(token: &[u8]) -> Result<u64, String> {
    let symbol = std::str::from_utf8(token).map_err(|_| "species".to_string())?;
    let atomic_number = crate::helpers::symbol_to_atomic_number(symbol);
    if atomic_number == 0 {
        return Err(format!("species {symbol}"));
    }
    Ok(atomic_number)
}

fn atom_values(
    line: &[u8],
    pos_col: usize,
    mass_col: Option<usize>,
    species_col: usize,
) -> Result<(f64, f64, f64, f64, u64), String> {
    let mut index = 0;
    let mut column = 0;
    let mut coords = [0.0; 3];
    let mut mass = f64::NAN;
    let mut got = 0;
    let mut species = None;
    let last_needed = species_col.max(pos_col + 2).max(mass_col.unwrap_or(0));
    while let Some((start, end)) = next_token(line, index) {
        if column == species_col {
            species = Some(atomic_number_at(&line[start..end])?);
        } else if column >= pos_col && column < pos_col + 3 {
            coords[column - pos_col] = parse_f64(&line[start..end])?;
            got += 1;
        } else if Some(column) == mass_col {
            mass = parse_f64(&line[start..end])?;
        }
        column += 1;
        index = end;
        if column > last_needed && got == 3 && species.is_some() {
            break;
        }
    }
    if got != 3 {
        return Err("atom".to_string());
    }
    let atomic_number = species.ok_or_else(|| "species".to_string())?;
    Ok((coords[0], coords[1], coords[2], mass, atomic_number))
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
    let mut types = Vec::new();
    let mut cursor = 0;
    let mut n_atoms = 0;
    let mut n_frames = 0;
    while cursor < lines.len() {
        let count_line = trim(lines[cursor]);
        if count_line.is_empty() {
            cursor += 1;
            continue;
        }
        let count = parse_f64(count_line)? as usize;
        if cursor + 1 >= lines.len() {
            return Err("comment".to_string());
        }
        let comment = lines[cursor + 1];
        let lattice = quoted_after(comment, b"Lattice=").ok_or("lattice")?;
        let properties = quoted_after(comment, b"Properties=").ok_or("properties")?;
        let (pos_col, mass_col, species_col) = column_layout(properties)?;
        let frame_cell = lattice_cell(lattice)?;
        cursor += 2;
        if cursor + count > lines.len() {
            return Err("atoms".to_string());
        }
        for line in &lines[cursor..cursor + count] {
            let (x, y, z, mass, atomic_number) = atom_values(line, pos_col, mass_col, species_col)?;
            positions.push(x);
            positions.push(y);
            positions.push(z);
            masses.push(mass);
            types.push(atomic_number);
        }
        cell.extend_from_slice(&frame_cell);
        cursor += count;
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
        types,
    })
}

pub fn tables_from_extxyz_path(path: &Path) -> Result<XyzTables, String> {
    let bytes = fs::read(path).map_err(|_| "read".to_string())?;
    tables_from_extxyz_bytes(&bytes)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn one_frame_keeps_lattice_position_and_mass() {
        let text = "\
2
Lattice=\"10 0 0 0 12 0 0 0 14\" Properties=species:S:1:pos:R:3:masses:R:1 pbc=\"T T T\"
Cu 1.0 2.0 3.0 63.546
Cu 4.0 5.0 6.0 63.546
";
        let tables = tables_from_extxyz_bytes(text.as_bytes()).expect("parse");
        assert_eq!(tables.n_frames, 1);
        assert_eq!(tables.n_atoms, 2);
        assert_eq!(tables.positions, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(tables.masses, vec![63.546, 63.546]);
        assert_eq!(tables.types, vec![29, 29]);
        assert!((tables.cell[0] - 10.0).abs() < 1e-12);
        assert!((tables.cell[3] - 90.0).abs() < 1e-8);
    }

    #[test]
    fn mixed_species_frames_keep_each_atomic_number() {
        let text = "\
2
Lattice=\"8 0 0 0 8 0 0 0 8\" Properties=species:S:1:pos:R:3
Cu 1.0 0.0 0.0
H 0.0 2.0 0.0
2
Lattice=\"9 0 0 0 9 0 0 0 9\" Properties=species:S:1:pos:R:3
O 0.0 0.0 3.0
Cu 4.0 0.0 0.0
";
        let tables = tables_from_extxyz_bytes(text.as_bytes()).expect("parse");
        assert_eq!(tables.n_frames, 2);
        assert_eq!(tables.n_atoms, 2);
        assert_eq!(tables.types, vec![29, 1, 8, 29]);
        assert_eq!(tables.positions[0], 1.0);
        assert_eq!(tables.positions[9], 4.0);
        assert!((tables.cell[0] - 8.0).abs() < 1e-12);
        assert!((tables.cell[6] - 9.0).abs() < 1e-12);
    }

    #[test]
    fn malformed_species_is_an_error() {
        let text = "\
1
Lattice=\"10 0 0 0 10 0 0 0 10\" Properties=species:S:1:pos:R:3
Nope 0.0 0.0 0.0
";
        let parsed = tables_from_extxyz_bytes(text.as_bytes());
        assert!(parsed.is_err());
        let message = parsed.expect_err("unknown species");
        assert!(message.contains("species"), "{message}");
    }

    #[test]
    fn missing_species_column_is_an_error() {
        let text = "\
1
Lattice=\"10 0 0 0 10 0 0 0 10\" Properties=pos:R:3
1.0 2.0 3.0
";
        let parsed = tables_from_extxyz_bytes(text.as_bytes());
        let message = parsed.expect_err("no species column");
        assert!(message.contains("species"), "{message}");
    }

    #[test]
    fn non_utf8_species_is_an_error() {
        let mut bytes = b"\
1
Lattice=\"10 0 0 0 10 0 0 0 10\" Properties=species:S:1:pos:R:3
"
        .to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b" 0.0 0.0 0.0\n");
        let parsed = tables_from_extxyz_bytes(&bytes);
        let message = parsed.expect_err("non-utf8 species");
        assert!(message.contains("species"), "{message}");
    }

    #[test]
    fn a_bad_float_and_a_missing_lattice_fail() {
        let bad = b"\
1
Lattice=\"nope 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3
Cu 0 0 0
";
        assert!(tables_from_extxyz_bytes(bad).is_err());
        let missing = b"\
1
Properties=species:S:1:pos:R:3
Cu 0 0 0
";
        assert!(tables_from_extxyz_bytes(missing).is_err());
        let blank = b"\
1
Lattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3

";
        assert!(tables_from_extxyz_bytes(blank).is_err());
    }

    #[test]
    fn malformed_frames_fail() {
        let cases: &[&[u8]] = &[
            b"",
            b"1\n",
            b"2\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nCu 0 0 0\n",
            b"1\nLattice=\"1 0 0\" Properties=species:S:1:pos:R:3\nCu 0 0 0\n",
            b"1\nLattice=\"1 0 Properties=species:S:1:pos:R:3\nCu 0 0 0\n",
            b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species\nCu 0 0 0\n",
            b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3:masses:R:x\nCu 0 0 0 1\n",
            b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nCu\n",
            b"not-a-count\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nCu 0 0 0\n",
        ];
        for (index, case) in cases.iter().enumerate() {
            assert!(
                tables_from_extxyz_bytes(case).is_err(),
                "case {index} parsed"
            );
        }
        let with_charge = b"\
1
Lattice=\"1 0 0 0 1 0 0 0 1\" Properties=charge:R:1:species:S:1:pos:R:3
0.5 Cu 0 0 0
";
        assert!(tables_from_extxyz_bytes(with_charge).is_ok());
        let unquoted = b"\
1
Lattice=1 0 0 0 1 0 0 0 1 Properties=species:S:1:pos:R:3
Cu 0 0 0
";
        assert!(tables_from_extxyz_bytes(unquoted).is_err());
        let trailing = b"\
1
Lattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3:
Cu 0 0 0
";
        let _ = tables_from_extxyz_bytes(trailing);
        let crlf = b"1\r\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\r\nCu 0 0 0\r\n";
        assert!(tables_from_extxyz_bytes(crlf).is_ok());
        let no_nl = b"1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nCu 0 0 0";
        assert!(tables_from_extxyz_bytes(no_nl).is_ok());
        let blank = b"\n1\nLattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S:1:pos:R:3\nCu 0 0 0\n";
        assert!(tables_from_extxyz_bytes(blank).is_ok());
        let half = b"\
1
Lattice=\"1 0 0 0 1 0 0 0 1\" Properties=species:S
Cu 0 0 0
";
        assert!(tables_from_extxyz_bytes(half).is_err());
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing.xyz");
        assert!(tables_from_extxyz_path(&missing).is_err());
        let present = dir.path().join("present.xyz");
        std::fs::write(&present, crlf).expect("write");
        assert!(tables_from_extxyz_path(&present).is_ok());
    }
}
