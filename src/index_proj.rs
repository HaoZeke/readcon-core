// Copyright (c) readcon developers
//! Campaign-store **index projection**: screening scalars and contracts derived from
//! a parsed [`ConFrame`](crate::types::ConFrame) so corpora (e.g. `readcon-db`) do not
//! fork CON semantics.
//!
//! # Finite scalars
//! Energy, mass, volume, \(f_{\max}\), and metadata channels used for ordered indexes
//! are included **only when finite** (`f64::is_finite`). NaN/Inf are omitted so B-tree
//! range scans never see non-orderable keys.
//!
//! # Sections mask
//! [`sections_present_mask`] / [`SECTIONS_MASK_*`] summarize forces / velocities /
//! energies presence for flag indexes without scanning blobs twice.
//!
//! # Spans
//! Multi-frame ingest should use [`crate::iterators::ConFrameIterator::next_with_raw_span`]
//! so stored blobs are exact substrings; [`frame_byte_spans`] is a structural pre-pass
//! (offsets only) for planning.

use crate::types::{ConFrame, SECTION_ENERGIES, SECTION_FORCES, SECTION_VELOCITIES};
use std::collections::BTreeMap;

/// Bit 0: forces section or per-atom forces present.
pub const SECTIONS_MASK_FORCES: u8 = 1 << 0;
/// Bit 1: velocities section or per-atom velocities present.
pub const SECTIONS_MASK_VELOCITIES: u8 = 1 << 1;
/// Bit 2: energies section or finite frame energy present.
pub const SECTIONS_MASK_ENERGIES: u8 = 1 << 2;

/// Canonical multiset formula: sorted non-empty `Sym:count` joined by `|`.
/// Example: Cu₂H₂ → `Cu:2|H:2`. Shared with campaign `idx_formula` keys.
pub fn composition_formula(counts: &[(String, u32)]) -> String {
    let mut parts: Vec<(String, u32)> = counts
        .iter()
        .filter(|(s, c)| !s.is_empty() && *c > 0)
        .cloned()
        .collect();
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    parts
        .into_iter()
        .map(|(s, c)| format!("{s}:{c}"))
        .collect::<Vec<_>>()
        .join("|")
}

/// Species multiset from atom symbols (non-empty only), sorted by symbol.
pub fn species_counts_from_symbols(
    symbols: impl Iterator<Item = impl AsRef<str>>,
) -> Vec<(String, u32)> {
    let mut m = BTreeMap::new();
    for s in symbols {
        let s = s.as_ref();
        if s.is_empty() {
            continue;
        }
        *m.entry(s.to_string()).or_insert(0u32) += 1;
    }
    m.into_iter().collect()
}

/// Species multiset from a frame's `atom_data` symbols.
pub fn frame_species_counts(frame: &ConFrame) -> Vec<(String, u32)> {
    species_counts_from_symbols(frame.atom_data.iter().map(|a| a.symbol.as_ref()))
}

/// Canonical formula string for `frame` (empty string if no non-empty symbols).
pub fn frame_composition_formula(frame: &ConFrame) -> String {
    composition_formula(&frame_species_counts(frame))
}

/// Finite frame energy from header helper or metadata `"energy"`; **None** if missing or non-finite.
pub fn finite_energy(frame: &ConFrame) -> Option<f64> {
    // `energy()` already reads metadata["energy"] as f64. A second lookup
    // cannot observe a value the first call missed, and JSON numbers are finite.
    frame.header.energy().filter(|e| e.is_finite())
}

/// Max Euclidean \(\|F_i\|\) over atoms with force data; None if no finite forces.
pub fn frame_fmax(frame: &ConFrame) -> Option<f64> {
    let mut m = None;
    for a in &frame.atom_data {
        if let Some(f) = a.force() {
            let mag = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
            if mag.is_finite() {
                m = Some(m.map_or(mag, |cur: f64| cur.max(mag)));
            }
        }
    }
    m
}

