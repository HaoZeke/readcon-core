// Copyright (c) readcon developers
//! Live handles with one null argument, so each later null check runs.

use std::ffi::{CStr, CString};
use std::ptr;

use readcon_core::ffi::*;

fn null_pointer() -> RKRStatus {
    RKRStatus::RKR_STATUS_NULL_POINTER
}

fn builder() -> *mut RKRConFrameBuilder {
    let cell = [10.0f64, 10.0, 10.0];
    let angles = [90.0f64, 90.0, 90.0];
    let handle = unsafe { rkr_frame_new(cell.as_ptr(), angles.as_ptr(), ptr::null(), ptr::null(), ptr::null(), ptr::null()) };
    assert!(!handle.is_null());
    handle
}

#[test]
fn each_later_null_argument_is_rejected() {
    let cell = [8.0f64, 8.0, 8.0];
    assert!(unsafe { rkr_frame_new(cell.as_ptr(), ptr::null(), ptr::null(), ptr::null(), ptr::null(), ptr::null()) }.is_null());

    let bare = builder();
    assert!(unsafe { rkr_frame_builder_masses_data(bare) }.is_null());
    assert!(unsafe { rkr_frame_builder_atom_ids_data(bare) }.is_null());
    let symbol = CString::new("Cu").unwrap();
    assert_eq!(
        unsafe { rkr_frame_add_atom(bare, symbol.as_ptr(), 0.0, 0.0, 0.0, false, 1, 63.546) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(unsafe { rkr_frame_builder_atom_count(bare) }, 1);

    assert_eq!(
        unsafe { rkr_frame_builder_set_last_velocity(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_force(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_displacement(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_spread(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_velocity(bare, 0, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_force(bare, 0, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_displacement(bare, 0, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_spread(bare, 0, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_positions_from_flat(bare, ptr::null(), 3) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_forces_from_flat(bare, ptr::null(), 3) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_displacements_from_flat(bare, ptr::null(), 3) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_spreads_from_flat(bare, ptr::null(), 3) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_energies_from_flat(bare, ptr::null(), 1) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_position(bare, 0, ptr::null_mut()) },
        null_pointer()
    );
    let mut xyz = [0.0f64; 3];
    let mut has = false;
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_velocity(bare, 0, ptr::null_mut(), &mut has) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_velocity(bare, 0, xyz.as_mut_ptr(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_force(bare, 0, ptr::null_mut(), &mut has) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_displacement(bare, 0, ptr::null_mut(), &mut has) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_spread(bare, 0, ptr::null_mut(), &mut has) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_energy(bare, 0, xyz.as_mut_ptr(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_force(bare, 0, xyz.as_mut_ptr(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_displacement(bare, 0, xyz.as_mut_ptr(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_spread(bare, 0, xyz.as_mut_ptr(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_energy(bare, 0, ptr::null_mut(), &mut has) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_mass(bare, 0, ptr::null_mut()) },
        null_pointer()
    );

    let mut tensor = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_builder_positions_dlpack_ex(bare, ptr::null(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_velocities_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_forces_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_displacements_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_spreads_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_energies_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_masses_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack(bare, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_velocities_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_builder_forces_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_builder_displacements_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_builder_spreads_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_energies_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert!(unsafe { rkr_frame_builder_velocities_data(bare) }.is_null());
    assert!(unsafe { rkr_frame_builder_forces_data(bare) }.is_null());
    assert!(unsafe { rkr_frame_builder_displacements_data(bare) }.is_null());
    assert!(unsafe { rkr_frame_builder_spreads_data(bare) }.is_null());
    assert!(unsafe { rkr_frame_builder_atom_energies_data(bare) }.is_null());

    let row = [1.0f64, 2.0, 3.0];
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_velocity(bare, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_force(bare, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_displacement(bare, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_spread(bare, row.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_energy(bare, 0, 4.5) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!unsafe { rkr_frame_builder_velocities_data(bare) }.is_null());
    assert!(!unsafe { rkr_frame_builder_forces_data(bare) }.is_null());
    assert!(!unsafe { rkr_frame_builder_displacements_data(bare) }.is_null());
    assert!(!unsafe { rkr_frame_builder_spreads_data(bare) }.is_null());
    assert!(!unsafe { rkr_frame_builder_atom_energies_data(bare) }.is_null());
    assert_eq!(
        unsafe { rkr_frame_builder_velocities_dlpack(bare, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(tensor) };
    let float_ids = RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_FLOAT,
            bits: 64,
            lanes: 1,
        },
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CPU,
            device_id: 0,
        },
    };
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack_ex(bare, &float_ids, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(tensor) };
    let uint32 = RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_UINT,
            bits: 32,
            lanes: 1,
        },
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CPU,
            device_id: 0,
        },
    };
    let wide_lanes = RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_UINT,
            bits: 64,
            lanes: 2,
        },
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CPU,
            device_id: 0,
        },
    };
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack_ex(bare, &uint32, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(tensor) };
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack_ex(bare, &wide_lanes, &mut tensor) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );

    let meta = CString::new(r#"{"bonds":[{"i":0,"j":0},{"i":0,"j":0,"order":2}]}"#).unwrap();
    assert_eq!(
        unsafe { rkr_frame_builder_set_metadata_json(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_metadata_json(bare, meta.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let units = CString::new(
        r#"{"length":"A","energy":"ev","mass":"amu","time":"(amu*angstrom^2/eV)^0.5"}"#,
    )
    .unwrap();
    assert_eq!(
        unsafe { rkr_frame_builder_set_units(bare, ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_units(bare, units.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let key = CString::new("note").unwrap();
    assert_eq!(
        unsafe { rkr_frame_builder_set_scalar_metadata(bare, ptr::null(), 1.0) },
        null_pointer()
    );
    let value = CString::new("cu").unwrap();
    assert_eq!(
        unsafe { rkr_frame_builder_set_string_metadata(bare, ptr::null(), value.as_ptr()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_string_metadata(bare, key.as_ptr(), ptr::null()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_string_metadata(bare, key.as_ptr(), value.as_ptr()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );

    let frame = unsafe { rkr_frame_builder_build(bare) };
    assert!(!frame.is_null());

    let mut bi = 0u32;
    let mut bj = 0u32;
    let mut has_order = 0u8;
    let mut order = 0i32;
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 0, ptr::null_mut(), &mut bj, &mut has_order, &mut order) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 0, &mut bi, ptr::null_mut(), &mut has_order, &mut order) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 0, &mut bi, &mut bj, ptr::null_mut(), &mut order) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 0, &mut bi, &mut bj, &mut has_order, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 0, &mut bi, &mut bj, &mut has_order, &mut order) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(has_order, 0);
    assert_eq!(
        unsafe { rkr_frame_bond_at(frame, 1, &mut bi, &mut bj, &mut has_order, &mut order) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(has_order, 1);
    assert_eq!(order, 2);

    let mut line = [0i8; 8];
    assert_eq!(
        unsafe { rkr_frame_get_header_line(frame, false, 0, ptr::null_mut(), line.len()) },
        null_pointer()
    );

    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_forces_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_displacements_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_spreads_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_atom_energies_dlpack(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_dlpack(frame, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(tensor) };

    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 0,
        cols: 0,
        dtype_code: 0,
        dtype_bits: 0,
    };
    assert_eq!(
        unsafe { rkr_frame_velocities_view(frame, ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(view.n, 1);
    let mut copied = [0.0f64; 3];
    assert_eq!(
        unsafe { rkr_frame_copy_velocities(frame, copied.as_mut_ptr(), copied.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(copied, row);
    let mut ids = [0u64; 1];
    assert_eq!(
        unsafe { rkr_frame_copy_atom_ids(frame, ptr::null_mut(), 1) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(ids[0], 1);

    let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let iter = unsafe { read_con_file_iterator(path.as_ptr()) };
    assert!(!iter.is_null());
    let mut yielded = ptr::null_mut();
    assert_eq!(
        unsafe { con_frame_iterator_next_status(ptr::null_mut(), &mut yielded) },
        null_pointer() as i32
    );
    assert_eq!(
        unsafe { con_frame_iterator_next_status(iter, ptr::null_mut()) },
        null_pointer() as i32
    );
    assert_eq!(
        unsafe { con_frame_iterator_next_status(iter, &mut yielded) },
        RKRStatus::RKR_STATUS_SUCCESS as i32
    );
    assert!(!yielded.is_null());
    unsafe { free_rkr_frame(yielded) };
    assert_eq!(
        unsafe { con_frame_iterator_next_status(iter, &mut yielded) },
        RKRStatus::RKR_STATUS_END_OF_FILE as i32
    );
    unsafe { free_con_frame_iterator(iter) };

    let junk = CString::new("this is not a con frame\n").unwrap();
    let bad = unsafe { read_con_string_iterator(junk.as_ptr()) };
    assert!(!bad.is_null());
    assert_eq!(
        unsafe { con_frame_iterator_next_status(bad, &mut yielded) },
        RKRStatus::RKR_STATUS_IO_ERROR as i32
    );
    let err = unsafe { CStr::from_ptr(con_frame_iterator_error_message(bad)) };
    assert!(err.to_bytes().len() > 4, "{err:?}");
    assert_eq!(
        unsafe { con_frame_iterator_next_status(bad, &mut yielded) },
        RKRStatus::RKR_STATUS_END_OF_FILE as i32
    );
    unsafe { free_con_frame_iterator(bad) };

    assert!(unsafe { read_con_buffer_iterator(ptr::null(), 4) }.is_null());
    let multi = CString::new("resources/test/tiny_multi_cuh2.con").unwrap();
    let multi_it = unsafe { read_con_file_iterator(multi.as_ptr()) };
    assert!(!multi_it.is_null());
    let skipped = unsafe { con_frame_iterator_nth(multi_it, 5) };
    assert!(skipped.is_null());
    unsafe { free_con_frame_iterator(multi_it) };

    let dir = std::env::temp_dir().join(format!("readcon-ffi-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    let out = dir.join("out.con");
    let out_c = CString::new(out.to_str().unwrap()).unwrap();
    let writer = unsafe { create_writer_from_path_c(out_c.as_ptr()) };
    assert!(!writer.is_null());
    assert_eq!(
        unsafe { rkr_writer_extend(writer, ptr::null(), 0) },
        null_pointer()
    );
    let null_frame: *const RKRConFrame = ptr::null();
    assert_eq!(
        unsafe { rkr_writer_extend(writer, &null_frame, 1) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_writer_extend_binding(writer, ptr::null(), 2) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_writer_extend_binding(writer, ptr::null(), 0) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let null_mut_frame: *mut RKRConFrame = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_writer_extend_binding(writer, &null_mut_frame, 1) },
        null_pointer()
    );
    let handle: *const RKRConFrame = frame;
    assert_eq!(
        unsafe { rkr_writer_extend(writer, &handle, 1) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { free_rkr_writer(writer) };

    let mut nframes = 0usize;
    let text = CString::new("Cu 0 0 0\n").unwrap();
    assert!(unsafe { rkr_read_chemfiles_memory(text.as_ptr(), ptr::null(), &mut nframes) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_memory(text.as_ptr(), c"XYZ".as_ptr(), ptr::null_mut()) }.is_null());
    let mut count = 0usize;
    assert!(unsafe { rkr_read_all_frames(path.as_ptr(), ptr::null_mut()) }.is_null());
    let frames = unsafe { rkr_read_all_frames(path.as_ptr(), &mut count) };
    assert!(!frames.is_null());
    assert_eq!(count, 1);
    unsafe { free_rkr_frame_array(frames, count) };
    assert!(unsafe { rkr_read_all_frames_n_threads(path.as_ptr(), ptr::null_mut(), 1) }.is_null());
    let auto = unsafe { rkr_read_all_frames_n_threads(path.as_ptr(), &mut count, 0) };
    assert!(!auto.is_null());
    assert_eq!(count, 1);
    let boxed = vec![ptr::null_mut::<*mut RKRConFrame>()].into_boxed_slice();
    let raw = Box::into_raw(boxed) as *mut *mut RKRConFrame;
    unsafe { free_rkr_frame_array(raw, 1) };
    unsafe { free_rkr_frame_array(auto, count) };

    let quiet = unsafe { read_con_file_iterator(path.as_ptr()) };
    let no_err = unsafe { CStr::from_ptr(con_frame_iterator_error_message(quiet)) };
    assert_eq!(no_err.to_str().unwrap(), "no error");
    let null_msg = unsafe { CStr::from_ptr(con_frame_iterator_error_message(ptr::null_mut())) };
    assert_eq!(null_msg.to_str().unwrap(), "null iterator");
    unsafe { free_con_frame_iterator(quiet) };
    let held = unsafe { read_con_string_iterator(junk.as_ptr()) };
    assert_eq!(
        unsafe { con_frame_iterator_next_status(held, &mut yielded) },
        RKRStatus::RKR_STATUS_IO_ERROR as i32
    );
    unsafe { free_con_frame_iterator(held) };

    let traj = unsafe { rkr_trajectory_open(path.as_ptr(), 0) };
    assert!(!traj.is_null());
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(traj, 0, ptr::null_mut()) },
        null_pointer()
    );
    unsafe { rkr_trajectory_free(traj) };

    assert_eq!(
        unsafe { rkr_frame_atom_ids_view(frame, ptr::null_mut()) },
        null_pointer()
    );
    let mut exported = ptr::null_mut();
    assert_eq!(
        unsafe {
            rkr_frame_positions_as_dlpack(
                frame,
                rkr_dl_device_type::RKR_DL_CUDA,
                0,
                0,
                1,
                0,
                &mut exported,
            )
        },
        RKRStatus::RKR_STATUS_FEATURE_DISABLED
    );
    assert_eq!(
        unsafe {
            rkr_frame_positions_as_dlpack(frame, 99, 0, 0, 1, 0, &mut exported)
        },
        RKRStatus::RKR_STATUS_FEATURE_DISABLED
    );
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(frame, &mut exported) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!exported.is_null());
    let bad_utf = [0xffu8, 0];
    assert!(
        unsafe { rkr_read_chemfiles_first(bad_utf.as_ptr() as *const std::ffi::c_char) }.is_null()
    );
    let blob = [1u8, 2, 3, 4];
    assert_eq!(
        unsafe { rkr_unpack_rcso_natoms(blob.as_ptr(), blob.len(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_positions(blob.as_ptr(), blob.len(), ptr::null_mut(), 1) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_forces(blob.as_ptr(), blob.len(), ptr::null_mut(), 1) },
        null_pointer()
    );
    let managed = unsafe { &mut *exported };
    managed.dl_tensor.data = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(frame, exported) },
        null_pointer()
    );
    let values = [9.0f64, 8.0, 7.0];
    managed.dl_tensor.data = values.as_ptr() as *mut _;
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(frame, exported) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut got = [0.0f64; 3];
    assert_eq!(
        unsafe { rkr_frame_copy_positions(frame, got.as_mut_ptr(), got.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((got[0] - 9.0).abs() < 1e-12);
    let mut shape = unsafe { std::slice::from_raw_parts(managed.dl_tensor.shape, 2).to_vec() };
    shape[1] = 2;
    managed.dl_tensor.shape = shape.as_mut_ptr();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(frame, exported) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    managed.dl_tensor.device.device_type =
        unsafe { std::mem::transmute(rkr_dl_device_type::RKR_DL_CUDA) };
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(frame, exported) },
        RKRStatus::RKR_STATUS_FEATURE_DISABLED
    );

    unsafe { rkr_dlpack_delete(exported) };

    let tiny = unsafe { rkr_read_nth_frame(path.as_ptr(), 0) };
    assert!(!tiny.is_null());
    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 0,
        cols: 0,
        dtype_code: 0,
        dtype_bits: 0,
    };
    let mut buf = [0.0f64; 12];
    for (present, sample) in [(frame, true), (tiny, false)] {
        assert_eq!(
            unsafe { rkr_frame_forces_view(present, &mut view) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_displacements_view(present, &mut view) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_spreads_view(present, &mut view) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_energies_view(present, &mut view) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_copy_forces(present, buf.as_mut_ptr(), buf.len()) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_copy_displacements(present, buf.as_mut_ptr(), buf.len()) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_copy_spreads(present, buf.as_mut_ptr(), buf.len()) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
        assert_eq!(
            unsafe { rkr_frame_copy_atom_energies(present, buf.as_mut_ptr(), buf.len()) },
            if sample { RKRStatus::RKR_STATUS_SUCCESS } else { RKRStatus::RKR_STATUS_SECTION_ABSENT }
        );
    }
    let mut energy_tensor = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_atom_energies_dlpack(frame, &mut energy_tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(energy_tensor) };
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack_ex(frame, ptr::null(), ptr::null_mut()) },
        null_pointer()
    );
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(frame, ptr::null()) },
        null_pointer()
    );

    let bad = [0xffu8, 0];
    let bad_ptr = bad.as_ptr() as *const std::ffi::c_char;
    let mut nchem = 0usize;
    assert!(unsafe { rkr_read_chemfiles(bad_ptr, &mut nchem) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_nth(bad_ptr, 0) }.is_null());
    assert_eq!(unsafe { rkr_chemfiles_nsteps(bad_ptr) }, usize::MAX);
    assert!(unsafe { rkr_read_chemfiles(path.as_ptr(), ptr::null_mut()) }.is_null());
    assert!(
        unsafe {
            rkr_read_chemfiles_range(bad_ptr, 0, 1, 1, ptr::null(), 0, &mut nchem)
        }
        .is_null()
    );
    assert!(
        unsafe {
            rkr_read_chemfiles_range(path.as_ptr(), 0, 1, usize::MAX, ptr::null(), 0, &mut nchem)
        }
        .is_null()
    );
    let empty_topo = CString::new("").unwrap();
    assert!(
        unsafe {
            rkr_read_chemfiles_range(
                path.as_ptr(),
                0,
                1,
                1,
                empty_topo.as_ptr(),
                0,
                &mut nchem,
            )
        }
        .is_null()
    );
    assert!(unsafe { rkr_read_chemfiles_memory(bad_ptr, c"XYZ".as_ptr(), &mut nchem) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_memory(text.as_ptr(), bad_ptr, &mut nchem) }.is_null());

    let mut need = 0usize;
    assert_eq!(
        unsafe { rkr_pack_rcso(frame, ptr::null_mut(), 0, &mut need) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut packed = vec![0u8; need];
    assert_eq!(
        unsafe { rkr_pack_rcso(frame, packed.as_mut_ptr(), packed.len(), &mut need) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut nat = 0u32;
    assert_eq!(
        unsafe { rkr_unpack_rcso_natoms(packed.as_ptr(), packed.len(), &mut nat) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut wide = vec![0.0f64; (nat as usize + 1) * 3];
    assert_eq!(
        unsafe { rkr_unpack_rcso_positions(packed.as_ptr(), packed.len(), wide.as_mut_ptr(), nat + 1) },
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_forces(packed.as_ptr(), packed.len(), wide.as_mut_ptr(), nat + 1) },
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
    );

    use readcon_core::chemfiles_selection::{SelectionMatch, SelectionResult};
    let empty = SelectionResult {
        selection: String::new(),
        context_size: 1,
        matches: Vec::new(),
    };
    let empty_h = Box::into_raw(Box::new(empty)) as *mut RKRSelectionResult;
    let mut atoms = [0u64; 4];
    assert_eq!(
        unsafe { rkr_selection_result_match_at(empty_h, 0, ptr::null_mut(), ptr::null_mut()) },
        null_pointer()
    );
    let mut written = 0u64;
    assert_eq!(
        unsafe { rkr_selection_result_primary_indices(empty_h, ptr::null_mut(), 0, &mut written) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(written, 0);
    unsafe { rkr_selection_result_free(empty_h) };
    let one = SelectionResult {
        selection: "all".into(),
        context_size: 1,
        matches: vec![SelectionMatch { size: 1, atoms: [0, 0, 0, 0] }],
    };
    let mut one_h = Box::into_raw(Box::new(one)) as *mut RKRSelectionResult;
    assert_eq!(
        unsafe { rkr_selection_result_match_at(one_h, 0, atoms.as_mut_ptr(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(atoms[0], 0);
    assert_eq!(
        unsafe { rkr_selection_result_primary_indices(one_h, ptr::null_mut(), 1, ptr::null_mut()) },
        null_pointer()
    );
    unsafe { rkr_selection_result_free(one_h) };
    assert_eq!(
        unsafe { rkr_frame_select(frame, ptr::null(), &mut one_h) },
        null_pointer()
    );
    let query = CString::new("name Cu").unwrap();
    assert_eq!(
        unsafe { rkr_frame_select(frame, query.as_ptr(), ptr::null_mut()) },
        null_pointer()
    );

    let mut ingest = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(tiny, &mut ingest) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let ingest_ref = unsafe { &mut *ingest };
    let natoms = unsafe { rkr_frame_atom_count(tiny) };
    let f32s = vec![1.0f32; natoms * 3];
    ingest_ref.dl_tensor.dtype.bits = 32;
    ingest_ref.dl_tensor.data = f32s.as_ptr() as *mut _;
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut one_d = [(natoms * 3) as i64];
    let flat = vec![4.0f64; natoms * 3];
    ingest_ref.dl_tensor.dtype.bits = 64;
    ingest_ref.dl_tensor.ndim = 1;
    ingest_ref.dl_tensor.shape = one_d.as_mut_ptr();
    ingest_ref.dl_tensor.data = flat.as_ptr() as *mut _;
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut bad_rows = [99i64, 3];
    ingest_ref.dl_tensor.ndim = 2;
    ingest_ref.dl_tensor.shape = bad_rows.as_mut_ptr();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    let mut host_shape = [natoms as i64, 3];
    ingest_ref.dl_tensor.dtype.bits = 16;
    ingest_ref.dl_tensor.ndim = 2;
    ingest_ref.dl_tensor.shape = host_shape.as_mut_ptr();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    ingest_ref.dl_tensor.dtype.code = unsafe { std::mem::transmute(rkr_dl_type_code::RKR_DL_INT) };
    ingest_ref.dl_tensor.dtype.bits = 32;
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    ingest_ref.dl_tensor.dtype.code = unsafe { std::mem::transmute(rkr_dl_type_code::RKR_DL_FLOAT) };
    ingest_ref.dl_tensor.dtype.bits = 64;
    ingest_ref.dl_tensor.ndim = 3;
    let mut cube = [1i64, 1, 3];
    ingest_ref.dl_tensor.shape = cube.as_mut_ptr();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    ingest_ref.dl_tensor.shape = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    unsafe {
        let raw = &mut *(tiny as *mut readcon_core::types::ConFrame);
        raw.positions = readcon_core::storage_dtype::FloatArray2::zeros_f64(0, 3);
    }
    let mut row_shape = [natoms as i64, 3];
    ingest_ref.dl_tensor.ndim = 2;
    ingest_ref.dl_tensor.shape = row_shape.as_mut_ptr();
    ingest_ref.dl_tensor.data = flat.as_ptr() as *mut _;
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(tiny, ingest) },
        RKRStatus::RKR_STATUS_VALIDATION_ERROR
    );
    ingest_ref.deleter = None;
    unsafe { rkr_dlpack_delete(ingest) };

    assert!(
        unsafe {
            rkr_read_chemfiles_range(path.as_ptr(), 0, 1, 1, ptr::null(), 0, ptr::null_mut())
        }
        .is_null()
    );
    assert!(
        unsafe {
            rkr_read_chemfiles_range(path.as_ptr(), 0, 1, 2, ptr::null(), 0, &mut nchem)
        }
        .is_null()
    );
    let bare_pack = unsafe { rkr_read_nth_frame(path.as_ptr(), 0) };
    assert!(!bare_pack.is_null());
    let mut bare_need = 0usize;
    assert_eq!(
        unsafe { rkr_pack_rcso(bare_pack, ptr::null_mut(), 0, &mut bare_need) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut bare_bytes = vec![0u8; bare_need];
    assert_eq!(
        unsafe { rkr_pack_rcso(bare_pack, bare_bytes.as_mut_ptr(), bare_bytes.len(), &mut bare_need) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut bare_nat = 0u32;
    assert_eq!(
        unsafe { rkr_unpack_rcso_natoms(bare_bytes.as_ptr(), bare_bytes.len(), &mut bare_nat) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut bare_dest = vec![0.0f64; bare_nat as usize * 3];
    assert_eq!(
        unsafe {
            rkr_unpack_rcso_forces(
                bare_bytes.as_ptr(),
                bare_bytes.len(),
                bare_dest.as_mut_ptr(),
                bare_nat,
            )
        },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    unsafe { free_rkr_frame(bare_pack) };

    unsafe { free_rkr_frame(tiny) };
    unsafe { free_rkr_frame(frame) };
    std::fs::remove_dir_all(&dir).unwrap();
}
