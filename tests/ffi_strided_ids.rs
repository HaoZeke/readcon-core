// Copyright (c) readcon developers
//! Atom ids stored as a matrix column are not one contiguous slice.

use std::ffi::CString;

use readcon_core::ffi::*;
use readcon_core::types::ConFrame;

#[test]
fn strided_atom_ids_copy_the_column_values() {
    let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let frame = unsafe { rkr_read_nth_frame(path.as_ptr(), 0) };
    assert!(!frame.is_null());
    let raw = unsafe { &mut *(frame as *mut ConFrame) };
    let ids: Vec<u64> = raw.atom_data.iter().map(|atom| atom.atom_id).collect();
    let mut grid = Vec::with_capacity(ids.len() * 2);
    for id in &ids {
        grid.push(*id);
        grid.push(0);
    }
    let table = ndarray::Array2::from_shape_vec((ids.len(), 2), grid).unwrap();
    let column = table.index_axis_move(ndarray::Axis(1), 0);
    assert!(column.as_slice_memory_order().is_none());
    raw.atom_ids = column.into_shared();
    let mut copied = vec![0u64; ids.len()];
    assert_eq!(
        unsafe { rkr_frame_copy_atom_ids(frame, copied.as_mut_ptr(), copied.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(copied, ids);
    unsafe { free_rkr_frame(frame) };
}
