//! Reader and writer behavior for clauses that no corpus file named.
//!
//! Each test's doc comment names the clause. The checks call the library.

use readcon_core::error::ParseError;
use readcon_core::iterators::ConFrameIterator;
use readcon_core::parser::parse_frame_header;
use readcon_core::types::{pre_grouping_indices, Bond, ConFrameBuilder};
use readcon_core::units::velocity_file_time_unit;
use readcon_core::writer::{ConFrameWriter, FloatFormat, WriteError};

fn meta(extra: &str) -> String {
    format!(
        r#"{{"con_spec_version":3,"con_spec_minor":1,"units":{{"length":"angstrom","energy":"eV","time":"fs"}},"sections":[]{extra}}}"#
    )
}

fn builder(extra: &str) -> ConFrameBuilder {
    let mut builder = ConFrameBuilder::new([10.0, 10.0, 10.0], [90.0, 90.0, 90.0]);
    builder
        .postbox_header([String::new(), String::new()])
        .prebox_header("frame")
        .set_metadata_json(&meta(extra))
        .unwrap();
    builder
}

fn one_atom(extra: &str, fixed: [bool; 3], atom_id: u64) -> readcon_core::types::ConFrame {
    let mut builder = builder(extra);
    builder.add_atom("H", 1.0, 2.0, 3.0, fixed, atom_id, 1.0);
    builder.build().unwrap()
}

fn write(frame: &readcon_core::types::ConFrame) -> String {
    let mut buffer = Vec::new();
    {
        let mut writer = ConFrameWriter::with_float_format(&mut buffer, FloatFormat::RoundTrip);
        writer.write_frame(frame).unwrap();
    }
    String::from_utf8(buffer).unwrap()
}

fn parse(text: &str) -> Result<Vec<readcon_core::types::ConFrame>, ParseError> {
    ConFrameIterator::new(text).collect()
}

/// CON-file.6: a writer does not insert a blank line between two frames.
#[test]
fn con_file_6_writer_does_not_insert_a_blank_line_between_frames() {
    let frame = one_atom("", [false, false, false], 4);
    let mut buffer = Vec::new();
    {
        let mut writer = ConFrameWriter::with_float_format(&mut buffer, FloatFormat::RoundTrip);
        writer.write_frame(&frame).unwrap();
        writer.write_frame(&frame).unwrap();
    }
    let text = String::from_utf8(buffer).unwrap();
    assert!(
        !text.contains("\n\nframe\n"),
        "an extra blank line precedes the second frame:\n{text}"
    );
    let frames = parse(&text).unwrap();
    assert_eq!(frames.len(), 2);
}

/// CON-header.1: lines 5 and 6 may be empty, and the reader accepts that file.
#[test]
fn con_header_1_empty_lines_5_and_6_are_accepted() {
    let text = write(&one_atom("", [false, false, false], 4));
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() > 6, "{text}");
    assert_eq!(lines[4], "", "line 5: {text}");
    assert_eq!(lines[5], "", "line 6: {text}");
    let frames = parse(&text).unwrap();
    assert_eq!(frames[0].header.postbox_header, ["", ""]);
}

/// CON-count.6: coordinate blocks follow the header's type order.
#[test]
fn con_count_6_coordinate_blocks_follow_header_order() {
    let mut builder = builder("");
    builder.add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 4, 1.0);
    builder.add_atom("O", 1.0, 0.0, 0.0, [false, false, false], 5, 16.0);
    let text = write(&builder.build().unwrap());
    let symbols: Vec<&str> = text
        .lines()
        .filter(|line| *line == "H" || *line == "O")
        .collect();
    assert_eq!(symbols, ["H", "O"], "{text}");
    let frame = parse(&text).unwrap().remove(0);
    assert_eq!(&*frame.atom_data[0].symbol, "H");
    assert_eq!(&*frame.atom_data[1].symbol, "O");
}

/// CON-mask.6 and CON-mask.7: spec 2 and later emit 7 for all-fixed, never 1.
#[test]
fn con_mask_6_and_7_all_fixed_is_written_as_7() {
    let text = write(&one_atom("", [true, true, true], 4));
    let row = text
        .lines()
        .find(|line| line.split_whitespace().count() >= 5 && line.contains(" 7 "))
        .unwrap_or_else(|| panic!("no all-fixed row in\n{text}"));
    let fields: Vec<&str> = row.split_whitespace().collect();
    assert_eq!(fields[3], "7", "{row}");
    assert_ne!(fields[3], "1");
    let frame = parse(&text).unwrap().remove(0);
    assert_eq!(frame.atom_data[0].fixed, [true, true, true]);
}

