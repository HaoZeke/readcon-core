#[test]
fn every_export_accepts_a_null_or_a_zero() {
    unsafe {
        let _ = rkr_abi_version_major();
        let _ = rkr_abi_version_minor();
        let _ = rkr_abi_layout_revision();
        let _ = rkr_abi_stamp();
        let _ = rkr_con_spec_version();
        let _ = rkr_library_version();
        let _ = rkr_frame_atom_index_by_id(ptr::null(), 0);
        let _ = rkr_symbol_to_z(ptr::null());
        let _ = rkr_z_to_symbol(0);
        let _ = rkr_frame_spec_version(ptr::null());
        let _ = rkr_frame_metadata_json(ptr::null());
        let _ = rkr_frame_energy(ptr::null());
        let _ = rkr_frame_index_energy(ptr::null());
        let _ = rkr_frame_composition_formula(ptr::null());
        let _ = rkr_frame_total_mass(ptr::null());
        let _ = rkr_frame_cell_volume(ptr::null());
        let _ = rkr_frame_fmax(ptr::null());
        let _ = rkr_frame_sections_mask(ptr::null());
        let _ = rkr_frame_index_natoms(ptr::null());
        let _ = rkr_frame_index_projection_json(ptr::null());
        let _ = rkr_frame_potential_type(ptr::null());
        let _ = rkr_frame_frame_index(ptr::null());
        let _ = rkr_frame_time(ptr::null());
        let _ = rkr_frame_timestep(ptr::null());
        let _ = rkr_frame_neb_bead(ptr::null());
        let _ = rkr_frame_neb_band(ptr::null());
        let _ = rkr_frame_bond_count(ptr::null());
        let _ = rkr_frame_bond_at(ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        let _ = rkr_last_error_message();
        let _ = read_con_file_iterator(ptr::null());
        let _ = read_con_string_iterator(ptr::null());
        let _ = read_con_buffer_iterator(ptr::null(), 0);
        let _ = con_frame_iterator_next(ptr::null_mut());
        let _ = con_frame_iterator_forward(ptr::null_mut());
        let _ = con_frame_iterator_skip(ptr::null_mut(), 0);
        let _ = con_frame_iterator_nth(ptr::null_mut(), 0);
        let _ = rkr_count_frames(ptr::null());
        let _ = rkr_read_nth_frame(ptr::null(), 0);
        let _ = rkr_trajectory_open(ptr::null(), 0);
        let _ = rkr_trajectory_len(ptr::null());
        let _ = rkr_trajectory_read_frame(ptr::null(), 0, ptr::null_mut());
        let _ = rkr_trajectory_free(ptr::null_mut());
        let _ = free_rkr_frame(ptr::null_mut());
        let _ = free_con_frame_iterator(ptr::null_mut());
        let _ = rkr_frame_to_c_frame(ptr::null());
        let _ = free_c_frame(ptr::null_mut());
        let _ = rkr_frame_get_header_line(ptr::null(), false, 0, ptr::null_mut(), 0);
        let _ = rkr_frame_get_header_line_cpp(ptr::null(), false, 0);
        let _ = rkr_free_string(ptr::null_mut());
        let _ = create_writer_from_path_c(ptr::null());
        let _ = create_writer_from_path_round_trip_c(ptr::null());
        let _ = rkr_writer_flush(ptr::null_mut());
        let _ = free_rkr_writer(ptr::null_mut());
        let _ = rkr_frame_binding_rewrite(ptr::null());
        let _ = rkr_writer_extend_binding(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_writer_extend(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_writer_set_canonical(ptr::null_mut(), 0);
        let _ = rkr_writer_is_canonical(ptr::null());
        let _ = create_writer_from_path_with_precision_c(ptr::null(), 0);
        let _ = rkr_frame_builder_set_last_velocity(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_last_force(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_last_displacement(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_last_spread(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_last_energy(ptr::null_mut(), 0.0);
        let _ = rkr_frame_builder_atom_count(ptr::null());
        let _ = rkr_frame_builder_set_atom_position(ptr::null_mut(), 0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_builder_set_atom_velocity(ptr::null_mut(), 0, ptr::null());
        let _ = rkr_frame_builder_set_atom_force(ptr::null_mut(), 0, ptr::null());
        let _ = rkr_frame_builder_set_atom_displacement(ptr::null_mut(), 0, ptr::null());
        let _ = rkr_frame_builder_set_atom_spread(ptr::null_mut(), 0, ptr::null());
        let _ = rkr_frame_builder_set_atom_energy(ptr::null_mut(), 0, 0.0);
        let _ = rkr_frame_builder_set_atom_fixed(ptr::null_mut(), 0, false, false, false);
        let _ = rkr_frame_builder_set_atom_mass(ptr::null_mut(), 0, 0.0);
        let _ = rkr_frame_builder_set_atom_id(ptr::null_mut(), 0, 0);
        let _ = rkr_frame_builder_clear_atom_velocity(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_clear_atom_force(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_clear_atom_displacement(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_clear_atom_spread(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_clear_atom_energy(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_set_positions_from_flat(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_frame_builder_set_forces_from_flat(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_frame_builder_set_displacements_from_flat(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_frame_builder_set_spreads_from_flat(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_frame_builder_set_atom_energies_from_flat(ptr::null_mut(), ptr::null(), 0);
        let _ = rkr_frame_builder_get_atom_position(ptr::null(), 0, ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_velocity(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_force(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_displacement(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_spread(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_energy(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_mass(ptr::null(), 0, ptr::null_mut());
        let _ = rkr_frame_builder_positions_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_velocities_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_forces_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_displacements_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_spreads_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_atom_energies_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_masses_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_masses_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_atom_ids_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_atom_ids_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_positions_data(ptr::null_mut());
        let _ = rkr_frame_builder_velocities_data(ptr::null_mut());
        let _ = rkr_frame_builder_forces_data(ptr::null_mut());
        let _ = rkr_frame_builder_displacements_data(ptr::null_mut());
        let _ = rkr_frame_builder_spreads_data(ptr::null_mut());
        let _ = rkr_frame_builder_atom_energies_data(ptr::null_mut());
        let _ = rkr_frame_builder_masses_data(ptr::null_mut());
        let _ = rkr_frame_builder_atom_ids_data(ptr::null());
        let _ = rkr_frame_add_atom_full(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, false, false, 0, 0.0, ptr::null(), ptr::null());
        let _ = rkr_frame_new(ptr::null(), ptr::null(), ptr::null(), ptr::null(), ptr::null(), ptr::null());
        let _ = rkr_frame_builder_set_metadata_json(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_units(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_builder_set_con_spec_minor(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_set_scalar_metadata(ptr::null_mut(), ptr::null(), 0.0);
        let _ = rkr_frame_builder_set_string_metadata(ptr::null_mut(), ptr::null(), ptr::null());
        let _ = rkr_frame_builder_set_energy(ptr::null_mut(), 0.0);
        let _ = rkr_frame_builder_set_frame_index(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_set_time(ptr::null_mut(), 0.0);
        let _ = rkr_frame_builder_set_timestep(ptr::null_mut(), 0.0);
        let _ = rkr_frame_builder_set_neb_bead(ptr::null_mut(), 0);
        let _ = rkr_frame_builder_set_neb_band(ptr::null_mut(), 0);
        let _ = rkr_frame_add_atom(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, 0, 0.0);
        let _ = rkr_frame_add_atom_with_fixed_mask(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, false, false, 0, 0.0);
        let _ = rkr_frame_add_atom_with_velocity(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, 0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_add_atom_with_velocity_fixed_mask(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, false, false, 0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_add_atom_with_forces(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, 0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_add_atom_with_forces_fixed_mask(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, false, false, 0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_add_atom_with_velocity_and_forces(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_add_atom_with_velocity_and_forces_fixed_mask(ptr::null_mut(), ptr::null(), 0.0, 0.0, 0.0, false, false, false, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let _ = rkr_frame_builder_build(ptr::null_mut());
        let _ = free_rkr_frame_builder(ptr::null_mut());
        let _ = rkr_frame_builder_clone(ptr::null());
        let _ = create_writer_gzip_c(ptr::null());
        let _ = create_writer_gzip_with_precision_c(ptr::null(), 0);
        let _ = create_writer_zstd_c(ptr::null());
        let _ = create_writer_zstd_with_precision_c(ptr::null(), 0);
        let _ = rkr_read_first_frame(ptr::null());
        let _ = rkr_read_all_frames(ptr::null(), ptr::null_mut());
        let _ = rkr_read_all_frames_n_threads(ptr::null(), ptr::null_mut(), 0);
        let _ = free_rkr_frame_array(ptr::null_mut(), 0);
        let _ = free_rkr_frame_ptr_array(ptr::null_mut(), 0);
        let _ = rkr_mts_block_free(ptr::null_mut());
        let _ = rkr_frame_metatensor_positions_block(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_metatensor_velocities_block(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_metatensor_forces_block(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_metatensor_atom_energies_block(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_atom_count(ptr::null());
        let _ = rkr_frame_xyz_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_velocities_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_forces_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_has_displacements(ptr::null());
        let _ = rkr_frame_displacements_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_has_spreads(ptr::null());
        let _ = rkr_frame_spreads_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_energies_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_masses_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_atom_ids_view(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_xyz_f64(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_velocities_f64(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_forces_f64(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_displacements_f64(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_spreads_f64(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_copy_positions(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_velocities(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_forces(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_displacements(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_spreads(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_atom_energies(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_masses(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_copy_atom_ids(ptr::null(), ptr::null_mut(), 0);
        let _ = rkr_frame_positions_as_dlpack(ptr::null(), 0, 0, 0, 0, 0, ptr::null_mut());
        let _ = rkr_frame_positions_from_dlpack(ptr::null_mut(), ptr::null());
        let _ = rkr_frame_positions_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_positions_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_velocities_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_velocities_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_forces_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_forces_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_displacements_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_spreads_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_atom_energies_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_select(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_selection_result_match_count(ptr::null());
        let _ = rkr_selection_result_context_size(ptr::null());
        let _ = rkr_selection_result_match_at(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_selection_result_primary_indices(ptr::null(), ptr::null_mut(), 0, ptr::null_mut());
        let _ = rkr_selection_result_free(ptr::null_mut());
        let _ = rkr_has_chemfiles_support();
        let _ = rkr_has_parallel_support();
        let _ = rkr_read_chemfiles_first(ptr::null());
        let _ = rkr_read_chemfiles(ptr::null(), ptr::null_mut());
        let _ = rkr_read_chemfiles_nth(ptr::null(), 0);
        let _ = rkr_chemfiles_nsteps(ptr::null());
        let _ = rkr_read_chemfiles_range(ptr::null(), 0, 0, 0, ptr::null(), 0, ptr::null_mut());
        let _ = rkr_read_chemfiles_memory(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_dlpack_delete(ptr::null_mut());
        let _ = rkr_pack_rcso(ptr::null(), ptr::null_mut(), 0, ptr::null_mut());
        let _ = rkr_unpack_rcso_natoms(ptr::null(), 0, ptr::null_mut());
        let _ = rkr_unpack_rcso_positions(ptr::null(), 0, ptr::null_mut(), 0);
        let _ = rkr_unpack_rcso_forces(ptr::null(), 0, ptr::null_mut(), 0);
    }
}

#[test]
fn a_builder_round_trip_covers_the_frame_accessors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("round.con");
    let name = CString::new(path.to_str().expect("utf8")).expect("path");
    let symbol = CString::new("Cu").expect("symbol");
    let meta = CString::new("{\"energy\":1.5}").expect("meta");
    let selection = CString::new("name Cu").expect("selection");
    let bad_sel = CString::new("not a selection (").expect("bad");
    unsafe {
        let _ = rkr_status_message(RKRStatus::RKR_STATUS_SUCCESS);
        let cell = [4.0_f64, 5.0, 6.0];
        let angles = [90.0_f64, 90.0, 90.0];
        let builder = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            CString::new("user").unwrap().as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert!(!builder.is_null());
        assert_eq!(
            rkr_frame_add_atom(builder, symbol.as_ptr(), 0.1, 0.2, 0.3, false, 1, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_add_atom_with_velocity(
                builder,
                symbol.as_ptr(),
                1.0,
                0.0,
                0.0,
                false,
                2,
                63.546,
                0.01,
                0.0,
                0.0
            ),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_add_atom_with_forces(
                builder,
                symbol.as_ptr(),
                0.0,
                1.0,
                0.0,
                false,
                3,
                63.546,
                0.1,
                0.0,
                0.0
            ),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let velocity = [0.01_f64, 0.0, 0.0];
        let force = [0.1_f64, 0.0, 0.0];
        let _ = rkr_frame_builder_set_last_velocity(builder, velocity.as_ptr());
        let _ = rkr_frame_builder_set_last_force(builder, force.as_ptr());
        let _ = rkr_frame_builder_set_metadata_json(builder, meta.as_ptr());
        let units = CString::new("{\"length\":\"angstrom\",\"energy\":\"eV\"}").expect("units");
        let _ = rkr_frame_builder_set_units(builder, units.as_ptr());
        let _ = rkr_frame_builder_set_energy(builder, 1.5);
        let _ = rkr_frame_builder_set_time(builder, 0.2);
        let _ = rkr_frame_builder_set_timestep(builder, 0.01);
        let _ = rkr_frame_builder_set_frame_index(builder, 4);
        let _ = rkr_frame_builder_atom_count(builder);
        let _ = rkr_frame_builder_positions_data(builder);
        let mut view = RKRArrayView {
            data: ptr::null(),
            n: 0,
            cols: 0,
            dtype_code: 0,
            dtype_bits: 0,
        };
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        let mut positions = [0.0_f64; 9];
        let _ = rkr_frame_copy_positions(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_velocities(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_forces(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_masses(frame, positions.as_mut_ptr(), positions.len());
        let mut ids = [0_u64; 3];
        let _ = rkr_frame_copy_atom_ids(frame, ids.as_mut_ptr(), ids.len());
        let _ = rkr_frame_xyz_view(frame, &mut view);
        let _ = rkr_frame_velocities_view(frame, &mut view);
        let _ = rkr_frame_forces_view(frame, &mut view);
        let _ = rkr_frame_masses_view(frame, &mut view);
        let _ = rkr_frame_atom_ids_view(frame, &mut view);
        let _ = rkr_frame_has_displacements(frame);
        let _ = rkr_frame_has_spreads(frame);
        let _ = rkr_frame_energy(frame);
        let _ = rkr_frame_total_mass(frame);
        let _ = rkr_frame_cell_volume(frame);
        let _ = rkr_frame_atom_count(frame);
        let formula = rkr_frame_composition_formula(frame);
        rkr_free_string(formula);
        let meta_out = rkr_frame_metadata_json(frame);
        rkr_free_string(meta_out);
        let mut result = ptr::null_mut();
        let _ = rkr_frame_select(frame, selection.as_ptr(), &mut result);
        rkr_selection_result_free(result);
        let _ = rkr_frame_select(frame, bad_sel.as_ptr(), &mut result);
        rkr_selection_result_free(result);
        let mut tensor = ptr::null_mut();
        let _ = rkr_frame_positions_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let writer = create_writer_from_path_c(name.as_ptr());
        assert!(!writer.is_null());
        let frame_ptr = frame as *const RKRConFrame;
        let _ = rkr_writer_extend(writer, &frame_ptr, 1);
        let _ = rkr_writer_flush(writer);
        free_rkr_writer(writer);
        let loaded = rkr_read_first_frame(name.as_ptr());
        free_rkr_frame(loaded);
        let _ = rkr_count_frames(name.as_ptr());
        let nth = rkr_read_nth_frame(name.as_ptr(), 0);
        free_rkr_frame(nth);
        free_rkr_frame(frame);
    }
}
