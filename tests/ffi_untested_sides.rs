//! Sides the r2.30 table called unreachable because no test built the value.

use std::ffi::CString;
use std::ptr;

use readcon_core::chemfiles_selection::{SelectionMatch, SelectionResult};
use readcon_core::ffi::*;

fn one_atom() -> *mut RKRConFrame {
    let cell = [4.0_f64, 5.0, 6.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let symbol = CString::new("Cu").unwrap();
    unsafe {
        let builder = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert!(!builder.is_null());
        assert_eq!(
            rkr_frame_add_atom(builder, symbol.as_ptr(), 0.1, 0.2, 0.3, false, 1, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        frame
    }
}

fn cpu_f64() -> RKRDlpackExportOptions {
    RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_FLOAT,
            bits: 64,
            lanes: 1,
        },
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CPU,
            device_id: 0,
        },
    }
}

#[test]
fn dlpack_import_rejects_a_bad_shape_and_accepts_float32() {
    let frame = one_atom();
    let opts = cpu_f64();
    let mut tensor = ptr::null_mut();
    unsafe {
        assert_eq!(
            rkr_frame_positions_dlpack_ex(frame, &opts, &mut tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(!tensor.is_null());
        let managed = &mut *tensor;
        let shape = std::slice::from_raw_parts_mut(managed.dl_tensor.shape, 2);
        let saved0 = shape[0];
        let saved1 = shape[1];
        shape[0] = 9;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        shape[0] = saved0;
        shape[1] = 4;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        shape[1] = saved1;
        let saved_ndim = managed.dl_tensor.ndim;
        managed.dl_tensor.ndim = 3;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        managed.dl_tensor.ndim = 1;
        let flat = *managed.dl_tensor.shape;
        *managed.dl_tensor.shape = 1;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        *managed.dl_tensor.shape = flat;
        managed.dl_tensor.ndim = saved_ndim;
        let saved_bits = managed.dl_tensor.dtype.bits;
        managed.dl_tensor.dtype.bits = 16;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        managed.dl_tensor.dtype.bits = saved_bits;
        let f32s = [0.25_f32, 0.5, 0.75];
        let saved_data = managed.dl_tensor.data;
        managed.dl_tensor.data = f32s.as_ptr() as *mut std::ffi::c_void;
        managed.dl_tensor.dtype.bits = 32;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        managed.dl_tensor.data = saved_data;
        managed.dl_tensor.dtype.bits = saved_bits;
        rkr_dlpack_delete(tensor);
        let mut copied = [0.0_f64; 3];
        assert_eq!(
            rkr_frame_copy_positions(frame, copied.as_mut_ptr(), copied.len()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!((copied[0] - 0.25).abs() < 1e-6);
        assert!(rkr_frame_velocities_f64(frame, ptr::null_mut()).is_null());
        assert!(rkr_frame_forces_f64(frame, ptr::null_mut()).is_null());
        assert!(rkr_frame_displacements_f64(frame, ptr::null_mut()).is_null());
        assert!(rkr_frame_spreads_f64(frame, ptr::null_mut()).is_null());
        free_rkr_frame(frame);
    }
}

#[test]
fn chemfiles_memory_rejects_a_null_format() {
    let data = CString::new("3\n\nCu 0 0 0\n").unwrap();
    let mut nframes = 0_usize;
    unsafe {
        assert!(
            rkr_read_chemfiles_memory(data.as_ptr(), ptr::null(), &mut nframes).is_null()
        );
    }
}

#[test]
fn rcso_without_forces_reports_the_section_absent() {
    let frame = one_atom();
    unsafe {
        let mut need = 0_usize;
        assert_eq!(
            rkr_pack_rcso(frame, ptr::null_mut(), 0, &mut need),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(need > 0);
        let mut buf = vec![0_u8; need];
        assert_eq!(
            rkr_pack_rcso(frame, buf.as_mut_ptr(), buf.len(), &mut need),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let mut dest = [0.0_f64; 3];
        assert_eq!(
            rkr_unpack_rcso_forces(buf.as_ptr(), need, dest.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        free_rkr_frame(frame);
    }
}

#[test]
fn selection_result_accepts_an_empty_match_list_and_a_null_size() {
    let empty = SelectionResult {
        selection: "name Cu".to_string(),
        context_size: 1,
        matches: Vec::new(),
    };
    let empty_ptr = Box::into_raw(Box::new(empty)) as *mut RKRSelectionResult;
    let mut written = 9_u64;
    unsafe {
        assert_eq!(
            rkr_selection_result_primary_indices(empty_ptr, ptr::null_mut(), 0, &mut written),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(written, 0);
        assert_eq!(
            rkr_selection_result_primary_indices(empty_ptr, ptr::null_mut(), 0, ptr::null_mut()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        rkr_selection_result_free(empty_ptr);
    }
    let hit = SelectionResult {
        selection: "name Cu".to_string(),
        context_size: 1,
        matches: vec![SelectionMatch {
            size: 1,
            atoms: [0, 0, 0, 0],
        }],
    };
    let hit_ptr = Box::into_raw(Box::new(hit)) as *mut RKRSelectionResult;
    unsafe {
        assert_eq!(
            rkr_selection_result_match_at(hit_ptr, 0, ptr::null_mut(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut atoms = [0_u64; 4];
        assert_eq!(
            rkr_selection_result_match_at(hit_ptr, 0, atoms.as_mut_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(atoms[0], 0);
        rkr_selection_result_free(hit_ptr);
    }
}
