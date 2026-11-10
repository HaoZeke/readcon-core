//! Prefix and declared-empty edges of the frame skip.

use readcon_core::iterators::ConFrameIterator;

#[test]
fn an_empty_section_list_stops_before_a_bad_byte() {
    let text = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
    let header = br#"{"con_spec_version":2,"sections":[]}"#;
    let end = text.iter().position(|b| *b == b'\n').unwrap();
    // The first newline is the user line. Find the metadata line.
    let meta = text[end + 1..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap()
        + end
        + 1;
    let mut out = text[..end + 1].to_vec();
    out.extend_from_slice(header);
    out.push(b'\n');
    out.extend_from_slice(&text[meta + 1..]);
    out.extend_from_slice(b"\n\xff\n");
    let mut iter = ConFrameIterator::from_bytes(&out);
    let frame = iter.next().unwrap().expect("frame");
    assert!(frame.atom_data.len() >= 1);
    assert!(frame.header.sections.is_empty());
}

#[test]
fn a_cut_legacy_velocity_prefix_is_not_a_finished_file() {
    let file = std::fs::read("resources/test/tiny_cuh2.convel").unwrap();
    let label = file
        .windows(b"Velocities of Component".len())
        .position(|w| w == b"Velocities of Component")
        .unwrap();
    let prefix = &file[..label + b"Velocities of Component 1\n".len()];
    let err = ConFrameIterator::from_prefix(prefix)
        .next()
        .unwrap()
        .expect_err("cut block");
    let message = err.to_string();
    assert!(
        message.contains("velocities") || message.contains("Incomplete"),
        "{message}"
    );
}