/// CON-id.2: the reader does not assume ids start at 0 or at 1.
#[test]
fn con_id_2_reader_accepts_ids_that_do_not_start_at_zero_or_one() {
    let mut builder = builder("");
    builder.add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 5, 1.0);
    builder.add_atom("H", 1.0, 0.0, 0.0, [false, false, false], 40, 1.0);
    let frame = parse(&write(&builder.build().unwrap())).unwrap().remove(0);
    let ids: Vec<u64> = frame.atom_data.iter().map(|atom| atom.atom_id).collect();
    assert_eq!(ids, [5, 40]);
}

/// CON-id.7: pre-grouping order is ascending unique id, and file order when an id repeats.
#[test]
fn con_id_7_pre_grouping_order_uses_con_id_5() {
    assert_eq!(pre_grouping_indices(&[5, 2, 9]), vec![1, 0, 2]);
    assert_eq!(pre_grouping_indices(&[5, 2, 5]), vec![0, 1, 2]);
}

/// CON-section.3: a writer does not put a blank line between component blocks of one section.
/// CON-section.5 and CON-section.6: the per-atom energy section is energies, not atom_energies.
#[test]
fn con_section_3_5_and_6_energies_section_has_no_blank_between_components() {
    let mut builder = builder("");
    builder
        .set_metadata_json(
            r#"{"con_spec_version":3,"con_spec_minor":1,"units":{"length":"angstrom","energy":"eV","time":"fs"},"sections":["energies"]}"#,
        )
        .unwrap();
    builder
        .add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 4, 1.0)
        .with_energy(0.25);
    builder
        .add_atom("O", 1.0, 0.0, 0.0, [false, false, false], 5, 16.0)
        .with_energy(0.5);
    let text = write(&builder.build().unwrap());
    assert!(text.contains("Energies of Component 1"), "{text}");
    assert!(text.contains("Energies of Component 2"), "{text}");
    assert!(!text.contains("atom_energies"), "{text}");
    let energies = text.split("Energies of Component 1").nth(1).unwrap();
    let between = energies.split("Energies of Component 2").next().unwrap();
    assert!(
        !between.contains("\n\n"),
        "blank line inside the energies section:\n{text}"
    );
}

/// CON-section.19: a frame energy that is not the sum of the energies rows is still read.
#[test]
fn con_section_19_energy_sum_mismatch_does_not_reject_the_frame() {
    let mut builder = builder(r#""#);
    builder
        .set_metadata_json(
            r#"{"con_spec_version":3,"con_spec_minor":1,"units":{"length":"angstrom","energy":"eV","time":"fs"},"sections":["energies"],"energy":9.0}"#,
        )
        .unwrap();
    builder
        .add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 4, 1.0)
        .with_energy(0.25);
    let frame = parse(&write(&builder.build().unwrap())).unwrap().remove(0);
    assert_eq!(frame.header.energy(), Some(9.0));
    assert_eq!(frame.atom_data[0].energy(), Some(0.25));
}

/// CON-validate.2: validate true does not relax the non-finite rejection.
#[test]
fn con_validate_2_does_not_relax_a_check_that_already_applies() {
    let text = write(&one_atom("", [false, false, false], 4));
    let broken = text.replacen("1.0", "NaN", 1);
    assert_ne!(broken, text, "replacen left the coordinate unchanged");
    assert!(broken.contains("NaN"), "{broken}");
    let err = parse(&broken).unwrap_err();
    assert!(
        matches!(err, ParseError::NonFiniteValue(_)),
        "validate is absent and NaN is still rejected: {err}"
    );
    let strict = text.replacen(
        r#""con_spec_version":3"#,
        r#""con_spec_version":3,"validate":true,"sections":[]"#,
        1,
    );
    assert_ne!(strict, text, "the validate insertion did not change the text");
    assert!(strict.contains("\"validate\":true"), "{strict}");
    let strict = strict.replacen("1.0", "NaN", 1);
    let err = parse(&strict).unwrap_err();
    assert!(
        matches!(err, ParseError::NonFiniteValue(_)),
        "validate true still rejects NaN: {err}"
    );
}

