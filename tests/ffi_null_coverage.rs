//! Calls into the C exports the default suite leaves cold.

use std::ffi::CString;
use std::ptr;

use readcon_core::ffi::*;

#[test]
fn every_export_accepts_a_null_argument() {
    let _ = unsafe { rkr_abi_version_major() };
    let _ = unsafe { rkr_abi_version_minor() };
    let _ = unsafe { rkr_abi_layout_revision() };
    let _ = unsafe { rkr_abi_stamp() };
    let _ = unsafe { rkr_con_spec_version() };
    let _ = unsafe { rkr_library_version() };
    let _ = unsafe { rkr_frame_atom_index_by_id(ptr::null(), 0u64) };
    let _ = unsafe { rkr_symbol_to_z(ptr::null()) };
    let _ = unsafe { rkr_z_to_symbol(0u64) };
    let _ = unsafe { rkr_frame_spec_version(ptr::null()) };
    let _ = unsafe { rkr_frame_metadata_json(ptr::null()) };
    let _ = unsafe { rkr_frame_energy(ptr::null()) };
    let _ = unsafe { rkr_frame_index_energy(ptr::null()) };
    let _ = unsafe { rkr_frame_composition_formula(ptr::null()) };
    let _ = unsafe { rkr_frame_total_mass(ptr::null()) };
    let _ = unsafe { rkr_frame_cell_volume(ptr::null()) };
    let _ = unsafe { rkr_frame_fmax(ptr::null()) };
    let _ = unsafe { rkr_frame_sections_mask(ptr::null()) };
    let _ = unsafe { rkr_frame_index_natoms(ptr::null()) };
    let _ = unsafe { rkr_frame_index_projection_json(ptr::null()) };
    let _ = unsafe { rkr_frame_potential_type(ptr::null()) };
    let _ = unsafe { rkr_frame_frame_index(ptr::null()) };
    let _ = unsafe { rkr_frame_time(ptr::null()) };
    let _ = unsafe { rkr_frame_timestep(ptr::null()) };
    let _ = unsafe { rkr_frame_neb_bead(ptr::null()) };
    let _ = unsafe { rkr_frame_neb_band(ptr::null()) };
    let _ = unsafe { rkr_frame_bond_count(ptr::null()) };
    let _ = unsafe {
        rkr_frame_bond_at(
            ptr::null(),
            0u64,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    let _ = unsafe { rkr_last_error_message() };
    let _ = unsafe { read_con_file_iterator(ptr::null()) };
    let _ = unsafe { read_con_string_iterator(ptr::null()) };
    let _ = unsafe { read_con_buffer_iterator(ptr::null(), 0usize) };
    let _ = unsafe { con_frame_iterator_next(ptr::null_mut()) };
    let _ = unsafe { con_frame_iterator_forward(ptr::null_mut()) };
    let _ = unsafe { con_frame_iterator_skip(ptr::null_mut(), 0usize) };
    let _ = unsafe { con_frame_iterator_nth(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_count_frames(ptr::null()) };
    let _ = unsafe { rkr_read_nth_frame(ptr::null(), 0usize) };
    let _ = unsafe { rkr_trajectory_open(ptr::null(), 0i32) };
    let _ = unsafe { rkr_trajectory_len(ptr::null()) };
    let _ = unsafe { rkr_trajectory_read_frame(ptr::null(), 0usize, ptr::null_mut()) };
    let _ = unsafe { rkr_trajectory_free(ptr::null_mut()) };
    let _ = unsafe { free_rkr_frame(ptr::null_mut()) };
    let _ = unsafe { free_con_frame_iterator(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_to_c_frame(ptr::null()) };
    let _ = unsafe { free_c_frame(ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_get_header_line(ptr::null(), false, 0usize, ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_get_header_line_cpp(ptr::null(), false, 0usize) };
    let _ = unsafe { rkr_free_string(ptr::null_mut()) };
    let _ = unsafe { create_writer_from_path_c(ptr::null()) };
    let _ = unsafe { create_writer_from_path_round_trip_c(ptr::null()) };
    let _ = unsafe { rkr_writer_flush(ptr::null_mut()) };
    let _ = unsafe { free_rkr_writer(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_binding_rewrite(ptr::null()) };
    let _ = unsafe { rkr_writer_extend_binding(ptr::null_mut(), ptr::null(), 0usize) };
    let _ = unsafe { rkr_writer_extend(ptr::null_mut(), ptr::null(), 0usize) };
    let _ = unsafe { rkr_writer_set_canonical(ptr::null_mut(), 0u8) };
    let _ = unsafe { rkr_writer_is_canonical(ptr::null()) };
    let _ = unsafe { create_writer_from_path_with_precision_c(ptr::null(), 0u8) };
    let _ = unsafe { rkr_frame_builder_set_last_velocity(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_last_force(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_last_displacement(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_last_spread(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_last_energy(ptr::null_mut(), 0.0f64) };
    let _ = unsafe { rkr_frame_builder_atom_count(ptr::null()) };
    let _ = unsafe {
        rkr_frame_builder_set_atom_position(ptr::null_mut(), 0usize, 0.0f64, 0.0f64, 0.0f64)
    };
    let _ = unsafe { rkr_frame_builder_set_atom_velocity(ptr::null_mut(), 0usize, ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_atom_force(ptr::null_mut(), 0usize, ptr::null()) };
    let _ =
        unsafe { rkr_frame_builder_set_atom_displacement(ptr::null_mut(), 0usize, ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_atom_spread(ptr::null_mut(), 0usize, ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_atom_energy(ptr::null_mut(), 0usize, 0.0f64) };
    let _ =
        unsafe { rkr_frame_builder_set_atom_fixed(ptr::null_mut(), 0usize, false, false, false) };
    let _ = unsafe { rkr_frame_builder_set_atom_mass(ptr::null_mut(), 0usize, 0.0f64) };
    let _ = unsafe { rkr_frame_builder_set_atom_id(ptr::null_mut(), 0usize, 0u64) };
    let _ = unsafe { rkr_frame_builder_clear_atom_velocity(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_builder_clear_atom_force(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_builder_clear_atom_displacement(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_builder_clear_atom_spread(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_builder_clear_atom_energy(ptr::null_mut(), 0usize) };
    let _ =
        unsafe { rkr_frame_builder_set_positions_from_flat(ptr::null_mut(), ptr::null(), 0usize) };
    let _ = unsafe { rkr_frame_builder_set_forces_from_flat(ptr::null_mut(), ptr::null(), 0usize) };
    let _ = unsafe {
        rkr_frame_builder_set_displacements_from_flat(ptr::null_mut(), ptr::null(), 0usize)
    };
    let _ =
        unsafe { rkr_frame_builder_set_spreads_from_flat(ptr::null_mut(), ptr::null(), 0usize) };
    let _ = unsafe {
        rkr_frame_builder_set_atom_energies_from_flat(ptr::null_mut(), ptr::null(), 0usize)
    };
    let _ = unsafe { rkr_frame_builder_get_atom_position(ptr::null(), 0usize, ptr::null_mut()) };
    let _ = unsafe {
        rkr_frame_builder_get_atom_velocity(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
    };
    let _ = unsafe {
        rkr_frame_builder_get_atom_force(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
    };
    let _ = unsafe {
        rkr_frame_builder_get_atom_displacement(
            ptr::null(),
            0usize,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    let _ = unsafe {
        rkr_frame_builder_get_atom_spread(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
    };
    let _ = unsafe {
        rkr_frame_builder_get_atom_energy(ptr::null(), 0usize, ptr::null_mut(), ptr::null_mut())
    };
    let _ = unsafe { rkr_frame_builder_get_atom_mass(ptr::null(), 0usize, ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_positions_dlpack(ptr::null(), ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_builder_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_velocities_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe {
        rkr_frame_builder_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
    };
    let _ = unsafe { rkr_frame_builder_forces_dlpack(ptr::null(), ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_builder_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_displacements_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe {
        rkr_frame_builder_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
    };
    let _ = unsafe { rkr_frame_builder_spreads_dlpack(ptr::null(), ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_builder_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_atom_energies_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe {
        rkr_frame_builder_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut())
    };
    let _ = unsafe { rkr_frame_builder_masses_dlpack(ptr::null(), ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_builder_masses_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_atom_ids_dlpack(ptr::null(), ptr::null_mut()) };
    let _ =
        unsafe { rkr_frame_builder_atom_ids_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_positions_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_velocities_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_forces_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_displacements_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_spreads_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_atom_energies_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_masses_data(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_atom_ids_data(ptr::null()) };
    let _ = unsafe {
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
    };
    let _ = unsafe {
        rkr_frame_new(
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    let _ = unsafe { rkr_frame_builder_set_metadata_json(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_units(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_con_spec_minor(ptr::null_mut(), 0u32) };
    let _ = unsafe { rkr_frame_builder_set_scalar_metadata(ptr::null_mut(), ptr::null(), 0.0f64) };
    let _ =
        unsafe { rkr_frame_builder_set_string_metadata(ptr::null_mut(), ptr::null(), ptr::null()) };
    let _ = unsafe { rkr_frame_builder_set_energy(ptr::null_mut(), 0.0f64) };
    let _ = unsafe { rkr_frame_builder_set_frame_index(ptr::null_mut(), 0u64) };
    let _ = unsafe { rkr_frame_builder_set_time(ptr::null_mut(), 0.0f64) };
    let _ = unsafe { rkr_frame_builder_set_timestep(ptr::null_mut(), 0.0f64) };
    let _ = unsafe { rkr_frame_builder_set_neb_bead(ptr::null_mut(), 0u64) };
    let _ = unsafe { rkr_frame_builder_set_neb_band(ptr::null_mut(), 0u64) };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe {
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
    };
    let _ = unsafe { rkr_frame_builder_build(ptr::null_mut()) };
    let _ = unsafe { free_rkr_frame_builder(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_builder_clone(ptr::null()) };
    let _ = unsafe { create_writer_gzip_c(ptr::null()) };
    let _ = unsafe { create_writer_gzip_with_precision_c(ptr::null(), 0u8) };
    let _ = unsafe { create_writer_zstd_c(ptr::null()) };
    let _ = unsafe { create_writer_zstd_with_precision_c(ptr::null(), 0u8) };
    let _ = unsafe { rkr_read_first_frame(ptr::null()) };
    let _ = unsafe { rkr_read_all_frames(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_read_all_frames_n_threads(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { free_rkr_frame_array(ptr::null_mut(), 0usize) };
    let _ = unsafe { free_rkr_frame_ptr_array(ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_mts_block_free(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_positions_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_velocities_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_forces_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_atom_energies_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { create_writer_zstd_c(ptr::null()) };
    let _ = unsafe { create_writer_zstd_with_precision_c(ptr::null(), 0u8) };
    let _ = unsafe { rkr_mts_block_free(ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_positions_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_velocities_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_forces_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_metatensor_atom_energies_block(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_atom_count(ptr::null()) };
    let _ = unsafe { rkr_frame_xyz_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_velocities_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_forces_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_has_displacements(ptr::null()) };
    let _ = unsafe { rkr_frame_displacements_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_has_spreads(ptr::null()) };
    let _ = unsafe { rkr_frame_spreads_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_energies_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_masses_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_atom_ids_view(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_xyz_f64(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_velocities_f64(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_forces_f64(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_displacements_f64(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_spreads_f64(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_copy_positions(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_velocities(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_forces(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_displacements(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_spreads(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_atom_energies(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_masses(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe { rkr_frame_copy_atom_ids(ptr::null(), ptr::null_mut(), 0usize) };
    let _ = unsafe {
        rkr_frame_positions_as_dlpack(ptr::null(), 0i32, 0i32, 0i64, 0u32, 0u32, ptr::null_mut())
    };
    let _ = unsafe { rkr_frame_positions_from_dlpack(ptr::null_mut(), ptr::null()) };
    let _ = unsafe { rkr_frame_positions_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_velocities_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_forces_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_displacements_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_spreads_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_atom_energies_dlpack(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_frame_select(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_selection_result_match_count(ptr::null()) };
    let _ = unsafe { rkr_selection_result_context_size(ptr::null()) };
    let _ = unsafe {
        rkr_selection_result_match_at(ptr::null(), 0u64, ptr::null_mut(), ptr::null_mut())
    };
    let _ = unsafe {
        rkr_selection_result_primary_indices(ptr::null(), ptr::null_mut(), 0u64, ptr::null_mut())
    };
    let _ = unsafe { rkr_selection_result_free(ptr::null_mut()) };
    let _ = unsafe { rkr_has_chemfiles_support() };
    let _ = unsafe { rkr_has_parallel_support() };
    let _ = unsafe { rkr_read_chemfiles_first(ptr::null()) };
    let _ = unsafe { rkr_read_chemfiles(ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_read_chemfiles_nth(ptr::null(), 0usize) };
    let _ = unsafe { rkr_chemfiles_nsteps(ptr::null()) };
    let _ = unsafe {
        rkr_read_chemfiles_range(
            ptr::null(),
            0usize,
            0usize,
            0usize,
            ptr::null(),
            0u8,
            ptr::null_mut(),
        )
    };
    let _ = unsafe { rkr_read_chemfiles_memory(ptr::null(), ptr::null(), ptr::null_mut()) };
    let _ = unsafe { rkr_dlpack_delete(ptr::null_mut()) };
    let _ = unsafe { rkr_pack_rcso(ptr::null(), ptr::null_mut(), 0usize, ptr::null_mut()) };
    let _ = unsafe { rkr_unpack_rcso_natoms(ptr::null(), 0usize, ptr::null_mut()) };
    let _ = unsafe { rkr_unpack_rcso_positions(ptr::null(), 0usize, ptr::null_mut(), 0u32) };
    let _ = unsafe { rkr_unpack_rcso_forces(ptr::null(), 0usize, ptr::null_mut(), 0u32) };

    for status in [
        RKRStatus::RKR_STATUS_SUCCESS,
        RKRStatus::RKR_STATUS_NULL_POINTER,
        RKRStatus::RKR_STATUS_INVALID_UTF8,
        RKRStatus::RKR_STATUS_INVALID_JSON,
        RKRStatus::RKR_STATUS_IO_ERROR,
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS,
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL,
        RKRStatus::RKR_STATUS_INTERNAL_ERROR,
        RKRStatus::RKR_STATUS_SECTION_ABSENT,
        RKRStatus::RKR_STATUS_VALIDATION_ERROR,
        RKRStatus::RKR_STATUS_SELECTION_ERROR,
        RKRStatus::RKR_STATUS_FEATURE_DISABLED,
        RKRStatus::RKR_STATUS_DEVICE_MISMATCH,
        RKRStatus::RKR_STATUS_DEVICE_ALLOC_UNSUPPORTED,
    ] {
        let _ = rkr_status_message(status);
    }
    let _ = rkr_last_error_message();
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
    let _ = unsafe { rkr_frame_xyz_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_velocities_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_forces_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_displacements_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_spreads_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_energies_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_masses_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_atom_ids_view(frame, &mut view) };
    let _ = unsafe { rkr_frame_has_displacements(frame) };
    let _ = unsafe { rkr_frame_has_spreads(frame) };
    let mut n = 0usize;
    let _ = unsafe { rkr_frame_xyz_f64(frame, &mut n) };
    let _ = unsafe { rkr_frame_velocities_f64(frame, &mut n) };
    let _ = unsafe { rkr_frame_forces_f64(frame, &mut n) };
    let _ = unsafe { rkr_frame_displacements_f64(frame, &mut n) };
    let _ = unsafe { rkr_frame_spreads_f64(frame, &mut n) };
    let mut buf = vec![0.0_f64; 64];
    let _ = unsafe { rkr_frame_copy_positions(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_velocities(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_forces(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_displacements(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_spreads(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_atom_energies(frame, buf.as_mut_ptr(), buf.len()) };
    let _ = unsafe { rkr_frame_copy_masses(frame, buf.as_mut_ptr(), buf.len()) };
    let mut ids = vec![0_u64; 16];
    let _ = unsafe { rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len()) };
    let mut line = vec![0_u8; 64];
    let _ = unsafe {
        rkr_frame_get_header_line(frame, true, 0, line.as_mut_ptr() as *mut _, line.len())
    };
    let _ = unsafe { rkr_frame_get_header_line(frame, true, 1, line.as_mut_ptr() as *mut _, 0) };
    let _ = unsafe {
        rkr_frame_get_header_line(frame, false, 9, line.as_mut_ptr() as *mut _, line.len())
    };
    let owned = unsafe { rkr_frame_get_header_line_cpp(frame, true, 0) };
    unsafe { rkr_free_string(owned) };
    let missing = unsafe { rkr_frame_get_header_line_cpp(frame, false, 9) };
    unsafe { rkr_free_string(missing) };
    let mut tensor = ptr::null_mut();
    let _ = unsafe { rkr_frame_positions_dlpack(frame, &mut tensor) };
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
    for opts in [ptr::null(), &cpu as *const _, &cuda, &lanes, &other] {
        let _ = unsafe { rkr_frame_positions_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_velocities_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_forces_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_displacements_dlpack(frame, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_displacements_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_spreads_dlpack(frame, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_spreads_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
        let _ = unsafe { rkr_frame_atom_energies_dlpack_ex(frame, opts, &mut tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
    }
    let rewritten = unsafe { rkr_frame_binding_rewrite(frame) };
    unsafe { free_rkr_frame(rewritten) };
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.con");
    let out_c = CString::new(out.to_str().unwrap()).unwrap();
    let writer = unsafe { create_writer_from_path_c(out_c.as_ptr()) };
    assert!(!writer.is_null());
    let _ = unsafe { rkr_writer_set_canonical(writer, 1) };
    let _ = unsafe { rkr_writer_is_canonical(writer) };
    let handles = [frame as *const _];
    let _ = unsafe { rkr_writer_extend(writer, handles.as_ptr(), 1) };
    let _ = unsafe { rkr_writer_flush(writer) };
    unsafe { free_rkr_writer(writer) };
    let gz = dir.path().join("out.con.gz");
    let gz_c = CString::new(gz.to_str().unwrap()).unwrap();
    let gzip = unsafe { create_writer_gzip_c(gz_c.as_ptr()) };
    unsafe { free_rkr_writer(gzip) };
    let zst = dir.path().join("out.con.zst");
    let zst_c = CString::new(zst.to_str().unwrap()).unwrap();
    let zstd = unsafe { create_writer_zstd_with_precision_c(zst_c.as_ptr(), 6) };
    unsafe { free_rkr_writer(zstd) };
    let round = unsafe { create_writer_from_path_round_trip_c(out_c.as_ptr()) };
    let mut_handles = [frame];
    let _ = unsafe { rkr_writer_extend_binding(round, mut_handles.as_ptr(), 1) };
    unsafe { free_rkr_writer(round) };
    let mut count = 0usize;
    let all = unsafe { rkr_read_all_frames(path.as_ptr(), &mut count) };
    unsafe { free_rkr_frame_array(all, count) };
    let threaded = unsafe { rkr_read_all_frames_n_threads(vel.as_ptr(), &mut count, 2) };
    unsafe { free_rkr_frame_array(threaded, count) };
    let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
    let text_c = CString::new(text.clone()).unwrap();
    let iter = unsafe { read_con_string_iterator(text_c.as_ptr()) };
    let next = unsafe { con_frame_iterator_next(iter) };
    unsafe { free_rkr_frame(next) };
    let _ = unsafe { con_frame_iterator_forward(iter) };
    let _ = unsafe { con_frame_iterator_skip(iter, 1) };
    let nth = unsafe { con_frame_iterator_nth(iter, 0) };
    unsafe { free_rkr_frame(nth) };
    unsafe { free_con_frame_iterator(iter) };
    let buf_iter = unsafe { read_con_buffer_iterator(text.as_ptr(), text.len()) };
    unsafe { free_con_frame_iterator(buf_iter) };
    let file_iter = unsafe { read_con_file_iterator(path.as_ptr()) };
    unsafe { free_con_frame_iterator(file_iter) };
    let bad =
        CString::new(b"not utf8 \xff".to_vec()).unwrap_or_else(|_| CString::new("x").unwrap());
    let _ = unsafe { read_con_string_iterator(bad.as_ptr()) };
    let mut out_block = ptr::null_mut();
    let _ = unsafe { rkr_frame_metatensor_positions_block(frame, &mut out_block) };
    let _ = unsafe { rkr_frame_metatensor_velocities_block(frame, &mut out_block) };
    let _ = unsafe { rkr_frame_metatensor_forces_block(frame, &mut out_block) };
    let _ = unsafe { rkr_frame_metatensor_atom_energies_block(frame, &mut out_block) };
    unsafe { rkr_mts_block_free(out_block) };
    let _ = unsafe { rkr_read_chemfiles_first(path.as_ptr()) };
    let _ = unsafe { rkr_read_chemfiles(path.as_ptr(), &mut count) };
    let _ = unsafe { rkr_read_chemfiles_nth(path.as_ptr(), 0) };
    let _ = unsafe { rkr_chemfiles_nsteps(path.as_ptr()) };
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
    let _ = unsafe {
        rkr_frame_bond_at(
            bonded,
            0,
            &mut bond_i,
            &mut bond_j,
            &mut has_order,
            &mut order,
        )
    };
    let _ = unsafe {
        rkr_frame_bond_at(
            bonded,
            4,
            &mut bond_i,
            &mut bond_j,
            &mut has_order,
            &mut order,
        )
    };
    let _ = unsafe { rkr_frame_positions_dlpack(bonded, &mut tensor) };
    if !tensor.is_null() {
        let _ = unsafe { rkr_frame_positions_from_dlpack(bonded, tensor) };
        unsafe { rkr_dlpack_delete(tensor) };
    }
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
        if frame.is_null() {
            continue;
        }
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
            let _ = unsafe { rkr_frame_positions_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
            let _ = unsafe { rkr_frame_velocities_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
            let _ = unsafe { rkr_frame_forces_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
            let _ = unsafe { rkr_frame_displacements_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
            let _ = unsafe { rkr_frame_spreads_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
            let _ = unsafe { rkr_frame_atom_energies_dlpack_ex(frame, &opts, &mut tensor) };
            unsafe { rkr_dlpack_delete(tensor) };
        }
        let mut buf = [0.0_f64; 1];
        let _ = unsafe { rkr_frame_copy_positions(frame, buf.as_mut_ptr(), buf.len()) };
        let _ = unsafe { rkr_frame_copy_velocities(frame, buf.as_mut_ptr(), buf.len()) };
        let _ = unsafe { rkr_frame_copy_forces(frame, buf.as_mut_ptr(), buf.len()) };
        let mut ids = [0_u64; 1];
        let _ = unsafe { rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len()) };
        unsafe { free_rkr_frame(frame) };
    }
    let path = CString::new("resources/test/tiny_multi_cuh2.con").unwrap();
    let traj = unsafe { rkr_trajectory_open(path.as_ptr(), 0) };
    let mut out = ptr::null_mut();
    let _ = unsafe { rkr_trajectory_read_frame(traj, 0, &mut out) };
    unsafe { free_rkr_frame(out) };
    let _ = unsafe { rkr_trajectory_read_frame(traj, 50, &mut out) };
    unsafe { free_rkr_frame(out) };
    unsafe { rkr_trajectory_free(traj) };
    let missing = CString::new("resources/test/does-not-exist.con").unwrap();
    let iter = unsafe { read_con_file_iterator(missing.as_ptr()) };
    unsafe { free_con_frame_iterator(iter) };
    let bytes = b"not utf8 \xff";
    let _ = unsafe { read_con_buffer_iterator(bytes.as_ptr(), bytes.len()) };

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
    let _ = unsafe {
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
    };
    let built = unsafe { rkr_frame_builder_build(handle) };
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
    let _ = unsafe { rkr_frame_velocities_dlpack_ex(built, &opts, &mut tensor) };
    unsafe { rkr_dlpack_delete(tensor) };
    let _ = unsafe { rkr_frame_forces_dlpack_ex(built, &opts, &mut tensor) };
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
    let _ = unsafe { rkr_selection_result_match_count(selected) };
    let _ = unsafe { rkr_selection_result_context_size(selected) };
    let mut atoms = [0_u64; 4];
    let mut size = 0_u32;
    let _ = unsafe { rkr_selection_result_match_at(selected, 0, atoms.as_mut_ptr(), &mut size) };
    let _ = unsafe { rkr_selection_result_match_at(selected, 3, atoms.as_mut_ptr(), &mut size) };
    let mut primary = [0_u64; 4];
    let mut written = 0_u64;
    let _ = unsafe {
        rkr_selection_result_primary_indices(selected, primary.as_mut_ptr(), 4, &mut written)
    };
    let _ = unsafe {
        rkr_selection_result_primary_indices(selected, primary.as_mut_ptr(), 0, &mut written)
    };
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
    unsafe { free_rkr_frame_array(frames, count) };
    let bad_top = CString::new(b"bad \xff".to_vec()).unwrap();
    let _ = unsafe {
        rkr_read_chemfiles_range(path.as_ptr(), 0, 1, 1, bad_top.as_ptr(), 1, &mut count)
    };
    let data = CString::new("H 0 0 0\n").unwrap();
    let format = CString::new("XYZ").unwrap();
    let mem = unsafe { rkr_read_chemfiles_memory(data.as_ptr(), format.as_ptr(), &mut count) };
    unsafe { free_rkr_frame_array(mem, count) };
    let sel = CString::new("name H").unwrap();
    let mut result = ptr::null_mut();
    let frame = unsafe { rkr_read_first_frame(path.as_ptr()) };
    let _ = unsafe { rkr_frame_select(frame, sel.as_ptr(), &mut result) };
    unsafe { rkr_selection_result_free(result) };
    unsafe { free_rkr_frame(frame) };
}