/// Total mass = Σ masses_per_type[i] * natms_per_type[i] (all finite).
pub fn frame_total_mass(frame: &ConFrame) -> Option<f64> {
    let h = &frame.header;
    if h.masses_per_type.is_empty() || h.natms_per_type.is_empty() {
        return None;
    }
    let n = h.masses_per_type.len().min(h.natms_per_type.len());
    let mut m = 0.0f64;
    for i in 0..n {
        let mi = h.masses_per_type[i];
        let ni = h.natms_per_type[i] as f64;
        // Every `usize` converts to a finite f64, so only the mass can be non-finite.
        if !mi.is_finite() {
            return None;
        }
        m += mi * ni;
    }
    m.is_finite().then_some(m)
}

fn scalar_triple(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

/// Cell volume: prefer lattice determinant; else triclinic from `boxl` + `angles` (degrees).
pub fn frame_cell_volume(frame: &ConFrame) -> Option<f64> {
    if let Some(lv) = frame.header.lattice_vectors() {
        let det = scalar_triple(lv[0], lv[1], lv[2]).abs();
        return det.is_finite().then_some(det);
    }
    let [a, b, c] = frame.header.boxl;
    let [alpha, beta, gamma] = frame.header.angles;
    if ![a, b, c, alpha, beta, gamma]
        .iter()
        .all(|x| x.is_finite() && *x > 0.0)
    {
        return None;
    }
    let ar = alpha.to_radians();
    let br = beta.to_radians();
    let gr = gamma.to_radians();
    let ca = ar.cos();
    let cb = br.cos();
    let cg = gr.cos();
    let sg = gr.sin();
    if sg.abs() < 1e-15 {
        return None;
    }
    let t = 1.0 - ca * ca - cb * cb - cg * cg + 2.0 * ca * cb * cg;
    if t <= 0.0 {
        return None;
    }
    let v = a * b * c * t.sqrt();
    v.is_finite().then_some(v)
}

/// True if forces are declared or any atom carries force data.
pub fn frame_has_forces(frame: &ConFrame) -> bool {
    frame
        .header
        .sections
        .iter()
        .any(|s| s.eq_ignore_ascii_case(SECTION_FORCES))
        || frame.atom_data.iter().any(|a| a.force().is_some())
        || frame.has_forces()
}

/// True if velocities are declared or any atom carries velocity data.
pub fn frame_has_velocities(frame: &ConFrame) -> bool {
    frame
        .header
        .sections
        .iter()
        .any(|s| s.eq_ignore_ascii_case(SECTION_VELOCITIES))
        || frame.atom_data.iter().any(|a| a.velocity().is_some())
        || frame.has_velocities()
}

/// True if energies section is declared or a finite frame energy exists.
pub fn frame_has_energies(frame: &ConFrame) -> bool {
    frame
        .header
        .sections
        .iter()
        .any(|s| s.eq_ignore_ascii_case(SECTION_ENERGIES))
        || finite_energy(frame).is_some()
        || frame.has_energies()
}

/// Bitmask of present sections (forces / velocities / energies) for flag indexes.
pub fn sections_present_mask(frame: &ConFrame) -> u8 {
    let mut m = 0u8;
    if frame_has_forces(frame) {
        m |= SECTIONS_MASK_FORCES;
    }
    if frame_has_velocities(frame) {
        m |= SECTIONS_MASK_VELOCITIES;
    }
    if frame_has_energies(frame) {
        m |= SECTIONS_MASK_ENERGIES;
    }
    m
}

fn meta_f64(frame: &ConFrame, key: &str) -> Option<f64> {
    // `serde_json::Number::as_f64` accepts i64, u64, and f64, and JSON
    // numbers are finite, so a later integer arm cannot run.
    frame
        .header
        .metadata
        .get(key)
        .and_then(|v| v.as_f64())
}

/// ASE.db-competitive / CON-derivable screening projection for one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameIndexProjection {
    pub n_atoms: u32,
    /// Distinct non-empty symbols (sorted via BTree insertion order of counts map).
    pub symbols: Vec<String>,
    /// Per-element counts aligned with campaign `idx_elem_count`.
    pub species_counts: Vec<(String, u32)>,
    /// Canonical multiset formula (`Cu:2|H:2`) or empty.
    pub formula: String,
    /// Finite energy only.
    pub energy: Option<f64>,
    /// Max force magnitude when forces present.
    pub fmax: Option<f64>,
    pub total_mass: Option<f64>,
    pub cell_volume: Option<f64>,
    /// Explicit PBC triple from metadata, if present.
    pub pbc: Option<[bool; 3]>,
    pub sections_mask: u8,
    pub has_forces: bool,
    pub has_velocities: bool,
    pub has_energy: bool,
    pub time: Option<f64>,
    pub timestep: Option<f64>,
    pub frame_index: Option<f64>,
    pub neb_bead: Option<f64>,
    pub neb_band: Option<f64>,
    pub charge: Option<f64>,
    pub magmom: Option<f64>,
}