/// CON-bonds.2: validate true rejects a malformed bonds entry.
#[test]
fn con_bonds_2_validate_true_rejects_a_malformed_bonds_entry() {
    let lines = [
        "PREBOX1",
        r#"{"con_spec_version":2,"sections":[],"validate":true,"bonds":[[0]]}"#,
        "10.0 20.0 30.0",
        "90.0 90.0 90.0",
        "POSTBOX1",
        "POSTBOX2",
        "1",
        "1",
        "12.011",
    ];
    let err = parse_frame_header(&mut lines.iter().copied()).unwrap_err();
    assert!(matches!(err, ParseError::InvalidMetadataJson(_)));
    assert!(err.to_string().contains("bonds"), "{err}");
}

/// CON-bonds.1: bonds is frame topology and is not a name in sections.
#[test]
fn con_bonds_1_bonds_are_not_a_section() {
    let mut builder = builder("");
    builder.add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 4, 1.0);
    builder.add_atom("H", 1.0, 0.0, 0.0, [false, false, false], 5, 1.0);
    builder.add_bond(Bond::new(0, 1));
    let text = write(&builder.build().unwrap());
    let line2 = text.lines().nth(1).unwrap();
    assert!(line2.contains("bonds"), "{line2}");
    assert!(!line2.contains("\"bonds\"]"), "{line2}");
    let frame = parse(&text).unwrap().remove(0);
    assert!(!frame.header.sections.iter().any(|name| name == "bonds"));
}

/// CON-units.15: an explicit time unit overrides the unit declared on the frame.
#[test]
fn con_units_15_consumer_may_apply_an_explicit_time_unit() {
    let metadata = serde_json::json!({
        "con_spec_version": 3,
        "con_spec_minor": 1,
        "units": {"length": "angstrom", "energy": "eV", "time": "fs"}
    });
    let metadata = metadata.as_object().unwrap();
    let declared = velocity_file_time_unit(3, metadata, None).unwrap();
    assert_eq!(declared, "fs");
    let overridden = velocity_file_time_unit(3, metadata, Some("ps")).unwrap();
    assert_eq!(overridden, "ps");
    assert_ne!(overridden, declared);
}

/// CON-num.4: the C ABI absent-value NaN is not a CON number and is not written.
#[test]
fn con_num_4_absent_abi_nan_is_not_written() {
    let frame = one_atom("", [false, false, false], 4);
    let handle = &frame as *const readcon_core::types::ConFrame
        as *const readcon_core::ffi::RKRConFrame;
    let sentinel = readcon_core::ffi::rkr_frame_energy(handle);
    let missing = readcon_core::ffi::rkr_frame_energy(std::ptr::null());
    assert!(sentinel.is_nan(), "a frame with no energy returns the ABI sentinel");
    assert!(missing.is_nan(), "a null frame returns the ABI sentinel");
    let text = write(&frame);
    let lower = text.to_ascii_lowercase();
    assert!(!lower.contains("nan"), "the absent-energy sentinel was written:\n{text}");
    assert!(!lower.contains("inf"), "{text}");
    let broken = text.replacen("1.0", "NaN", 1);
    assert_ne!(broken, text);
    let err = parse(&broken).unwrap_err();
    assert!(
        matches!(err, ParseError::NonFiniteValue(_)),
        "a non-finite token was not rejected: {err}"
    );
    let mut builder = builder("");
    builder
        .add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 4, 1.0)
        .with_spread([0.1, 0.2, 0.3]);
    let mut poisoned = builder.build().unwrap();
    poisoned.atom_data[0].set_spread(Some([f64::NAN, 0.2, 0.3]));
    let mut buffer = Vec::new();
    let err = {
        let mut writer = ConFrameWriter::with_float_format(&mut buffer, FloatFormat::RoundTrip);
        writer.write_frame(&poisoned).unwrap_err()
    };
    assert!(
        matches!(err, WriteError::Format(ParseError::NonFiniteValue(_))),
        "{err}"
    );
    assert!(
        !String::from_utf8_lossy(&buffer).to_ascii_lowercase().contains("nan"),
        "a rejected non-finite spread was written"
    );
}

/// CON-token.1: an integer token is digits only. A sign is not an id.
#[test]
fn con_token_1_a_signed_integer_is_not_an_id_token() {
    let text = write(&one_atom("", [false, false, false], 4));
    let row = text
        .lines()
        .find(|line| line.split_whitespace().last() == Some("4"))
        .unwrap();
    let fields: Vec<&str> = row.split_whitespace().collect();
    assert!(fields[4].bytes().all(|byte| byte.is_ascii_digit()), "{row}");
    let bad = text.replacen(" 4\n", " -4\n", 1);
    assert!(parse(&bad).is_err(), "a signed id was accepted:\n{bad}");
}
