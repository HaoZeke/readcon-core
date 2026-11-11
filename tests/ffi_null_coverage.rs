// Copyright (c) readcon-core developers
//! Calls into the C exports the default suite leaves cold.

use std::ffi::CString;
use std::ptr;

use readcon_core::ffi::*;

#[test]
fn every_export_accepts_a_null_argument() {
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert_eq!(unsafe { rkr_abi_version_minor() }, 1);
    assert_eq!(unsafe { rkr_abi_layout_revision() }, 1);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(rkr_abi_stamp()) },
        c"readcon-core/abi-1.1/layout-1"
    );
    assert_eq!(unsafe { rkr_con_spec_version() }, 3);
    let expected = std::ffi::CString::new(env!("CARGO_PKG_VERSION")).unwrap();
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(rkr_library_version()) },
        expected.as_c_str()
    );
    assert_eq!(
        unsafe { rkr_frame_atom_index_by_id(ptr::null(), 0u64) },
        u64::MAX
    );
    assert_eq!(unsafe { rkr_symbol_to_z(ptr::null()) }, 0u64);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(rkr_z_to_symbol(0u64)) },
        c"X"
    );
    assert_eq!(unsafe { rkr_frame_spec_version(ptr::null()) }, 0u32);
    assert!(unsafe { rkr_frame_metadata_json(ptr::null()) }.is_null());
    assert!(unsafe { rkr_frame_energy(ptr::null()) }.is_nan());
    assert!(unsafe { rkr_frame_index_energy(ptr::null()) }.is_nan());
    assert!(unsafe { rkr_frame_composition_formula(ptr::null()) }.is_null());
    assert!(unsafe { rkr_frame_total_mass(ptr::null()) }.is_nan());
    assert!(unsafe { rkr_frame_cell_volume(ptr::null()) }.is_nan());
    assert!(unsafe { rkr_frame_fmax(ptr::null()) }.is_nan());
    assert_eq!(unsafe { rkr_frame_sections_mask(ptr::null()) }, 0u8);
    assert_eq!(unsafe { rkr_frame_index_natoms(ptr::null()) }, 0u32);
    assert!(unsafe { rkr_frame_index_projection_json(ptr::null()) }.is_null());
    assert!(unsafe { rkr_frame_potential_type(ptr::null()) }.is_null());
    assert_eq!(unsafe { rkr_frame_frame_index(ptr::null()) }, u64::MAX);
    assert!(unsafe { rkr_frame_time(ptr::null()) }.is_nan());
    assert!(unsafe { rkr_frame_timestep(ptr::null()) }.is_nan());
    assert_eq!(unsafe { rkr_frame_neb_bead(ptr::null()) }, u64::MAX);
    assert_eq!(unsafe { rkr_frame_neb_band(ptr::null()) }, u64::MAX);
    assert_eq!(unsafe { rkr_frame_bond_count(ptr::null()) }, 0u64);
    assert_eq!(
        unsafe {
            rkr_frame_bond_at(
                ptr::null(),
                0u64,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    {
        let message = unsafe { rkr_last_error_message() };
        if !message.is_null() {
            let text = unsafe { std::ffi::CStr::from_ptr(message) }
                .to_str()
                .unwrap();
            assert!(!text.is_empty());
        }
    }
    assert!(unsafe { read_con_file_iterator(ptr::null()) }.is_null());
    assert!(unsafe { read_con_string_iterator(ptr::null()) }.is_null());
    let empty = unsafe { read_con_buffer_iterator(ptr::null(), 0usize) };
    assert!(!empty.is_null(), "an empty buffer is a live iterator");
    assert!(unsafe { con_frame_iterator_next(empty) }.is_null());
    unsafe { free_con_frame_iterator(empty) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { con_frame_iterator_next(ptr::null_mut()) }.is_null());
    assert_eq!(
        unsafe { con_frame_iterator_forward(ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { con_frame_iterator_skip(ptr::null_mut(), 0usize) },
        usize::MAX
    );
    assert!(unsafe { con_frame_iterator_nth(ptr::null_mut(), 0usize) }.is_null());
    assert_eq!(unsafe { rkr_count_frames(ptr::null()) }, usize::MAX);
    assert!(unsafe { rkr_read_nth_frame(ptr::null(), 0usize) }.is_null());
    assert!(unsafe { rkr_trajectory_open(ptr::null(), 0i32) }.is_null());
    assert_eq!(unsafe { rkr_trajectory_len(ptr::null()) }, usize::MAX);
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(ptr::null(), 0usize, ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    unsafe { rkr_trajectory_free(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    unsafe { free_rkr_frame(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    unsafe { free_con_frame_iterator(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { rkr_frame_to_c_frame(ptr::null()) }.is_null());
    unsafe { free_c_frame(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert_eq!(
        unsafe { rkr_frame_get_header_line(ptr::null(), false, 0usize, ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(unsafe { rkr_frame_get_header_line_cpp(ptr::null(), false, 0usize) }.is_null());
    unsafe { rkr_free_string(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { create_writer_from_path_c(ptr::null()) }.is_null());
    assert!(unsafe { create_writer_from_path_round_trip_c(ptr::null()) }.is_null());
    assert_eq!(
        unsafe { rkr_writer_flush(ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    unsafe { free_rkr_writer(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { rkr_frame_binding_rewrite(ptr::null()) }.is_null());
    assert_eq!(
        unsafe { rkr_writer_extend_binding(ptr::null_mut(), ptr::null(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_writer_extend(ptr::null_mut(), ptr::null(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_writer_set_canonical(ptr::null_mut(), 0u8) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(unsafe { rkr_writer_is_canonical(ptr::null()) }, 0u8);
    assert!(unsafe { create_writer_from_path_with_precision_c(ptr::null(), 0u8) }.is_null());
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_velocity(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_force(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_displacement(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_spread(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_energy(ptr::null_mut(), 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(unsafe { rkr_frame_builder_atom_count(ptr::null()) }, 0usize);
    assert_eq!(
        unsafe {
            rkr_frame_builder_set_atom_position(ptr::null_mut(), 0usize, 0.0f64, 0.0f64, 0.0f64)
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_velocity(ptr::null_mut(), 0usize, ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_force(ptr::null_mut(), 0usize, ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_displacement(ptr::null_mut(), 0usize, ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_spread(ptr::null_mut(), 0usize, ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_energy(ptr::null_mut(), 0usize, 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_fixed(ptr::null_mut(), 0usize, false, false, false) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_mass(ptr::null_mut(), 0usize, 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_atom_id(ptr::null_mut(), 0usize, 0u64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_clear_atom_velocity(ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_clear_atom_force(ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_clear_atom_displacement(ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_clear_atom_spread(ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_clear_atom_energy(ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_positions_from_flat(ptr::null_mut(), ptr::null(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_forces_from_flat(ptr::null_mut(), ptr::null(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_set_displacements_from_flat(ptr::null_mut(), ptr::null(), 0usize)
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_spreads_from_flat(ptr::null_mut(), ptr::null(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_set_atom_energies_from_flat(ptr::null_mut(), ptr::null(), 0usize)
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_position(ptr::null(), 0usize, ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_get_atom_velocity(
                ptr::null(),
                0usize,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_get_atom_force(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_get_atom_displacement(
                ptr::null(),
                0usize,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_get_atom_spread(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_get_atom_energy(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_mass(ptr::null(), 0usize, ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_positions_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_velocities_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_forces_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_displacements_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_spreads_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_energies_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_builder_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_masses_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_masses_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_atom_ids_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(unsafe { rkr_frame_builder_positions_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_velocities_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_forces_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_displacements_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_spreads_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_atom_energies_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_masses_data(ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_builder_atom_ids_data(ptr::null()) }.is_null());
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_full(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                false,
                false,
                0u64,
                0.0f64,
                ptr::null(),
                ptr::null(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(
        unsafe {
            rkr_frame_new(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
            )
        }
        .is_null()
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_metadata_json(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_units(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_con_spec_minor(ptr::null_mut(), 0u32) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_scalar_metadata(ptr::null_mut(), ptr::null(), 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_string_metadata(ptr::null_mut(), ptr::null(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_energy(ptr::null_mut(), 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_frame_index(ptr::null_mut(), 0u64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_time(ptr::null_mut(), 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_timestep(ptr::null_mut(), 0.0f64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_neb_bead(ptr::null_mut(), 0u64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_builder_set_neb_band(ptr::null_mut(), 0u64) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                0u64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_fixed_mask(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                false,
                false,
                0u64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_velocity(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_velocity_fixed_mask(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                false,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_forces(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_forces_fixed_mask(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                false,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_velocity_and_forces(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_with_velocity_and_forces_fixed_mask(
                ptr::null_mut(),
                ptr::null(),
                0.0f64,
                0.0f64,
                0.0f64,
                false,
                false,
                false,
                0u64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
                0.0f64,
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(unsafe { rkr_frame_builder_build(ptr::null_mut()) }.is_null());
    unsafe { free_rkr_frame_builder(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { rkr_frame_builder_clone(ptr::null()) }.is_null());
    assert!(unsafe { create_writer_gzip_c(ptr::null()) }.is_null());
    assert!(unsafe { create_writer_gzip_with_precision_c(ptr::null(), 0u8) }.is_null());
    assert!(unsafe { create_writer_zstd_c(ptr::null()) }.is_null());
    assert!(unsafe { create_writer_zstd_with_precision_c(ptr::null(), 0u8) }.is_null());
    assert!(unsafe { rkr_read_first_frame(ptr::null()) }.is_null());
    assert!(unsafe { rkr_read_all_frames(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(
        unsafe { rkr_read_all_frames_n_threads(ptr::null(), ptr::null_mut(), 0usize) }.is_null()
    );
    unsafe { free_rkr_frame_array(ptr::null_mut(), 0usize) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    unsafe { free_rkr_frame_ptr_array(ptr::null_mut(), 0usize) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    unsafe { rkr_mts_block_free(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    #[cfg(feature = "metatensor")]
    let mts_null = RKRStatus::RKR_STATUS_NULL_POINTER;
    #[cfg(not(feature = "metatensor"))]
    let mts_null = RKRStatus::RKR_STATUS_FEATURE_DISABLED;
    assert_eq!(
        unsafe { rkr_frame_metatensor_positions_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_velocities_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_forces_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_atom_energies_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert!(unsafe { create_writer_zstd_c(ptr::null()) }.is_null());
    assert!(unsafe { create_writer_zstd_with_precision_c(ptr::null(), 0u8) }.is_null());
    unsafe { rkr_mts_block_free(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert_eq!(
        unsafe { rkr_frame_metatensor_positions_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_velocities_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_forces_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(
        unsafe { rkr_frame_metatensor_atom_energies_block(ptr::null(), ptr::null_mut()) },
        mts_null
    );
    assert_eq!(unsafe { rkr_frame_atom_count(ptr::null()) }, 0usize);
    assert_eq!(
        unsafe { rkr_frame_xyz_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_forces_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(!unsafe { rkr_frame_has_displacements(ptr::null()) });
    assert_eq!(
        unsafe { rkr_frame_displacements_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(!unsafe { rkr_frame_has_spreads(ptr::null()) });
    assert_eq!(
        unsafe { rkr_frame_spreads_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_energies_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_masses_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_atom_ids_view(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert!(unsafe { rkr_frame_xyz_f64(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_velocities_f64(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_forces_f64(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_displacements_f64(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_frame_spreads_f64(ptr::null(), ptr::null_mut()) }.is_null());
    assert_eq!(
        unsafe { rkr_frame_copy_positions(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_velocities(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_forces(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_displacements(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_spreads(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_atom_energies(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_masses(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_copy_atom_ids(ptr::null(), ptr::null_mut(), 0usize) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_frame_positions_as_dlpack(
                ptr::null(),
                0i32,
                0i32,
                0i64,
                0u32,
                0u32,
                ptr::null_mut(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_forces_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_displacements_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_spreads_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_atom_energies_dlpack(ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_frame_select(ptr::null(), ptr::null(), ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_selection_result_match_count(ptr::null()) },
        0u64
    );
    assert_eq!(
        unsafe { rkr_selection_result_context_size(ptr::null()) },
        0u32
    );
    assert_eq!(
        unsafe {
            rkr_selection_result_match_at(ptr::null(), 0u64, ptr::null_mut(), ptr::null_mut())
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe {
            rkr_selection_result_primary_indices(
                ptr::null(),
                ptr::null_mut(),
                0u64,
                ptr::null_mut(),
            )
        },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    unsafe { rkr_selection_result_free(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    #[cfg(feature = "chemfiles")]
    assert_eq!(unsafe { rkr_has_chemfiles_support() }, 1);
    #[cfg(not(feature = "chemfiles"))]
    assert_eq!(unsafe { rkr_has_chemfiles_support() }, 0);
    // Default features leave `parallel` off, so the export is 0.
    // A build with `--features parallel` reports 1.
    #[cfg(feature = "parallel")]
    assert_eq!(unsafe { rkr_has_parallel_support() }, 1);
    #[cfg(not(feature = "parallel"))]
    assert_eq!(unsafe { rkr_has_parallel_support() }, 0);
    assert!(unsafe { rkr_read_chemfiles_first(ptr::null()) }.is_null());
    assert!(unsafe { rkr_read_chemfiles(ptr::null(), ptr::null_mut()) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_nth(ptr::null(), 0usize) }.is_null());
    assert_eq!(unsafe { rkr_chemfiles_nsteps(ptr::null()) }, usize::MAX);
    assert!(
        unsafe {
            rkr_read_chemfiles_range(
                ptr::null(),
                0usize,
                0usize,
                0usize,
                ptr::null(),
                0u8,
                ptr::null_mut(),
            )
        }
        .is_null()
    );
    assert!(
        unsafe { rkr_read_chemfiles_memory(ptr::null(), ptr::null(), ptr::null_mut()) }.is_null()
    );
    unsafe { rkr_dlpack_delete(ptr::null_mut()) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert_eq!(
        unsafe { rkr_pack_rcso(ptr::null(), ptr::null_mut(), 0usize, ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_natoms(ptr::null(), 0usize, ptr::null_mut()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_positions(ptr::null(), 0usize, ptr::null_mut(), 0u32) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    assert_eq!(
        unsafe { rkr_unpack_rcso_forces(ptr::null(), 0usize, ptr::null_mut(), 0u32) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    let messages = [
        (RKRStatus::RKR_STATUS_SUCCESS, c"success"),
        (RKRStatus::RKR_STATUS_NULL_POINTER, c"null pointer"),
        (RKRStatus::RKR_STATUS_INVALID_UTF8, c"invalid UTF-8"),
        (RKRStatus::RKR_STATUS_INVALID_JSON, c"invalid JSON"),
        (RKRStatus::RKR_STATUS_IO_ERROR, c"I/O error"),
        (
            RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS,
            c"index out of bounds",
        ),
        (RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL, c"buffer too small"),
        (RKRStatus::RKR_STATUS_INTERNAL_ERROR, c"internal error"),
        (RKRStatus::RKR_STATUS_SECTION_ABSENT, c"section absent"),
        (RKRStatus::RKR_STATUS_VALIDATION_ERROR, c"validation error"),
        (RKRStatus::RKR_STATUS_SELECTION_ERROR, c"selection error"),
        (
            RKRStatus::RKR_STATUS_FEATURE_DISABLED,
            c"feature disabled in this build",
        ),
        (
            RKRStatus::RKR_STATUS_DEVICE_MISMATCH,
            c"DLPack device mismatch",
        ),
        (
            RKRStatus::RKR_STATUS_DEVICE_ALLOC_UNSUPPORTED,
            c"device allocation unsupported in this build",
        ),
    ];
    for (status, text) in messages {
        let message = rkr_status_message(status);
        assert!(!message.is_null());
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(message) }, text);
    }
    let trailing = rkr_last_error_message();
    if !trailing.is_null() {
        let text = unsafe { std::ffi::CStr::from_ptr(trailing) }
            .to_str()
            .unwrap();
        assert!(!text.is_empty());
    }
}

#[test]
fn live_frame_covers_the_export_bodies() {
    let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let vel = CString::new("resources/test/tiny_cuh2_vel_forces.con").unwrap();
    let frame = unsafe { rkr_read_first_frame(path.as_ptr()) };
    assert!(!frame.is_null());
    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 0,
        cols: 0,
        dtype_code: 0,
        dtype_bits: 0,
    };
    assert_eq!(
        unsafe { rkr_frame_xyz_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(view.n >= 1);
    assert_eq!(
        unsafe { rkr_frame_velocities_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_forces_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_displacements_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_spreads_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_energies_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_masses_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_atom_ids_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!unsafe { rkr_frame_has_displacements(frame) });
    assert!(!unsafe { rkr_frame_has_spreads(frame) });
    let mut n = 0usize;
    assert!(!unsafe { rkr_frame_xyz_f64(frame, &mut n) }.is_null());
    assert!(n >= 3);
    assert!(unsafe { rkr_frame_velocities_f64(frame, &mut n) }.is_null());
    assert!(unsafe { rkr_frame_forces_f64(frame, &mut n) }.is_null());
    assert!(unsafe { rkr_frame_displacements_f64(frame, &mut n) }.is_null());
    assert!(unsafe { rkr_frame_spreads_f64(frame, &mut n) }.is_null());
    let mut buf = vec![0.0_f64; 64];
    assert_eq!(
        unsafe { rkr_frame_copy_positions(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_copy_velocities(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_copy_forces(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_copy_displacements(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_copy_spreads(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_copy_atom_energies(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(
        unsafe { rkr_frame_copy_masses(frame, buf.as_mut_ptr(), buf.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut ids = vec![0_u64; 16];
    assert_eq!(
        unsafe { rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut line = vec![0_u8; 64];
    assert_eq!(
        unsafe {
            rkr_frame_get_header_line(frame, true, 0, line.as_mut_ptr() as *mut _, line.len())
        },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_frame_get_header_line(frame, true, 1, line.as_mut_ptr() as *mut _, 0) },
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
    );
    assert_eq!(
        unsafe {
            rkr_frame_get_header_line(frame, false, 9, line.as_mut_ptr() as *mut _, line.len())
        },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    let owned = unsafe { rkr_frame_get_header_line_cpp(frame, true, 0) };
    assert!(!owned.is_null());
    unsafe { rkr_free_string(owned) };
    let missing = unsafe { rkr_frame_get_header_line_cpp(frame, false, 9) };
    assert!(missing.is_null());
    let mut tensor = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(frame, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    let cpu = RKRDlpackExportOptions {
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
    let cuda = RKRDlpackExportOptions {
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CUDA,
            device_id: 0,
        },
        ..cpu
    };
    let lanes = RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_FLOAT,
            bits: 64,
            lanes: 2,
        },
        ..cpu
    };
    let other = RKRDlpackExportOptions {
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CUDA_HOST,
            device_id: 0,
        },
        ..cpu
    };
    let expect = |opts: *const RKRDlpackExportOptions, present: bool| -> RKRStatus {
        if opts.is_null() {
            return if present {
                RKRStatus::RKR_STATUS_SUCCESS
            } else {
                RKRStatus::RKR_STATUS_SECTION_ABSENT
            };
        }
        let opts = unsafe { *opts };
        if opts.dtype.lanes != 1 {
            return RKRStatus::RKR_STATUS_VALIDATION_ERROR;
        }
        if opts.device.device_type != rkr_dl_device_type::RKR_DL_CPU {
            return RKRStatus::RKR_STATUS_FEATURE_DISABLED;
        }
        if present {
            RKRStatus::RKR_STATUS_SUCCESS
        } else {
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        }
    };
    for opts in [ptr::null(), &cpu as *const _, &cuda, &lanes, &other] {
        assert_eq!(
            unsafe { rkr_frame_positions_dlpack_ex(frame, opts, &mut tensor) },
            expect(opts, true)
        );
        if !tensor.is_null() {
            unsafe { rkr_dlpack_delete(tensor) };
        }
        for present in [false] {
            assert_eq!(
                unsafe { rkr_frame_velocities_dlpack_ex(frame, opts, &mut tensor) },
                expect(opts, present)
            );
            assert_eq!(
                unsafe { rkr_frame_forces_dlpack_ex(frame, opts, &mut tensor) },
                expect(opts, present)
            );
            assert_eq!(
                unsafe { rkr_frame_displacements_dlpack_ex(frame, opts, &mut tensor) },
                expect(opts, present)
            );
            assert_eq!(
                unsafe { rkr_frame_spreads_dlpack_ex(frame, opts, &mut tensor) },
                expect(opts, present)
            );
            assert_eq!(
                unsafe { rkr_frame_atom_energies_dlpack_ex(frame, opts, &mut tensor) },
                expect(opts, present)
            );
        }
        assert_eq!(
            unsafe { rkr_frame_displacements_dlpack(frame, &mut tensor) },
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            unsafe { rkr_frame_spreads_dlpack(frame, &mut tensor) },
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
    }
    let rewritten = unsafe { rkr_frame_binding_rewrite(frame) };
    assert!(!rewritten.is_null());
    unsafe { free_rkr_frame(rewritten) };
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.con");
    let out_c = CString::new(out.to_str().unwrap()).unwrap();
    let writer = unsafe { create_writer_from_path_c(out_c.as_ptr()) };
    assert!(!writer.is_null());
    assert_eq!(
        unsafe { rkr_writer_set_canonical(writer, 1) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(unsafe { rkr_writer_is_canonical(writer) }, 1);
    let handles = [frame as *const _];
    assert_eq!(
        unsafe { rkr_writer_extend(writer, handles.as_ptr(), 1) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(
        unsafe { rkr_writer_flush(writer) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { free_rkr_writer(writer) };
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
    let gz = dir.path().join("out.con.gz");
    let gz_c = CString::new(gz.to_str().unwrap()).unwrap();
    let gzip = unsafe { create_writer_gzip_c(gz_c.as_ptr()) };
    assert!(!gzip.is_null());
    unsafe { free_rkr_writer(gzip) };
    let zst = dir.path().join("out.con.zst");
    let zst_c = CString::new(zst.to_str().unwrap()).unwrap();
    let zstd = unsafe { create_writer_zstd_with_precision_c(zst_c.as_ptr(), 6) };
    // Without the zstd feature the writer is null and the last error
    // says compression is not built. With the feature the pointer is live.
    #[cfg(feature = "zstd")]
    {
        assert!(!zstd.is_null());
        unsafe { free_rkr_writer(zstd) };
    }
    #[cfg(not(feature = "zstd"))]
    {
        assert!(zstd.is_null());
        let message = unsafe { std::ffi::CStr::from_ptr(rkr_last_error_message()) };
        assert!(
            message.to_string_lossy().contains("compression not built"),
            "{message:?}"
        );
    }
    let round = unsafe { create_writer_from_path_round_trip_c(out_c.as_ptr()) };
    assert!(!round.is_null());
    let mut_handles = [frame];
    assert_eq!(
        unsafe { rkr_writer_extend_binding(round, mut_handles.as_ptr(), 1) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { free_rkr_writer(round) };
    let mut count = 0usize;
    let all = unsafe { rkr_read_all_frames(path.as_ptr(), &mut count) };
    assert!(!all.is_null());
    assert_eq!(count, 1);
    unsafe { free_rkr_frame_array(all, count) };
    let threaded = unsafe { rkr_read_all_frames_n_threads(vel.as_ptr(), &mut count, 2) };
    assert!(!threaded.is_null());
    assert!(count >= 1);
    unsafe { free_rkr_frame_array(threaded, count) };
    let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
    let text_c = CString::new(text.clone()).unwrap();
    let iter = unsafe { read_con_string_iterator(text_c.as_ptr()) };
    let next = unsafe { con_frame_iterator_next(iter) };
    assert!(!next.is_null());
    unsafe { free_rkr_frame(next) };
    assert_eq!(
        unsafe { con_frame_iterator_forward(iter) },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    assert_eq!(unsafe { con_frame_iterator_skip(iter, 1) }, 0);
    let nth = unsafe { con_frame_iterator_nth(iter, 0) };
    assert!(nth.is_null());
    unsafe { free_con_frame_iterator(iter) };
    let buf_iter = unsafe { read_con_buffer_iterator(text.as_ptr(), text.len()) };
    assert!(!buf_iter.is_null());
    unsafe { free_con_frame_iterator(buf_iter) };
    let file_iter = unsafe { read_con_file_iterator(path.as_ptr()) };
    assert!(!file_iter.is_null());
    unsafe { free_con_frame_iterator(file_iter) };
    let bad = CString::new(b"not utf8 \xff".to_vec()).unwrap();
    assert!(unsafe { read_con_string_iterator(bad.as_ptr()) }.is_null());
    let mut out_block = ptr::null_mut();
    #[cfg(not(feature = "metatensor"))]
    {
        assert_eq!(
            unsafe { rkr_frame_metatensor_positions_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert_eq!(
            unsafe { rkr_frame_metatensor_velocities_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert_eq!(
            unsafe { rkr_frame_metatensor_forces_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert_eq!(
            unsafe { rkr_frame_metatensor_atom_energies_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert!(out_block.is_null());
    }
    #[cfg(feature = "metatensor")]
    {
        assert_eq!(
            unsafe { rkr_frame_metatensor_positions_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(!out_block.is_null());
        unsafe { rkr_mts_block_free(out_block) };
        out_block = ptr::null_mut();
        assert_eq!(
            unsafe { rkr_frame_metatensor_velocities_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            unsafe { rkr_frame_metatensor_forces_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            unsafe { rkr_frame_metatensor_atom_energies_block(frame, &mut out_block) },
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert!(out_block.is_null());
    }
    unsafe { rkr_mts_block_free(out_block) };
    assert_eq!(unsafe { rkr_abi_version_major() }, 1);
    assert!(unsafe { rkr_read_chemfiles_first(path.as_ptr()) }.is_null());
    assert!(unsafe { rkr_read_chemfiles(path.as_ptr(), &mut count) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_nth(path.as_ptr(), 0) }.is_null());
    assert_eq!(unsafe { rkr_chemfiles_nsteps(path.as_ptr()) }, usize::MAX);
    let mut bond_i = 0_u32;
    let mut bond_j = 0_u32;
    let mut has_order = 0_u8;
    let mut order = 0_i32;
    let built = {
        let mut builder =
            readcon_core::types::ConFrameBuilder::new([8.0, 8.0, 8.0], [90.0, 90.0, 90.0]);
        builder
            .add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 1, 1.0)
            .add_bond(readcon_core::types::Bond::new(0, 0).with_order(2));
        builder.build().unwrap()
    };
    let bond_path = dir.path().join("bonds.con");
    {
        let mut writer = readcon_core::writer::ConFrameWriter::from_path(&bond_path).unwrap();
        writer.extend(std::iter::once(&built)).unwrap();
    }
    let bond_c = CString::new(bond_path.to_str().unwrap()).unwrap();
    let bonded = unsafe { rkr_read_first_frame(bond_c.as_ptr()) };
    assert!(!bonded.is_null());
    assert_eq!(unsafe { rkr_frame_bond_count(bonded) }, 1);
    assert_eq!(
        unsafe {
            rkr_frame_bond_at(
                bonded,
                0,
                &mut bond_i,
                &mut bond_j,
                &mut has_order,
                &mut order,
            )
        },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!((bond_i, bond_j, has_order, order), (0, 0, 1, 2));
    assert_eq!(
        unsafe {
            rkr_frame_bond_at(
                bonded,
                4,
                &mut bond_i,
                &mut bond_j,
                &mut has_order,
                &mut order,
            )
        },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    assert_eq!(
        unsafe { rkr_frame_positions_dlpack(bonded, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!tensor.is_null());
    assert_eq!(
        unsafe { rkr_frame_positions_from_dlpack(bonded, tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    unsafe { rkr_dlpack_delete(tensor) };
    unsafe { free_rkr_frame(bonded) };
    unsafe { free_rkr_frame(frame) };
}

#[test]
fn section_frames_cover_dtype_and_trajectory_exports() {
    let files = [
        "resources/test/tiny_cuh2_vel_forces.con",
        "resources/test/tiny_cuh2_forces.con",
        "resources/test/tiny_cuh2_displacements.con",
        "resources/test/tiny_cuh2_spreads.con",
        "resources/test/tiny_cuh2_charges_spins_magmoms.con",
    ];
    let dtypes = [
        (rkr_dl_type_code::RKR_DL_FLOAT, 32u8),
        (rkr_dl_type_code::RKR_DL_FLOAT, 64),
        (rkr_dl_type_code::RKR_DL_INT, 8),
        (rkr_dl_type_code::RKR_DL_INT, 16),
        (rkr_dl_type_code::RKR_DL_INT, 32),
        (rkr_dl_type_code::RKR_DL_INT, 64),
        (rkr_dl_type_code::RKR_DL_UINT, 8),
        (rkr_dl_type_code::RKR_DL_UINT, 16),
        (rkr_dl_type_code::RKR_DL_UINT, 32),
        (rkr_dl_type_code::RKR_DL_UINT, 64),
        (rkr_dl_type_code::RKR_DL_BOOL, 8),
        (rkr_dl_type_code::RKR_DL_COMPLEX, 64),
    ];
    for path in files {
        let c_path = CString::new(path).unwrap();
        let frame = unsafe { rkr_read_first_frame(c_path.as_ptr()) };
        assert!(!frame.is_null(), "{path}");
        let has_vel = path.contains("vel");
        let has_frc = path.contains("force");
        let has_dsp = path.contains("displacement");
        let has_spr = path.contains("spread");
        let mut tensor = ptr::null_mut();
        for (code, bits) in dtypes {
            let opts = RKRDlpackExportOptions {
                dtype: RKRDLDataType {
                    code,
                    bits,
                    lanes: 1,
                },
                device: RKRDLDevice {
                    device_type: rkr_dl_device_type::RKR_DL_CPU,
                    device_id: 0,
                },
            };
            let dtype_status = if code == rkr_dl_type_code::RKR_DL_COMPLEX {
                RKRStatus::RKR_STATUS_VALIDATION_ERROR
            } else {
                RKRStatus::RKR_STATUS_SUCCESS
            };
            let section = |present: bool| {
                if code == rkr_dl_type_code::RKR_DL_COMPLEX && present {
                    RKRStatus::RKR_STATUS_VALIDATION_ERROR
                } else if present {
                    RKRStatus::RKR_STATUS_SUCCESS
                } else {
                    RKRStatus::RKR_STATUS_SECTION_ABSENT
                }
            };
            assert_eq!(
                unsafe { rkr_frame_positions_dlpack_ex(frame, &opts, &mut tensor) },
                dtype_status,
                "{path} positions {code}/{bits}"
            );
            if !tensor.is_null() {
                unsafe { rkr_dlpack_delete(tensor) };
            }
            assert_eq!(
                unsafe { rkr_frame_velocities_dlpack_ex(frame, &opts, &mut tensor) },
                section(has_vel)
            );
            assert_eq!(
                unsafe { rkr_frame_forces_dlpack_ex(frame, &opts, &mut tensor) },
                section(has_frc)
            );
            assert_eq!(
                unsafe { rkr_frame_displacements_dlpack_ex(frame, &opts, &mut tensor) },
                section(has_dsp)
            );
            assert_eq!(
                unsafe { rkr_frame_spreads_dlpack_ex(frame, &opts, &mut tensor) },
                section(has_spr)
            );
            assert_eq!(
                unsafe { rkr_frame_atom_energies_dlpack_ex(frame, &opts, &mut tensor) },
                RKRStatus::RKR_STATUS_SECTION_ABSENT
            );
        }
        let mut buf = [0.0_f64; 1];
        assert_eq!(
            unsafe { rkr_frame_copy_positions(frame, buf.as_mut_ptr(), buf.len()) },
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        let absent_or_small = |present: bool| {
            if present {
                RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
            } else {
                RKRStatus::RKR_STATUS_SECTION_ABSENT
            }
        };
        assert_eq!(
            unsafe { rkr_frame_copy_velocities(frame, buf.as_mut_ptr(), buf.len()) },
            absent_or_small(has_vel)
        );
        assert_eq!(
            unsafe { rkr_frame_copy_forces(frame, buf.as_mut_ptr(), buf.len()) },
            absent_or_small(has_frc)
        );
        let mut ids = [0_u64; 1];
        assert_eq!(
            unsafe { rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len()) },
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        unsafe { free_rkr_frame(frame) };
    }
    let path = CString::new("resources/test/tiny_multi_cuh2.con").unwrap();
    let traj = unsafe { rkr_trajectory_open(path.as_ptr(), 0) };
    assert!(!traj.is_null());
    let mut out = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(traj, 0, &mut out) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!out.is_null());
    unsafe { free_rkr_frame(out) };
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(traj, 50, &mut out) },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    assert!(out.is_null());
    unsafe { rkr_trajectory_free(traj) };
    let missing = CString::new("resources/test/does-not-exist.con").unwrap();
    let iter = unsafe { read_con_file_iterator(missing.as_ptr()) };
    assert!(iter.is_null());
    let bytes = b"not utf8 \xff";
    assert!(unsafe { read_con_buffer_iterator(bytes.as_ptr(), bytes.len()) }.is_null());

    let handle = {
        let cell = [8.0_f64, 8.0, 8.0];
        let angles = [90.0_f64, 90.0, 90.0];
        let user = CString::new("built").unwrap();
        unsafe {
            rkr_frame_new(
                cell.as_ptr(),
                angles.as_ptr(),
                user.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
            )
        }
    };
    let symbol = CString::new("H").unwrap();
    let velocity = [0.1_f64, 0.2, 0.3];
    let force = [1.0_f64, 0.0, -1.0];
    assert_eq!(
        unsafe {
            rkr_frame_add_atom_full(
                handle,
                symbol.as_ptr(),
                0.0,
                0.0,
                0.0,
                false,
                false,
                false,
                1,
                1.0,
                velocity.as_ptr(),
                force.as_ptr(),
            )
        },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let built = unsafe { rkr_frame_builder_build(handle) };
    assert!(!built.is_null());
    let opts = RKRDlpackExportOptions {
        dtype: RKRDLDataType {
            code: rkr_dl_type_code::RKR_DL_FLOAT,
            bits: 32,
            lanes: 1,
        },
        device: RKRDLDevice {
            device_type: rkr_dl_device_type::RKR_DL_CPU,
            device_id: 0,
        },
    };
    let mut tensor = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_velocities_dlpack_ex(built, &opts, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    assert_eq!(
        unsafe { rkr_frame_forces_dlpack_ex(built, &opts, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    unsafe { free_rkr_frame(built) };

    let selected = Box::new(readcon_core::chemfiles_selection::SelectionResult {
        selection: "name H".to_string(),
        context_size: 2,
        matches: vec![readcon_core::chemfiles_selection::SelectionMatch {
            size: 2,
            atoms: [0, 1, 0, 0],
        }],
    });
    let selected = Box::into_raw(selected) as *mut RKRSelectionResult;
    assert_eq!(unsafe { rkr_selection_result_match_count(selected) }, 1);
    assert_eq!(unsafe { rkr_selection_result_context_size(selected) }, 2);
    let mut atoms = [0_u64; 4];
    let mut size = 0_u32;
    assert_eq!(
        unsafe { rkr_selection_result_match_at(selected, 0, atoms.as_mut_ptr(), &mut size) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(size, 2);
    assert_eq!(&atoms[..2], &[0, 1]);
    assert_eq!(
        unsafe { rkr_selection_result_match_at(selected, 3, atoms.as_mut_ptr(), &mut size) },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    let mut primary = [0_u64; 4];
    let mut written = 0_u64;
    assert_eq!(
        unsafe {
            rkr_selection_result_primary_indices(selected, primary.as_mut_ptr(), 4, &mut written)
        },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert_eq!(written, 1);
    assert_eq!(primary[0], 0);
    assert_eq!(
        unsafe {
            rkr_selection_result_primary_indices(selected, primary.as_mut_ptr(), 0, &mut written)
        },
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
    );
    unsafe { rkr_selection_result_free(selected) };

    let mut count = 0_usize;
    let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let topology = CString::new("resources/test/tiny_cuh2.con").unwrap();
    let frames = unsafe {
        rkr_read_chemfiles_range(
            path.as_ptr(),
            0,
            1,
            usize::MAX,
            topology.as_ptr(),
            0,
            &mut count,
        )
    };
    assert!(frames.is_null());
    let bad_top = CString::new(b"bad \xff".to_vec()).unwrap();
    assert!(
        unsafe {
            rkr_read_chemfiles_range(path.as_ptr(), 0, 1, 1, bad_top.as_ptr(), 1, &mut count)
        }
        .is_null()
    );
    let data = CString::new("H 0 0 0\n").unwrap();
    let format = CString::new("XYZ").unwrap();
    let mem = unsafe { rkr_read_chemfiles_memory(data.as_ptr(), format.as_ptr(), &mut count) };
    assert!(mem.is_null());
    let sel = CString::new("name H").unwrap();
    let mut result = ptr::null_mut();
    let frame = unsafe { rkr_read_first_frame(path.as_ptr()) };
    assert!(!frame.is_null());
    let selected = unsafe { rkr_frame_select(frame, sel.as_ptr(), &mut result) };
    #[cfg(not(feature = "chemfiles"))]
    {
        assert_eq!(selected, RKRStatus::RKR_STATUS_SELECTION_ERROR);
        assert!(result.is_null());
    }
    #[cfg(feature = "chemfiles")]
    {
        assert_eq!(selected, RKRStatus::RKR_STATUS_SUCCESS);
        assert!(!result.is_null());
        let count = unsafe { rkr_selection_result_match_count(result) };
        assert!(count >= 1);
        unsafe { rkr_selection_result_free(result) };
    }
    unsafe { free_rkr_frame(frame) };
}