impl FrameIndexProjection {
    /// Build projection from a fully parsed frame (single source of truth for indexes).
    pub fn from_frame(frame: &ConFrame) -> Self {
        let species_counts = frame_species_counts(frame);
        let symbols: Vec<String> = species_counts.iter().map(|(s, _)| s.clone()).collect();
        let formula = composition_formula(&species_counts);
        let energy = finite_energy(frame);
        let has_forces = frame_has_forces(frame);
        let has_velocities = frame_has_velocities(frame);
        let has_energy = frame_has_energies(frame);
        let sections_mask = {
            let mut m = 0u8;
            if has_forces {
                m |= SECTIONS_MASK_FORCES;
            }
            if has_velocities {
                m |= SECTIONS_MASK_VELOCITIES;
            }
            if has_energy {
                m |= SECTIONS_MASK_ENERGIES;
            }
            m
        };
        let time = frame
            .header
            .time()
            .filter(|t| t.is_finite())
            .or_else(|| meta_f64(frame, "time"));
        let timestep = frame
            .header
            .timestep()
            .filter(|t| t.is_finite())
            .or_else(|| meta_f64(frame, "timestep"));
        let frame_index = frame
            .header
            .frame_index()
            .map(|i| i as f64)
            .filter(|t| t.is_finite())
            .or_else(|| meta_f64(frame, "frame_index"));
        let neb_bead = frame
            .header
            .neb_bead()
            .map(|i| i as f64)
            .filter(|t| t.is_finite())
            .or_else(|| meta_f64(frame, "neb_bead"));
        let neb_band = meta_f64(frame, "neb_band");
        Self {
            n_atoms: frame.atom_data.len() as u32,
            symbols,
            species_counts,
            formula,
            energy,
            fmax: frame_fmax(frame),
            total_mass: frame_total_mass(frame),
            cell_volume: frame_cell_volume(frame),
            pbc: frame.header.pbc(),
            sections_mask,
            has_forces,
            has_velocities,
            has_energy,
            time,
            timestep,
            frame_index,
            neb_bead,
            neb_band,
            charge: meta_f64(frame, "charge"),
            magmom: meta_f64(frame, "magmom"),
        }
    }
}

/// Byte span of one frame within a multi-frame CON buffer (`start` inclusive, `end` exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameByteSpan {
    pub start: usize,
    pub end: usize,
}

impl FrameByteSpan {
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn slice(self, file_contents: &str) -> Option<&str> {
        file_contents.get(self.start..self.end)
    }
}

