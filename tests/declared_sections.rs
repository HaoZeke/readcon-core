// Copyright (c) readcon developers
//! Declared sections that are missing, unknown, or stored as energies.

use readcon_core::error::ParseError;
use readcon_core::iterators::ConFrameIterator;

fn body() -> &'static str {
    include_str!("../resources/test/tiny_cuh2.con")
}

fn with_header(header: &str) -> String {
    let mut lines = body().lines();
    let user = lines.next().unwrap();
    let _old = lines.next();
    let rest: Vec<_> = lines.collect();
    format!("{user}\n{header}\n{}\n", rest.join("\n"))
}

fn parse(text: &str) -> Result<readcon_core::types::ConFrame, ParseError> {
    ConFrameIterator::new(text).next().expect("a frame result")
}

#[test]
fn a_declared_section_with_no_block_names_that_section() {
    let names = [
        "velocities",
        "forces",
        "energies",
        "charges",
        "spins",
        "magmoms",
        "displacements",
        "spreads",
    ];
    for name in names {
        let text = with_header(&format!(
            r#"{{"con_spec_version":2,"sections":["{name}"]}}"#
        ));
        let err = parse(&text).expect_err(name);
        let message = err.to_string();
        assert!(
            message.contains(name) || message.contains("Incomplete"),
            "{name}: {message}"
        );
    }
}

#[test]
fn an_unknown_section_is_rejected_and_a_later_minor_needs_an_arity() {
    let current = with_header(r#"{"con_spec_version":2,"sections":["widgets"]}"#);
    let err = parse(&current).expect_err("unknown");
    assert!(err.to_string().contains("widgets"), "{err}");

    let later = with_header(
        r#"{"con_spec_version":3,"con_spec_minor":2,"units":{"length":"angstrom","energy":"eV"},"sections":["widgets"]}"#,
    );
    let err = parse(&later).expect_err("later minor");
    assert!(err.to_string().contains("widgets"), "{err}");
}

#[test]
fn energies_round_trip_the_stored_value() {
    let header = r#"{"con_spec_version":2,"sections":["energies"],"validate":true}"#;
    let mut text = with_header(header);
    text.push_str(
        "\nCu\nEnergies of Component 1\n  4.5 7 0\n  1.25 7 1\nH\nEnergies of Component 2\n  0.5 0 2\n  0.25 0 3\n",
    );
    let frame = parse(&text).expect("energies");
    let values: Vec<_> = frame
        .atom_data
        .iter()
        .map(|atom| atom.energy().unwrap())
        .collect();
    assert_eq!(values, vec![4.5, 1.25, 0.5, 0.25]);

    let mut wrong = with_header(header);
    wrong.push_str("\nCu\nForces of Component 1\n  4.5 7 0\n");
    let err = parse(&wrong).expect_err("wrong label");
    assert!(err.to_string().contains("energies") || err.to_string().contains("Incomplete"), "{err}");
}
