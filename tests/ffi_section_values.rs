//! Each section copy is its own function. Absent columns error. Present columns match.

use std::ffi::CString;
use std::ptr;

use readcon_core::ffi::*;

fn builder_with_columns() -> *mut RKRConFrame {
    let cell = [10.0f64, 10.0, 10.0];
    let angles = [90.0f64, 90.0, 90.0];
    let builder = unsafe {
        rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert!(!builder.is_null());
    let symbol = CString::new("Cu").unwrap();
    assert_eq!(
        unsafe { rkr_frame_add_atom(builder, symbol.as_ptr(), 1.25, 2.5, 3.75, false, 7, 63.546) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let row = [0.1f64, 0.2, 0.3];
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_velocity(builder, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_force(builder, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_displacement(builder, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_spread(builder, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_energy(builder, 0, 4.5) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let units = CString::new(
        r#"{"length":"A","energy":"ev","mass":"amu","time":"(amu*angstrom^2/eV)^0.5"}"#,
    )
    .unwrap();
    assert_eq!(
        unsafe { rkr_frame_builder_set_units(builder, units.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let frame = unsafe { rkr_frame_builder_build(builder) };
    assert!(!frame.is_null());
    frame
}

#[test]
fn plain_columns_copy_and_optional_columns_are_absent() {
    let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let frame = unsafe { rkr_read_nth_frame(path.as_ptr(), 0) };
    assert!(!frame.is_null());
    let mut xyz = [0.0f64; 12];
    assert_eq!(
        unsafe { rkr_frame_copy_positions(frame, xyz.as_mut_ptr(), xyz.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((xyz[0] - 0.6394).abs() < 1e-9);
    assert!((xyz[1] - 0.9045).abs() < 1e-9);
    assert!((xyz[2] - 6.9753).abs() < 1e-9);
    let mut mass = [0.0f64; 4];
    assert_eq!(
        unsafe { rkr_frame_copy_masses(frame, mass.as_mut_ptr(), mass.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((mass[0] - 63.546).abs() < 1e-6);
    assert!((mass[2] - 1.00793).abs() < 1e-6);
    let absent = RKRStatus::RKR_STATUS_SECTION_ABSENT;
    assert_eq!(
        unsafe { rkr_frame_copy_velocities(frame, xyz.as_mut_ptr(), xyz.len()) },
        absent
    );
    assert_eq!(
        unsafe { rkr_frame_copy_forces(frame, xyz.as_mut_ptr(), xyz.len()) },
        absent
    );
    assert_eq!(
        unsafe { rkr_frame_copy_displacements(frame, xyz.as_mut_ptr(), xyz.len()) },
        absent
    );
    assert_eq!(
        unsafe { rkr_frame_copy_spreads(frame, xyz.as_mut_ptr(), xyz.len()) },
        absent
    );
    assert_eq!(
        unsafe { rkr_frame_copy_atom_energies(frame, xyz.as_mut_ptr(), xyz.len()) },
        absent
    );
    unsafe { free_rkr_frame(frame) };
}

#[test]
fn optional_columns_round_trip_the_stored_numbers() {
    let frame = builder_with_columns();
    let mut row = [0.0f64; 3];
    assert_eq!(
        unsafe { rkr_frame_copy_velocities(frame, row.as_mut_ptr(), row.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(row, [0.1, 0.2, 0.3]);
    assert_eq!(
        unsafe { rkr_frame_copy_forces(frame, row.as_mut_ptr(), row.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(row, [0.1, 0.2, 0.3]);
    assert_eq!(
        unsafe { rkr_frame_copy_displacements(frame, row.as_mut_ptr(), row.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(row, [0.1, 0.2, 0.3]);
    assert_eq!(
        unsafe { rkr_frame_copy_spreads(frame, row.as_mut_ptr(), row.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(row, [0.1, 0.2, 0.3]);
    let mut energy = [0.0f64; 1];
    assert_eq!(
        unsafe { rkr_frame_copy_atom_energies(frame, energy.as_mut_ptr(), energy.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((energy[0] - 4.5).abs() < 1e-12);
    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 0,
        cols: 0,
        dtype_code: 0,
        dtype_bits: 0,
    };
    assert_eq!(
        unsafe { rkr_frame_velocities_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(view.n, 1);
    assert_eq!(view.cols, 3);
    unsafe { free_rkr_frame(frame) };
}
