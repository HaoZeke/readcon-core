//! Calls the public C entry points on one configuration file and on null.

use readcon_core::ffi::{
    free_rkr_frame, rkr_count_frames, rkr_frame_atom_count, rkr_frame_bond_count,
    rkr_frame_cell_volume, rkr_frame_composition_formula, rkr_frame_energy, rkr_frame_fmax,
    rkr_frame_metadata_json, rkr_frame_total_mass, rkr_library_version, rkr_read_first_frame,
    rkr_read_nth_frame, rkr_status_message, rkr_symbol_to_z, rkr_z_to_symbol, RKRStatus,
};
use std::ffi::{CStr, CString};
use std::fs;
use std::io::Write;
use std::ptr;

fn write_con(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("cover.con");
    let mut file = fs::File::create(&path).expect("create");
    write!(
        file,
        "user\n{{\"con_spec_version\":2}}\n8.0 9.0 10.0\n90.0 90.0 90.0\n0 0\n0 0\n1\n1\n63.546\nCu\nCoordinates of Component 1\n1.0 0.0 0.0 0 0\n"
    )
    .expect("write");
    path
}

#[test]
fn the_public_c_api_reads_one_frame_and_rejects_null() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_con(dir.path());
    let name = CString::new(path.to_str().expect("utf8")).expect("cstring");
    unsafe {
        assert!(!rkr_library_version().is_null());
        assert!(!rkr_status_message(RKRStatus::RKR_STATUS_SUCCESS).is_null());
        let copper = CString::new("Cu").expect("cu");
        assert_eq!(rkr_symbol_to_z(copper.as_ptr()), 29);
        assert_eq!(rkr_symbol_to_z(ptr::null()), 0);
        assert!(!rkr_z_to_symbol(1).is_null());
        assert_eq!(rkr_count_frames(name.as_ptr()), 1);
        assert_eq!(rkr_count_frames(ptr::null()), usize::MAX);
        let frame = rkr_read_first_frame(name.as_ptr());
        assert!(!frame.is_null());
        assert_eq!(rkr_frame_atom_count(frame), 1);
        let _ = rkr_frame_atom_count(ptr::null());
        assert!(rkr_frame_total_mass(frame).is_finite());
        assert!(rkr_frame_total_mass(ptr::null()).is_nan());
        assert!(rkr_frame_cell_volume(frame).is_finite());
        assert!(rkr_frame_cell_volume(ptr::null()).is_nan());
        assert_eq!(rkr_frame_bond_count(frame), 0);
        assert_eq!(rkr_frame_bond_count(ptr::null()), 0);
        let formula = rkr_frame_composition_formula(frame);
        assert!(!formula.is_null());
        let _ = CStr::from_ptr(formula);
        readcon_core::ffi::rkr_free_string(formula);
        assert!(rkr_frame_composition_formula(ptr::null()).is_null());
        let meta = rkr_frame_metadata_json(frame);
        assert!(!meta.is_null());
        readcon_core::ffi::rkr_free_string(meta);
        let _ = rkr_frame_energy(frame);
        let _ = rkr_frame_energy(ptr::null());
        let _ = rkr_frame_fmax(frame);
        let _ = rkr_frame_fmax(ptr::null());
        free_rkr_frame(frame);
        free_rkr_frame(ptr::null_mut());
        let nth = rkr_read_nth_frame(name.as_ptr(), 0);
        assert!(!nth.is_null());
        free_rkr_frame(nth);
        assert!(rkr_read_first_frame(ptr::null()).is_null());
        assert!(rkr_read_nth_frame(ptr::null(), 0).is_null());
    }
}
