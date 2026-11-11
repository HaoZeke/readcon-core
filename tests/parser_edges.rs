// Copyright (c) readcon developers
//! Section and header edges through the library iterator.

use readcon_core::error::ParseError;
use readcon_core::iterators::ConFrameIterator;
use readcon_core::parser::parse_line_of_range_f64;

#[test]
fn a_zero_section_arity_is_rejected() {
    let err = parse("user\n{\"con_spec_version\":2,\"section_arity\":{\"widgets\":0}}\n")
        .expect_err("arity");
    assert!(err.to_string().contains(">= 1"), "{err}");
}

#[test]
fn a_short_float_line_uses_the_stack_buffer() {
    let three = parse_line_of_range_f64("1.5 2.5 3.5", 3, 3, &[0.0; 3]).unwrap();
    assert_eq!(three, vec![1.5, 2.5, 3.5]);
    let six = parse_line_of_range_f64("1 2 3 4 5 6", 6, 6, &[0.0; 6]).unwrap();
    assert_eq!(six.len(), 6);
    assert!((six[5] - 6.0).abs() < 1e-12);
}

fn parse(text: &str) -> Result<readcon_core::types::ConFrame, ParseError> {
    ConFrameIterator::new(text).next().expect("a result")
}

fn replace_header(path: &str, header: &str) -> String {
    let raw = std::fs::read_to_string(path).unwrap();
    let mut lines = raw.lines();
    let user = lines.next().unwrap();
    let _old = lines.next().unwrap();
    format!("{user}\n{header}\n{}\n", lines.collect::<Vec<_>>().join("\n"))
}

#[test]
fn a_malformed_velocity_row_is_still_an_error() {
    let text = "\
user
{\"con_spec_version\":2}
10.0 10.0 10.0
90.0 90.0 90.0


1
1
1.0
H
Coordinates of Component 1
0.0 0.0 0.0 0 1

H
Velocities of Component 1
bad 0.2 0.3 0 1
";
    let err = parse(text).expect_err("bad velocity");
    let message = err.to_string();
    assert!(
        message.contains("bad") || message.contains("float") || message.contains("number"),
        "{err}"
    );

    let good = text.replace("bad 0.2 0.3 0 1", "0.1 0.2 0.3 0 1");
    let frame = parse(&good).expect("velocity");
    let velocity = frame.atom_data[0].velocity().expect("stored velocity");
    assert_eq!(velocity, [0.1, 0.2, 0.3]);
}

#[test]
fn a_legacy_minor_one_velocity_block_needs_a_time_unit() {
    let text = replace_header(
        "resources/test/tiny_cuh2.convel",
        r#"{"con_spec_version":3,"con_spec_minor":1,"units":{"length":"angstrom","energy":"eV"}}"#,
    );
    let err = parse(&text).expect_err("missing time");
    assert!(err.to_string().contains("time"), "{err}");
}

#[test]
fn validated_forces_keep_the_stored_component() {
    let text = "\
user
{\"con_spec_version\":2,\"sections\":[\"forces\"],\"validate\":true}
10.0 10.0 10.0
90.0 90.0 90.0


1
1
1.0
Cu
Coordinates of Component 1
0.0 0.0 0.0 0 1

Cu
Forces of Component 1
0.1 0.2 0.3 0 1
";
    let frame = parse(text).expect("forces");
    let force = frame.atom_data[0].force().unwrap();
    assert_eq!(force, [0.1, 0.2, 0.3]);

    let bad = text.replace("Forces of Component", "Force of Component");
    let err = parse(&bad).expect_err("label");
    assert!(
        err.to_string().contains("Forces")
            || err.to_string().contains("force")
            || err.to_string().contains("Incomplete"),
        "{err}"
    );
}

#[test]
fn a_newer_minor_skips_an_unknown_block_only_when_the_rows_are_there() {
    let base = replace_header(
        "resources/test/tiny_cuh2.con",
        r#"{"con_spec_version":3,"con_spec_minor":2,"units":{"length":"angstrom","energy":"eV"},"sections":["widgets"],"section_arity":{"widgets":1}}"#,
    );
    let missing = parse(&base).expect_err("no block");
    assert!(missing.to_string().contains("widgets") || missing.to_string().contains("Incomplete"), "{missing}");

    let mut opened = base.clone();
    opened.push_str("\n");
    let blank = parse(&opened).expect_err("blank only");
    assert!(blank.to_string().contains("Incomplete") || blank.to_string().contains("widgets"), "{blank}");

    let mut named = base.clone();
    named.push_str("\nCu\n");
    let label = parse(&named).expect_err("no label");
    assert!(label.to_string().contains("Incomplete") || label.to_string().contains("widgets"), "{label}");
}

#[test]
fn a_wrong_spin_label_and_a_wrong_magmom_label_are_incomplete() {
    let charges = std::fs::read_to_string("resources/test/tiny_cuh2_charges_spins_magmoms.con").unwrap();
    let frame = parse(&charges).expect("sections");
    assert!(frame.atom_data.iter().any(|atom| atom.charge().is_some()));
    let spins = charges.replace("Spins of Component", "Spin of Component");
    let err = parse(&spins).expect_err("spin label");
    assert!(err.to_string().contains("spin") || err.to_string().contains("Incomplete"), "{err}");
    let mag = charges.replace("Magmoms of Component", "Mag of Component");
    let err = parse(&mag).expect_err("magmom label");
    assert!(err.to_string().contains("magmom") || err.to_string().contains("Incomplete") || err.to_string().contains("Mag"), "{err}");
}
