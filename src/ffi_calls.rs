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
        let disp = [0.0_f64, 0.0, 0.0];
        let _ = rkr_frame_builder_set_last_displacement(builder, disp.as_ptr());
        let _ = rkr_frame_builder_set_last_spread(builder, disp.as_ptr());
        let _ = rkr_frame_builder_set_last_energy(builder, 0.2);
        let _ = rkr_frame_builder_set_atom_position(builder, 0, 0.1, 0.2, 0.3);
        let _ = rkr_frame_builder_set_atom_velocity(builder, 0, velocity.as_ptr());
        let _ = rkr_frame_builder_set_atom_force(builder, 0, force.as_ptr());
        let _ = rkr_frame_builder_set_atom_displacement(builder, 0, disp.as_ptr());
        let _ = rkr_frame_builder_set_atom_spread(builder, 0, disp.as_ptr());
        let _ = rkr_frame_builder_set_atom_energy(builder, 0, 0.2);
        let _ = rkr_frame_builder_set_atom_fixed(builder, 0, false, false, false);
        let _ = rkr_frame_builder_set_atom_mass(builder, 0, 63.546);
        let _ = rkr_frame_builder_set_atom_id(builder, 0, 7);
        let _ = rkr_frame_builder_set_atom_position(builder, 50, 0.0, 0.0, 0.0);
        let flat = [0.1_f64, 0.2, 0.3, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let _ = rkr_frame_builder_set_positions_from_flat(builder, flat.as_ptr(), flat.len());
        let _ = rkr_frame_builder_set_forces_from_flat(builder, flat.as_ptr(), flat.len());
        let _ = rkr_frame_builder_set_displacements_from_flat(builder, flat.as_ptr(), flat.len());
        let _ = rkr_frame_builder_set_spreads_from_flat(builder, flat.as_ptr(), flat.len());
        let energies = [0.1_f64, 0.2, 0.3];
        let _ = rkr_frame_builder_set_atom_energies_from_flat(builder, energies.as_ptr(), energies.len());
        let mut got = [0.0_f64; 3];
        let mut present = false;
        let _ = rkr_frame_builder_get_atom_position(builder, 0, got.as_mut_ptr());
        let _ = rkr_frame_builder_get_atom_velocity(builder, 0, got.as_mut_ptr(), &mut present);
        let _ = rkr_frame_builder_get_atom_force(builder, 0, got.as_mut_ptr(), &mut present);
        let _ = rkr_frame_builder_get_atom_displacement(builder, 0, got.as_mut_ptr(), &mut present);
        let _ = rkr_frame_builder_get_atom_spread(builder, 0, got.as_mut_ptr(), &mut present);
        let mut energy = 0.0_f64;
        let _ = rkr_frame_builder_get_atom_energy(builder, 0, &mut energy, &mut present);
        let mut mass = 0.0_f64;
        let _ = rkr_frame_builder_get_atom_mass(builder, 0, &mut mass);
        let mut tensor = ptr::null_mut();
        let _ = rkr_frame_builder_positions_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_positions_dlpack_ex(builder, ptr::null(), &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_velocities_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_forces_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_displacements_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_spreads_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_atom_energies_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_masses_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_atom_ids_dlpack(builder, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_builder_velocities_data(builder);
        let _ = rkr_frame_builder_forces_data(builder);
        let _ = rkr_frame_builder_displacements_data(builder);
        let _ = rkr_frame_builder_spreads_data(builder);
        let _ = rkr_frame_builder_atom_energies_data(builder);
        let _ = rkr_frame_builder_masses_data(builder);
        let _ = rkr_frame_builder_atom_ids_data(builder);
        let key = CString::new("note").expect("key");
        let value = CString::new("round").expect("value");
        let _ = rkr_frame_builder_set_con_spec_minor(builder, 1);
        let _ = rkr_frame_builder_set_scalar_metadata(builder, key.as_ptr(), 1.0);
        let _ = rkr_frame_builder_set_string_metadata(builder, key.as_ptr(), value.as_ptr());
        let _ = rkr_frame_builder_set_neb_bead(builder, 1);
        let _ = rkr_frame_builder_set_neb_band(builder, 2);
        let _ = rkr_frame_add_atom_full(
            builder,
            symbol.as_ptr(),
            0.2,
            0.2,
            0.2,
            false,
            false,
            false,
            4,
            63.546,
            velocity.as_ptr(),
            force.as_ptr(),
        );
        let _ = rkr_frame_add_atom_with_fixed_mask(
            builder, symbol.as_ptr(), 0.3, 0.0, 0.0, true, false, false, 5, 63.546,
        );
        let _ = rkr_frame_add_atom_with_velocity_and_forces(
            builder,
            symbol.as_ptr(),
            0.4,
            0.0,
            0.0,
            false,
            6,
            63.546,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let cloned = rkr_frame_builder_clone(builder);
        free_rkr_frame_builder(cloned);
        let mut view = RKRArrayView {
            data: ptr::null(),
            n: 0,
            cols: 0,
            dtype_code: 0,
            dtype_bits: 0,
        };
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        let mut positions = [0.0_f64; 24];
        let _ = rkr_frame_copy_positions(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_velocities(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_forces(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_masses(frame, positions.as_mut_ptr(), positions.len());
        let mut ids = [0_u64; 8];
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
        let _ = rkr_symbol_to_z(symbol.as_ptr());
        let _ = rkr_z_to_symbol(29);
        let _ = rkr_frame_atom_index_by_id(frame, 1);
        let _ = rkr_frame_spec_version(frame);
        let _ = rkr_frame_index_energy(frame);
        let _ = rkr_frame_fmax(frame);
        let _ = rkr_frame_sections_mask(frame);
        let _ = rkr_frame_index_natoms(frame);
        let proj = rkr_frame_index_projection_json(frame);
        rkr_free_string(proj);
        let kind = rkr_frame_potential_type(frame);
        rkr_free_string(kind);
        let _ = rkr_frame_frame_index(frame);
        let _ = rkr_frame_time(frame);
        let _ = rkr_frame_timestep(frame);
        let _ = rkr_frame_neb_bead(frame);
        let _ = rkr_frame_neb_band(frame);
        let _ = rkr_frame_bond_count(frame);
        let mut header = [0u8; 256];
        let _ = rkr_frame_get_header_line(
            frame,
            false,
            0,
            header.as_mut_ptr().cast(),
            header.len(),
        );
        let header_line = rkr_frame_get_header_line_cpp(frame, false, 0);
        rkr_free_string(header_line);
        let mut n_out = 0_usize;
        let _ = rkr_frame_xyz_f64(frame, &mut n_out);
        let _ = rkr_frame_velocities_f64(frame, &mut n_out);
        let _ = rkr_frame_forces_f64(frame, &mut n_out);
        let _ = rkr_frame_displacements_f64(frame, &mut n_out);
        let _ = rkr_frame_spreads_f64(frame, &mut n_out);
        let _ = rkr_frame_displacements_view(frame, &mut view);
        let _ = rkr_frame_spreads_view(frame, &mut view);
        let _ = rkr_frame_energies_view(frame, &mut view);
        let short = [0.0_f64; 1];
        let _ = rkr_frame_copy_positions(frame, short.as_ptr() as *mut f64, 1);
        let _ = rkr_frame_copy_displacements(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_spreads(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_copy_atom_energies(frame, positions.as_mut_ptr(), positions.len());
        let _ = rkr_frame_velocities_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_forces_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_displacements_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_spreads_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let _ = rkr_frame_atom_energies_dlpack(frame, &mut tensor);
        rkr_dlpack_delete(tensor);
        let mut block = ptr::null_mut();
        let _ = rkr_frame_metatensor_positions_block(frame, &mut block);
        rkr_mts_block_free(block);
        let _ = rkr_frame_metatensor_velocities_block(frame, &mut block);
        rkr_mts_block_free(block);
        let _ = rkr_frame_metatensor_forces_block(frame, &mut block);
        rkr_mts_block_free(block);
        let _ = rkr_frame_metatensor_atom_energies_block(frame, &mut block);
        rkr_mts_block_free(block);
        let mut n_frames = 0_usize;
        let all = rkr_read_all_frames(name.as_ptr(), &mut n_frames);
        free_rkr_frame_array(all, n_frames);
        let all_threads = rkr_read_all_frames_n_threads(name.as_ptr(), &mut n_frames, 2);
        free_rkr_frame_array(all_threads, n_frames);
        let file_iter = read_con_file_iterator(name.as_ptr());
        let stepped = con_frame_iterator_next(file_iter);
        free_rkr_frame(stepped);
        free_con_frame_iterator(file_iter);
        let traj = rkr_trajectory_open(name.as_ptr(), 1);
        let _ = rkr_trajectory_len(traj);
        let mut traj_frame = ptr::null_mut();
        let _ = rkr_trajectory_read_frame(traj, 0, &mut traj_frame);
        free_rkr_frame(traj_frame);
        rkr_trajectory_free(traj);
        let gz_path = dir.path().join("round.con.gz");
        let gz_name = CString::new(gz_path.to_str().expect("utf8")).expect("gz");
        let gzip = create_writer_gzip_c(gz_name.as_ptr());
        if !gzip.is_null() {
            let _ = rkr_writer_set_canonical(gzip, 1);
            let _ = rkr_writer_is_canonical(gzip);
            let _ = rkr_writer_extend(gzip, &frame_ptr, 1);
            let _ = rkr_writer_flush(gzip);
            free_rkr_writer(gzip);
        }
        #[cfg(feature = "zstd")]
        {
            let zst_path = dir.path().join("round.con.zst");
            let zst_name = CString::new(zst_path.to_str().expect("utf8")).expect("zst");
            let zstd = create_writer_zstd_c(zst_name.as_ptr());
            if !zstd.is_null() {
                let _ = rkr_writer_extend(zstd, &frame_ptr, 1);
                let _ = rkr_writer_flush(zstd);
                free_rkr_writer(zstd);
            }
        }
        let _ = rkr_has_chemfiles_support();
        let _ = rkr_has_parallel_support();
        let _ = rkr_read_chemfiles_first(name.as_ptr());
        free_rkr_frame(frame);
    }
}