/// Structural pre-pass: frame byte ranges via span-preserving iteration (parses each frame).
/// Concatenating spans in order reproduces `file_contents` for well-formed multi-frame CON
/// with no leading/trailing junk (trailing newline at EOF may be absorbed into the last span).
/// Frame ranges from a skip walk ([`crate::iterators::frame_start_offsets`]).
/// Does not parse atom payloads. Prefer this over [`frame_byte_spans`] when
/// you only need to seek.
pub fn frame_byte_spans_skip(
    file_contents: &str,
) -> Result<Vec<FrameByteSpan>, crate::error::ParseError> {
    let starts = crate::iterators::frame_start_offsets(file_contents);
    let n = file_contents.len();
    Ok(starts
        .iter()
        .enumerate()
        .map(|(i, &start)| FrameByteSpan {
            start,
            end: starts.get(i + 1).copied().unwrap_or(n),
        })
        .collect())
}

/// Parse frame `index` from `file_contents` using a skip-built span table.
pub fn parse_frame_at(
    file_contents: &str,
    index: usize,
) -> Result<crate::types::ConFrame, crate::error::ParseError> {
    let spans = frame_byte_spans_skip(file_contents)?;
    let span = spans
        .get(index)
        .ok_or(crate::error::ParseError::IndexOutOfBounds {
            index,
            len: spans.len(),
        })?;
    let slice = span
        .slice(file_contents)
        .ok_or(crate::error::ParseError::IncompleteFrame)?;
    match crate::iterators::ConFrameIterator::new(slice).next() {
        Some(r) => r,
        None => Err(crate::error::ParseError::IncompleteFrame),
    }
}

pub fn frame_byte_spans(
    file_contents: &str,
) -> Result<Vec<FrameByteSpan>, crate::error::ParseError> {
    let mut it = crate::iterators::ConFrameIterator::new(file_contents);
    let mut out = Vec::new();
    while let Some(item) = it.next_with_raw_span(file_contents) {
        let (_frame, span) = item?;
        let start = span.as_ptr() as usize - file_contents.as_ptr() as usize;
        let end = start + span.len();
        out.push(FrameByteSpan { start, end });
    }
    Ok(out)
}

/// Symbol multiset histogram without retaining frames (ingest planning / cheap stats).
pub fn symbol_histogram(
    file_contents: &str,
) -> Result<BTreeMap<String, u32>, crate::error::ParseError> {
    let mut hist = BTreeMap::new();
    for item in crate::iterators::ConFrameIterator::new(file_contents) {
        let frame = item?;
        for a in &frame.atom_data {
            if a.symbol.is_empty() {
                continue;
            }
            *hist.entry(a.symbol.to_string()).or_insert(0u32) += 1;
        }
    }
    Ok(hist)
}

