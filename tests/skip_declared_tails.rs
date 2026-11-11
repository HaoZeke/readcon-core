// Copyright (c) readcon-core developers
//! An empty section list and a cut legacy row leave the tail in place.
use readcon_core::iterators::ConFrameIterator;

fn coords(sections: bool) -> String {
    let line2 = if sections {
        "{\"con_spec_version\":2,\"sections\":[]}"
    } else {
        "{\"con_spec_version\":2}"
    };
    format!(
        "user\n{line2}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n1\n1\n1.0\nH\nCoordinates of Component 1\n0.0 0.0 0.0 0 1\n"
    )
}

#[test]
fn an_empty_section_list_keeps_a_bad_byte() {
    let mut bytes = coords(true).into_bytes();
    bytes.push(0xff);
    bytes.push(b'\n');
    let mut it = ConFrameIterator::from_bytes(&bytes);
    assert!(it.forward_fast().unwrap().is_ok());
    let pos = it.byte_pos();
    assert_eq!(bytes[pos], 0xff);
    let err = it.next().unwrap().unwrap_err();
    let text = err.to_string();
    assert!(
        text.contains("UTF-8") || text.contains("invalid"),
        "{text}"
    );
}

#[test]
fn an_empty_section_list_keeps_a_closed_blank() {
    let mut text = coords(true);
    text.push('\n');
    text.push_str("not-a-section\n");
    let bytes = text.into_bytes();
    let mut it = ConFrameIterator::from_bytes(&bytes);
    assert!(it.forward_fast().unwrap().is_ok());
    let tail = std::str::from_utf8(&bytes[it.byte_pos()..]).unwrap();
    assert_eq!(tail, "\nnot-a-section\n");
}

#[test]
fn a_cut_legacy_row_stays_after_the_blank() {
    let mut text = coords(false);
    text.push('\n');
    text.push('H');
    let bytes = text.into_bytes();
    let mut it = ConFrameIterator::from_bytes(&bytes);
    assert!(it.forward_fast().unwrap().is_ok());
    let tail = &bytes[it.byte_pos()..];
    assert_eq!(tail, b"\nH");
}

#[test]
fn a_legacy_label_without_a_newline_stays_put() {
    let mut text = coords(false);
    text.push('\n');
    text.push_str("H\nVelocities of Component 1");
    let bytes = text.into_bytes();
    let mut it = ConFrameIterator::from_bytes(&bytes);
    assert!(it.forward_fast().unwrap().is_ok());
    assert_eq!(&bytes[it.byte_pos()..], b"\nH\nVelocities of Component 1");
}