/// Ingest contract: successive `next_with_raw_span` slices concatenate to the full buffer
/// when the file is only frames (no prefix garbage). Used by tests and corpus docs.
pub fn spans_cover_buffer(file_contents: &str) -> Result<bool, crate::error::ParseError> {
    let spans = frame_byte_spans(file_contents)?;
    if spans.is_empty() {
        return Ok(file_contents.is_empty());
    }
    // `next_with_raw_span` starts at byte 0 and sets each frame's end to the
    // next frame's start, so a gap or a nonzero start cannot occur.
    let end = spans.last().map(|span| span.end);
    Ok(end == Some(file_contents.len()) || end == Some(file_contents.trim_end().len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterators::ConFrameIterator;
    use crate::writer::ConFrameWriter;
    use std::io::Cursor;

    fn fixture_text() -> String {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/tiny_cuh2.con");
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn formula_canonical_order() {
        let f1 = composition_formula(&[("H".into(), 2), ("Cu".into(), 2)]);
        let f2 = composition_formula(&[("Cu".into(), 2), ("H".into(), 2)]);
        assert_eq!(f1, "Cu:2|H:2");
        assert_eq!(f1, f2);
    }

    #[test]
    fn projection_from_fixture() {
        let text = fixture_text();
        let fr = ConFrameIterator::new(&text).next().unwrap().unwrap();
        let p = FrameIndexProjection::from_frame(&fr);
        assert!(p.n_atoms >= 1);
        assert!(p.formula.contains(':'));
        assert!(p.total_mass.is_some_and(|m| m > 0.0));
        assert!(p.cell_volume.is_some_and(|v| v > 0.0));
        assert_eq!(
            p.sections_mask & SECTIONS_MASK_FORCES == SECTIONS_MASK_FORCES,
            p.has_forces
        );
    }

    #[test]
    fn finite_energy_reads_a_finite_metadata_number() {
        let text = fixture_text();
        let mut fr = ConFrameIterator::new(&text).next().unwrap().unwrap();
        fr.header.metadata.remove("energy");
        assert!(finite_energy(&fr).is_none());
        fr.header.set_energy(-2.5);
        assert_eq!(finite_energy(&fr), Some(-2.5));
        fr.header
            .metadata
            .insert("energy".into(), serde_json::json!("missing"));
        assert!(finite_energy(&fr).is_none());
    }

    #[test]
    fn skip_spans_seek_second_frame() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/tiny_multi_cuh2.con");
        let text = std::fs::read_to_string(p).unwrap();
        let skip = frame_byte_spans_skip(&text).unwrap();
        assert_eq!(skip.len(), 2);
        let a = parse_frame_at(&text, 0).unwrap();
        let b = parse_frame_at(&text, 1).unwrap();
        let mut it = ConFrameIterator::new(&text);
        let fa = it.next().unwrap().unwrap();
        let fb = it.next().unwrap().unwrap();
        assert_eq!(a.atom_data[0].x, fa.atom_data[0].x);
        assert_eq!(b.atom_data[0].x, fb.atom_data[0].x);
        assert!(parse_frame_at(&text, 9).is_err());
    }

    #[test]
    fn span_preserving_concat() {
        let text = fixture_text();
        let spans = frame_byte_spans(&text).unwrap();
        assert!(!spans.is_empty());
        let mut acc = String::new();
        for s in &spans {
            acc.push_str(s.slice(&text).unwrap());
        }
        // re-parse acc must yield same frame count
        let n_orig = ConFrameIterator::new(&text).count();
        let n_acc = ConFrameIterator::new(&acc).count();
        assert_eq!(n_orig, n_acc);
        assert!(spans_cover_buffer(&text).unwrap());
    }

    #[test]
    fn symbol_histogram_nonempty() {
        let h = symbol_histogram(&fixture_text()).unwrap();
        assert!(!h.is_empty());
        let mut builder = crate::types::ConFrameBuilder::new([10.0, 10.0, 10.0], [90.0, 90.0, 90.0]);
        builder.add_atom("", 0.0, 0.0, 0.0, [false; 3], 0, 1.0);
        builder.add_atom("H", 1.0, 0.0, 0.0, [false; 3], 1, 1.0);
        let built = builder.build().unwrap();
        let mut buf = Vec::new();
        {
            let mut writer = ConFrameWriter::new(&mut buf);
            writer.write_frame(&built).unwrap();
        }
        let written = String::from_utf8(buf).unwrap();
        let hist = symbol_histogram(&written).unwrap();
        assert_eq!(hist.get("H"), Some(&1));
        assert!(hist.get("").is_none());
    }

    /// Projection contract vs shipped frame parse (regression if index_proj drifts from CON).
    #[test]
    fn projection_matches_iterator_frame_fields() {
        let text = fixture_text();
        let fr = ConFrameIterator::new(&text).next().unwrap().unwrap();
        let p = FrameIndexProjection::from_frame(&fr);
        assert_eq!(p.n_atoms as usize, fr.atom_data.len());
        assert_eq!(p.n_atoms, fr.positions.nrows() as u32);
        let expect_formula = frame_composition_formula(&fr);
        assert_eq!(p.formula, expect_formula);
        assert!(!p.formula.is_empty());
        assert_eq!(p.species_counts, frame_species_counts(&fr));
        assert!(p.total_mass.is_some_and(|m| m > 0.0));
        assert!(p.cell_volume.is_some_and(|v| v > 0.0));
        let expect_forces = frame_has_forces(&fr);
        let expect_vel = frame_has_velocities(&fr);
        assert_eq!(p.has_forces, expect_forces);
        assert_eq!(p.has_velocities, expect_vel);
        assert_eq!(p.sections_mask, sections_present_mask(&fr));
        for name in [
            "tiny_cuh2_forces.con",
            "tiny_cuh2.convel",
        ] {
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources/test")
                .join(name);
            let body = std::fs::read_to_string(path).unwrap();
            let sample = ConFrameIterator::new(&body).next().unwrap().unwrap();
            let mask = sections_present_mask(&sample);
            assert_eq!(mask, sections_present_mask(&sample));
            assert!(sample.atom_data.len() as u32 == FrameIndexProjection::from_frame(&sample).n_atoms);
        }
        let mut energetic = fr.clone();
        energetic.header.set_energy(1.0);
        assert_eq!(
            sections_present_mask(&energetic) & SECTIONS_MASK_ENERGIES,
            SECTIONS_MASK_ENERGIES
        );

        let forces = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources/test/tiny_cuh2_forces.con"),
        )
        .unwrap();
        let mut forced = ConFrameIterator::new(&forces).next().unwrap().unwrap();
        forced.header.set_energy(0.25);
        let projected = FrameIndexProjection::from_frame(&forced);
        assert!(frame_has_forces(&forced));
        assert!(frame_has_energies(&forced));
        assert_eq!(
            projected.sections_mask & SECTIONS_MASK_FORCES,
            SECTIONS_MASK_FORCES
        );
        assert_eq!(
            projected.sections_mask & SECTIONS_MASK_ENERGIES,
            SECTIONS_MASK_ENERGIES
        );
        let velocities = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources/test/tiny_cuh2.convel"),
        )
        .unwrap();
        let moving = ConFrameIterator::new(&velocities).next().unwrap().unwrap();
        assert!(frame_has_velocities(&moving));
        assert_eq!(
            sections_present_mask(&moving) & SECTIONS_MASK_VELOCITIES,
            SECTIONS_MASK_VELOCITIES
        );
    }

    fn load_named(name: &str) -> ConFrame {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test")
            .join(name);
        let text = std::fs::read_to_string(path).unwrap();
        ConFrameIterator::new(&text).next().unwrap().unwrap()
    }

    #[test]
    fn screening_scalars_cover_empty_degenerate_and_metadata_channels() {
        let counts = species_counts_from_symbols(["Cu", "", "Cu", "H"].into_iter());
        assert_eq!(counts, vec![("Cu".into(), 2u32), ("H".into(), 1)]);
        assert_eq!(
            composition_formula(&[("".into(), 4), ("H".into(), 0), ("Cu".into(), 1)]),
            "Cu:1"
        );

        let mut forced = load_named("tiny_cuh2_forces.con");
        for atom in &mut forced.atom_data {
            atom.set_force(Some([0.0, 0.0, 0.0]));
        }
        forced.atom_data[0].set_force(Some([3.0, 0.0, 0.0]));
        forced.atom_data[1].set_force(Some([0.0, 4.0, 0.0]));
        assert_eq!(frame_fmax(&forced).unwrap(), 4.0);
        forced.atom_data[0].set_force(Some([f64::NAN, 0.0, 0.0]));
        assert_eq!(frame_fmax(&forced).unwrap(), 4.0);
        for atom in &mut forced.atom_data {
            atom.set_force(Some([f64::INFINITY, 0.0, 0.0]));
        }
        assert!(frame_fmax(&forced).is_none());

        let mut massless = load_named("tiny_cuh2.con");
        massless.header.masses_per_type.clear();
        assert!(frame_total_mass(&massless).is_none());
        massless = load_named("tiny_cuh2.con");
        massless.header.natms_per_type.clear();
        assert!(frame_total_mass(&massless).is_none());
        massless = load_named("tiny_cuh2.con");
        massless.header.masses_per_type[0] = f64::NAN;
        assert!(frame_total_mass(&massless).is_none());
        massless = load_named("tiny_cuh2.con");
        massless.header.masses_per_type[0] = 1e308;
        massless.header.natms_per_type[0] = 10;
        assert!(frame_total_mass(&massless).is_none());
        let finite_mass = frame_total_mass(&load_named("tiny_cuh2.con")).unwrap();
        assert!(finite_mass.is_finite());
        assert!(finite_mass > 0.0);

        let mut cell = load_named("tiny_cuh2.con");
        cell.header.set_lattice_vectors([
            [10.0, 0.0, 0.0],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 14.0],
        ]);
        let volume = frame_cell_volume(&cell).unwrap();
        assert!((volume - 1680.0).abs() < 1e-6, "{volume}");
        cell.header.set_lattice_vectors([
            [1e308, 0.0, 0.0],
            [0.0, 1e308, 0.0],
            [0.0, 0.0, 1e308],
        ]);
        assert!(frame_cell_volume(&cell).is_none());
        cell.header.metadata.remove(crate::types::meta::LATTICE_VECTORS);
        cell.header.boxl = [f64::NAN, 1.0, 1.0];
        cell.header.angles = [90.0, 90.0, 90.0];
        assert!(frame_cell_volume(&cell).is_none());
        cell.header.boxl = [0.0, 1.0, 1.0];
        assert!(frame_cell_volume(&cell).is_none());
        cell.header.boxl = [1.0, 1.0, 1.0];
        cell.header.angles = [90.0, 90.0, 1e-18];
        assert!(frame_cell_volume(&cell).is_none());
        cell.header.angles = [5.0, 5.0, 5.0];
        let skewed = frame_cell_volume(&cell).unwrap();
        assert!(skewed.is_finite(), "{skewed}");
        assert!(skewed > 0.0, "{skewed}");
        cell.header.angles = [10.0, 10.0, 170.0];
        assert!(frame_cell_volume(&cell).is_none());

        let mut plain = load_named("tiny_cuh2.con");
        plain.header.sections.clear();
        plain.header.metadata.remove("energy");
        plain.atom_energies = crate::storage_dtype::Array1Storage::zeros_f64(plain.positions.nrows());
        assert!(plain.has_energies());
        assert!(frame_has_energies(&plain));
        plain.atom_energies = crate::storage_dtype::Array1Storage::zeros_f64(0);
        assert!(!frame_has_energies(&plain));
        assert!(!frame_has_forces(&plain));
        assert!(!frame_has_velocities(&plain));
        assert!(!frame_has_energies(&plain));
        plain.header.sections.push("Forces".into());
        assert!(frame_has_forces(&plain));
        plain.header.sections = vec!["Velocities".into()];
        assert!(frame_has_velocities(&plain));
        plain.header.sections = vec!["Energies".into()];
        assert!(frame_has_energies(&plain));
        plain.header.sections.clear();
        plain.atom_data[0].set_force(Some([1.0, 0.0, 0.0]));
        assert!(frame_has_forces(&plain));
        plain.atom_data[0].set_force(None);
        plain.atom_data[0].set_velocity(Some([0.0, 1.0, 0.0]));
        assert!(frame_has_velocities(&plain));
        plain.atom_data[0].set_velocity(None);
        plain.header.set_energy(3.0);
        assert!(frame_has_energies(&plain));
        assert_eq!(finite_energy(&plain), Some(3.0));

        let mut soa = load_named("tiny_cuh2_forces.con");
        soa.header.sections.clear();
        for atom in &mut soa.atom_data {
            atom.set_force(None);
        }
        assert!(soa.has_forces());
        assert!(frame_has_forces(&soa));
        let mut moving = load_named("tiny_cuh2.convel");
        moving.header.sections.clear();
        for atom in &mut moving.atom_data {
            atom.set_velocity(None);
        }
        assert!(moving.has_velocities());
        assert!(frame_has_velocities(&moving));

        plain.header.set_time(1.5);
        plain.header.set_timestep(0.25);
        plain.header.set_frame_index(3);
        plain.header.set_neb_bead(2);
        plain.header.set_neb_band(4);
        plain.header.set_pbc([true, false, true]);
        plain
            .header
            .metadata
            .insert("charge".into(), serde_json::json!(-1.0));
        plain
            .header
            .metadata
            .insert("magmom".into(), serde_json::json!("spin"));
        let moving_full = load_named("tiny_cuh2.convel");
        let moving_projection = FrameIndexProjection::from_frame(&moving_full);
        assert_ne!(
            moving_projection.sections_mask & SECTIONS_MASK_VELOCITIES,
            0
        );
        let forced_projection = FrameIndexProjection::from_frame(&load_named("tiny_cuh2_forces.con"));
        assert_ne!(forced_projection.sections_mask & SECTIONS_MASK_FORCES, 0);
        let projected = FrameIndexProjection::from_frame(&plain);
        assert_eq!(projected.time, Some(1.5));
        assert_eq!(projected.timestep, Some(0.25));
        assert_eq!(projected.frame_index, Some(3.0));
        assert_eq!(projected.neb_bead, Some(2.0));
        assert_eq!(projected.neb_band, Some(4.0));
        assert_eq!(projected.charge, Some(-1.0));
        assert_eq!(projected.magmom, None);
        assert_eq!(projected.pbc, Some([true, false, true]));
        assert_eq!(projected.energy, Some(3.0));
        plain
            .header
            .metadata
            .insert("time".into(), serde_json::json!("later"));
        plain.header.metadata.remove("charge");
        let cleared = FrameIndexProjection::from_frame(&plain);
        assert_eq!(cleared.time, None);
        assert_eq!(cleared.charge, None);

        let empty_span = FrameByteSpan { start: 2, end: 2 };
        assert_eq!(empty_span.len(), 0);
        assert!(empty_span.is_empty());
        assert_eq!(empty_span.slice("abcd"), Some(""));
        assert!(FrameByteSpan { start: 0, end: 8 }.slice("ab").is_none());
        assert_eq!(
            FrameByteSpan { start: 1, end: 3 }.slice("abcd"),
            Some("bc")
        );

        assert!(spans_cover_buffer("").unwrap());
        assert!(parse_frame_at("", 0).is_err());
        let multi = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources/test/tiny_multi_cuh2.con"),
        )
        .unwrap();
        let spans = frame_byte_spans(&multi).unwrap();
        assert!(spans.len() >= 2);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, spans[1].start);
        assert!(spans_cover_buffer(&multi).unwrap());
        let trailed = format!("{multi}   ");
        let trailed_cover = spans_cover_buffer(&trailed);
        assert!(trailed_cover.is_ok());
        let text = fixture_text();
        let with_gap_check = spans_cover_buffer(&text).unwrap();
        assert!(with_gap_check);
        let prefixed = format!("xx\n{text}");
        assert!(spans_cover_buffer(&prefixed).is_err());
        let trailing = format!("{text}\nnot-a-frame\n");
        assert!(spans_cover_buffer(&trailing).is_err());
    }

    #[test]
    fn canonical_writer_deterministic() {
        let text = fixture_text();
        let fr = ConFrameIterator::new(&text).next().unwrap().unwrap();
        let mut a = Cursor::new(Vec::new());
        let mut b = Cursor::new(Vec::new());
        {
            let mut wa = ConFrameWriter::new(&mut a).canonical(true);
            let mut wb = ConFrameWriter::new(&mut b).canonical(true);
            wa.write_frame(&fr).unwrap();
            wb.write_frame(&fr).unwrap();
        }
        assert_eq!(a.into_inner(), b.into_inner());
    }
}
