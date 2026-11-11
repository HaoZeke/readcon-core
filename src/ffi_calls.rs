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
        let _ = rkr_frame_bond_at(
            ptr::null(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
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
        let _ =
            rkr_frame_builder_get_atom_velocity(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_force(ptr::null(), 0, ptr::null_mut(), ptr::null_mut());
        let _ = rkr_frame_builder_get_atom_displacement(
            ptr::null(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        );
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
        let _ =
            rkr_frame_builder_displacements_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_spreads_dlpack(ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_spreads_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
        let _ = rkr_frame_builder_atom_energies_dlpack(ptr::null(), ptr::null_mut());
        let _ =
            rkr_frame_builder_atom_energies_dlpack_ex(ptr::null(), ptr::null(), ptr::null_mut());
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
        let _ = rkr_frame_add_atom_full(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            false,
            false,
            0,
            0.0,
            ptr::null(),
            ptr::null(),
        );
        let _ = rkr_frame_new(
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
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
        let _ = rkr_frame_add_atom_with_fixed_mask(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            false,
            false,
            0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_velocity(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_velocity_fixed_mask(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            false,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_forces(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_forces_fixed_mask(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            false,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_velocity_and_forces(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let _ = rkr_frame_add_atom_with_velocity_and_forces_fixed_mask(
            ptr::null_mut(),
            ptr::null(),
            0.0,
            0.0,
            0.0,
            false,
            false,
            false,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
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
        let _ =
            rkr_selection_result_primary_indices(ptr::null(), ptr::null_mut(), 0, ptr::null_mut());
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
        let _ = rkr_frame_builder_set_atom_energies_from_flat(
            builder,
            energies.as_ptr(),
            energies.len(),
        );
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
            builder,
            symbol.as_ptr(),
            0.3,
            0.0,
            0.0,
            true,
            false,
            false,
            5,
            63.546,
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
        let _ =
            rkr_frame_get_header_line(frame, false, 0, header.as_mut_ptr().cast(), header.len());
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
        for gzip_path in [ptr::null(), gz_name.as_ptr()] {
            let gzip = create_writer_gzip_c(gzip_path);
            if !gzip.is_null() {
                let _ = rkr_writer_set_canonical(gzip, 1);
                let _ = rkr_writer_is_canonical(gzip);
                let _ = rkr_writer_extend(gzip, &frame_ptr, 1);
                let _ = rkr_writer_flush(gzip);
                free_rkr_writer(gzip);
            } else {
                assert!(gzip.is_null());
            }
        }
        #[cfg(feature = "zstd")]
        {
            let zst_path = dir.path().join("round.con.zst");
            let zst_name = CString::new(zst_path.to_str().expect("utf8")).expect("zst");
            for zstd_path in [ptr::null(), zst_name.as_ptr()] {
                let zstd = create_writer_zstd_c(zstd_path);
                if !zstd.is_null() {
                    let _ = rkr_writer_extend(zstd, &frame_ptr, 1);
                    let _ = rkr_writer_flush(zstd);
                    free_rkr_writer(zstd);
                } else {
                    assert!(zstd.is_null());
                }
            }
        }
        let _ = rkr_has_chemfiles_support();
        let _ = rkr_has_parallel_support();
        let _ = rkr_read_chemfiles_first(name.as_ptr());
        free_rkr_frame(frame);
    }
}

#[test]
fn bonds_dlpack_import_rcso_and_force_errors() {
    let cell = [4.0_f64, 4.0, 4.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let symbol = CString::new("Cu").expect("symbol");
    let hydrogen = CString::new("H").expect("symbol");
    let bonds = CString::new(r#"{"bonds":[[0,1],{"i":1,"j":0,"order":2}]}"#).expect("bonds");
    let bad_json = CString::new("{").expect("json");
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
        let force = [0.2_f64, 0.0, 0.0];
        assert_eq!(
            rkr_frame_add_atom_with_forces(
                builder,
                symbol.as_ptr(),
                0.1,
                0.0,
                0.0,
                false,
                1,
                63.546,
                force[0],
                force[1],
                force[2]
            ),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_add_atom(builder, hydrogen.as_ptr(), 1.0, 0.0, 0.0, false, 2, 1.008),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let _ = rkr_frame_builder_set_last_force(builder, force.as_ptr());
        let _ = rkr_frame_builder_set_atom_force(builder, 0, force.as_ptr());
        let _ = rkr_frame_builder_set_atom_force(builder, 9, force.as_ptr());
        let _ = rkr_frame_builder_set_atom_force(builder, 0, ptr::null());
        let _ = rkr_frame_builder_clear_atom_force(builder, 0);
        let _ = rkr_frame_builder_clear_atom_force(builder, 9);
        let _ = rkr_frame_builder_set_metadata_json(builder, bonds.as_ptr());
        let _ = rkr_frame_builder_set_metadata_json(builder, bad_json.as_ptr());
        let bad_utf = [0xff_u8, 0];
        let _ = rkr_frame_builder_set_metadata_json(builder, bad_utf.as_ptr().cast());
        let _ = rkr_frame_builder_set_metadata_json(builder, bonds.as_ptr());
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        let _ = rkr_frame_bond_count(frame);
        let mut left = 0_u32;
        let mut right = 0_u32;
        let mut has_order = 0_u8;
        let mut order = 0_i32;
        let _ = rkr_frame_bond_at(frame, 0, &mut left, &mut right, &mut has_order, &mut order);
        let _ = rkr_frame_bond_at(frame, 1, &mut left, &mut right, &mut has_order, &mut order);
        let _ = rkr_frame_bond_at(frame, 8, &mut left, &mut right, &mut has_order, &mut order);
        let _ = rkr_frame_bond_at(
            frame,
            0,
            ptr::null_mut(),
            &mut right,
            &mut has_order,
            &mut order,
        );
        let mut tensor = ptr::null_mut();
        for live in [false, true] {
            if live {
                let _ = rkr_frame_positions_dlpack(frame, &mut tensor);
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                let _ = rkr_frame_positions_from_dlpack(ptr::null_mut(), tensor);
                let _ = rkr_frame_positions_from_dlpack(frame, ptr::null());
            }
            if !tensor.is_null() {
                let managed = &mut *tensor;
                let saved_device = managed.dl_tensor.device.device_type;
                managed.dl_tensor.device.device_type = dlpk::sys::DLDeviceType::kDLCUDA;
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                managed.dl_tensor.device.device_type = saved_device;
                let saved_shape = managed.dl_tensor.shape;
                managed.dl_tensor.shape = ptr::null_mut();
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                managed.dl_tensor.shape = saved_shape;
                for shape in [ptr::null_mut(), saved_shape] {
                    if !shape.is_null() {
                        let saved_ndim = managed.dl_tensor.ndim;
                        let saved_first = *saved_shape;
                        managed.dl_tensor.ndim = 1;
                        *saved_shape = 1;
                        let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                        managed.dl_tensor.ndim = saved_ndim;
                        *saved_shape = saved_first;
                    } else {
                        assert!(shape.is_null());
                    }
                }
                let saved_bits = managed.dl_tensor.dtype.bits;
                managed.dl_tensor.dtype.bits = 32;
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                managed.dl_tensor.dtype.bits = saved_bits;
                let saved_code = managed.dl_tensor.dtype.code;
                managed.dl_tensor.dtype.code = dlpk::sys::DLDataTypeCode::kDLInt;
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                managed.dl_tensor.dtype.code = saved_code;
                let saved_data = managed.dl_tensor.data;
                managed.dl_tensor.data = ptr::null_mut();
                let _ = rkr_frame_positions_from_dlpack(frame, tensor);
                managed.dl_tensor.data = saved_data;
                rkr_dlpack_delete(tensor);
                tensor = ptr::null_mut();
            } else {
                assert!(tensor.is_null());
            }
        }
        let mut need = 0_usize;
        let _ = rkr_pack_rcso(frame, ptr::null_mut(), 0, &mut need);
        let _ = rkr_pack_rcso(ptr::null(), ptr::null_mut(), 0, &mut need);
        let _ = rkr_pack_rcso(frame, ptr::null_mut(), 0, ptr::null_mut());
        let mut short = [0_u8; 4];
        let _ = rkr_pack_rcso(frame, short.as_mut_ptr(), short.len(), &mut need);
        let mut buf = vec![0_u8; need.max(1)];
        let _ = rkr_pack_rcso(frame, buf.as_mut_ptr(), buf.len(), &mut need);
        let mut natoms = 0_u32;
        let _ = rkr_unpack_rcso_natoms(buf.as_ptr(), need, &mut natoms);
        let _ = rkr_unpack_rcso_natoms(ptr::null(), need, &mut natoms);
        let _ = rkr_unpack_rcso_natoms(buf.as_ptr(), 4, &mut natoms);
        let mut dest = vec![0.0_f64; (natoms.max(1) as usize) * 3];
        let _ = rkr_unpack_rcso_positions(buf.as_ptr(), need, dest.as_mut_ptr(), natoms);
        let _ = rkr_unpack_rcso_positions(
            buf.as_ptr(),
            need,
            dest.as_mut_ptr(),
            natoms.saturating_add(1),
        );
        let _ = rkr_unpack_rcso_positions(ptr::null(), need, dest.as_mut_ptr(), natoms);
        let _ = rkr_unpack_rcso_forces(buf.as_ptr(), need, dest.as_mut_ptr(), natoms);
        let _ = rkr_unpack_rcso_forces(
            buf.as_ptr(),
            need,
            dest.as_mut_ptr(),
            natoms.saturating_add(1),
        );
        let _ = rkr_unpack_rcso_forces(ptr::null(), need, dest.as_mut_ptr(), natoms);
        let junk = [0_u8; 8];
        let _ = rkr_unpack_rcso_natoms(junk.as_ptr(), junk.len(), &mut natoms);
        free_rkr_frame(frame);
    }
}

#[test]
fn section_copies_and_selection_check_status() {
    let cell = [4.0_f64, 5.0, 6.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let copper = CString::new("Cu").unwrap();
    let hydrogen = CString::new("H").unwrap();
    let bare = unsafe {
        rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert!(!bare.is_null());
    unsafe {
        assert_eq!(
            rkr_frame_add_atom(bare, copper.as_ptr(), 0.1, 0.2, 0.3, false, 1, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_add_atom(bare, hydrogen.as_ptr(), 1.0, 0.0, 0.0, false, 2, 1.008),
            RKRStatus::RKR_STATUS_SUCCESS
        );
    }
    let plain = unsafe { rkr_frame_builder_build(bare) };
    assert!(!plain.is_null());
    let mut buf = [0.0_f64; 12];
    let mut small = [0.0_f64; 1];
    unsafe {
        assert_eq!(
            rkr_frame_copy_positions(ptr::null(), buf.as_mut_ptr(), 6),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_copy_positions(plain, ptr::null_mut(), 6),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_copy_positions(plain, small.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_positions(plain, buf.as_mut_ptr(), 6),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!((buf[0] - 0.1).abs() < 1e-12);
        for copy in [
            rkr_frame_copy_velocities,
            rkr_frame_copy_forces,
            rkr_frame_copy_displacements,
            rkr_frame_copy_spreads,
        ] {
            assert_eq!(
                copy(ptr::null(), buf.as_mut_ptr(), 6),
                RKRStatus::RKR_STATUS_NULL_POINTER
            );
            assert_eq!(
                copy(plain, ptr::null_mut(), 6),
                RKRStatus::RKR_STATUS_NULL_POINTER
            );
            assert_eq!(
                copy(plain, buf.as_mut_ptr(), 6),
                RKRStatus::RKR_STATUS_SECTION_ABSENT
            );
        }
        assert_eq!(
            rkr_frame_copy_atom_energies(plain, buf.as_mut_ptr(), 2),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_copy_masses(plain, buf.as_mut_ptr(), 2),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!((buf[0] - 63.546).abs() < 1e-9);
        assert_eq!(
            rkr_frame_copy_masses(plain, small.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
    }
    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 0,
        cols: 0,
        dtype_code: 0,
        dtype_bits: 0,
    };
    unsafe {
        assert_eq!(
            rkr_frame_velocities_view(plain, &mut view),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_forces_view(plain, &mut view),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_xyz_view(ptr::null(), &mut view),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_xyz_view(plain, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_xyz_view(plain, &mut view),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(view.n, 2);
        let mut n = 9_usize;
        assert!(rkr_frame_velocities_f64(plain, &mut n).is_null());
        assert_eq!(n, 0);
        assert!(!rkr_frame_xyz_f64(plain, &mut n).is_null());
        assert_eq!(n, 2);
        assert!(rkr_frame_xyz_f64(ptr::null(), &mut n).is_null());
        assert_eq!(n, 0);
    }
    let selection = CString::new("name Cu").unwrap();
    let mut rejected = ptr::null_mut();
    unsafe {
        assert_eq!(
            rkr_frame_select(plain, selection.as_ptr(), &mut rejected),
            RKRStatus::RKR_STATUS_SELECTION_ERROR
        );
        assert!(rejected.is_null());
        let bad = [0xff_u8, 0];
        assert_eq!(
            rkr_frame_select(plain, bad.as_ptr().cast(), &mut rejected),
            RKRStatus::RKR_STATUS_INVALID_UTF8
        );
        assert_eq!(
            rkr_frame_select(ptr::null(), selection.as_ptr(), &mut rejected),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
    }
    let owned = crate::chemfiles_selection::SelectionResult {
        selection: "name Cu".to_string(),
        context_size: 1,
        matches: vec![crate::chemfiles_selection::SelectionMatch {
            size: 1,
            atoms: [0, 0, 0, 0],
        }],
    };
    let result = Box::into_raw(Box::new(owned)) as *mut RKRSelectionResult;
    unsafe {
        assert_eq!(rkr_selection_result_match_count(result), 1);
        assert_eq!(rkr_selection_result_context_size(result), 1);
        let mut atoms = [9_u64; 4];
        let mut size = 0_u32;
        assert_eq!(
            rkr_selection_result_match_at(result, 0, atoms.as_mut_ptr(), &mut size),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(size, 1);
        assert_eq!(atoms[0], 0);
        assert_eq!(
            rkr_selection_result_match_at(result, 1, atoms.as_mut_ptr(), &mut size),
            RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
        );
        assert_eq!(
            rkr_selection_result_match_at(ptr::null(), 0, atoms.as_mut_ptr(), &mut size),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut written = 0_u64;
        let mut too_small = [0_u64; 1];
        assert_eq!(
            rkr_selection_result_primary_indices(result, too_small.as_mut_ptr(), 0, &mut written),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(written, 1);
        assert_eq!(
            rkr_selection_result_primary_indices(result, ptr::null_mut(), 1, &mut written),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut indices = [9_u64; 1];
        assert_eq!(
            rkr_selection_result_primary_indices(result, indices.as_mut_ptr(), 1, &mut written),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(indices[0], 0);
        rkr_selection_result_free(result);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one.con");
    let name = CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        let writer = create_writer_from_path_c(name.as_ptr());
        assert!(!writer.is_null());
        assert_eq!(rkr_writer_is_canonical(ptr::null()), 0);
        assert_eq!(
            rkr_writer_set_canonical(writer, 1),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(rkr_writer_is_canonical(writer), 1);
        let frame_ptr = plain as *const RKRConFrame;
        assert_eq!(
            rkr_writer_extend(writer, &frame_ptr, 1),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_writer_extend(writer, ptr::null(), 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let null_frame: *mut RKRConFrame = ptr::null_mut();
        assert_eq!(
            rkr_writer_extend_binding(writer, &null_frame, 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_writer_extend_binding(ptr::null_mut(), &null_frame, 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(rkr_writer_flush(writer), RKRStatus::RKR_STATUS_SUCCESS);
        free_rkr_writer(writer);
        assert!(rkr_last_error_message().is_null());
        let mut ids = [0_u64; 2];
        assert_eq!(
            rkr_frame_copy_atom_ids(ptr::null(), ids.as_mut_ptr(), 2),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_copy_atom_ids(plain, ids.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_atom_ids(plain, ids.as_mut_ptr(), 2),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(ids[0], 1);
        let mut tensor = ptr::null_mut();
        assert_eq!(
            rkr_frame_positions_as_dlpack(
                plain,
                rkr_dl_device_type::RKR_DL_CPU,
                0,
                0,
                0,
                0,
                &mut tensor
            ),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(!tensor.is_null());
        rkr_dlpack_delete(tensor);
        assert_eq!(
            rkr_frame_positions_as_dlpack(
                plain,
                rkr_dl_device_type::RKR_DL_CUDA,
                0,
                0,
                0,
                0,
                &mut tensor
            ),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert_eq!(
            rkr_frame_positions_as_dlpack(plain, 99, 0, 0, 0, 0, &mut tensor),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert_eq!(
            rkr_frame_displacements_dlpack(plain, &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_spreads_dlpack(plain, &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_displacements_dlpack(ptr::null(), &mut tensor),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut n_frames = 0_usize;
        let path = CString::new("resources/test/tiny_cuh2.con").unwrap();
        let bad = [0xff_u8, 0];
        assert!(rkr_read_chemfiles_range(
            path.as_ptr(),
            0,
            1,
            usize::MAX,
            ptr::null(),
            0,
            &mut n_frames
        )
        .is_null());
        assert!(rkr_read_chemfiles_range(
            bad.as_ptr().cast(),
            0,
            1,
            4,
            path.as_ptr(),
            1,
            &mut n_frames
        )
        .is_null());
        let data = CString::new("2\n\nCu 0 0 0\nH 1 0 0\n").unwrap();
        let format = CString::new("XYZ").unwrap();
        assert!(rkr_read_chemfiles_memory(data.as_ptr(), format.as_ptr(), &mut n_frames).is_null());
        assert!(
            rkr_read_chemfiles_memory(bad.as_ptr().cast(), format.as_ptr(), &mut n_frames)
                .is_null()
        );
        assert!(
            rkr_read_chemfiles_memory(data.as_ptr(), bad.as_ptr().cast(), &mut n_frames).is_null()
        );
        assert!(rkr_read_chemfiles(path.as_ptr(), &mut n_frames).is_null());
        assert!(rkr_read_chemfiles(bad.as_ptr().cast(), &mut n_frames).is_null());
        assert_eq!(rkr_chemfiles_nsteps(path.as_ptr()), usize::MAX);
        assert_eq!(rkr_chemfiles_nsteps(bad.as_ptr().cast()), usize::MAX);
        let topology = CString::new("").unwrap();
        assert!(rkr_read_chemfiles_range(
            path.as_ptr(),
            0,
            1,
            4,
            topology.as_ptr(),
            0,
            &mut n_frames
        )
        .is_null());
        let mut left = 0_u32;
        let mut right = 0_u32;
        let mut has_order = 0_u8;
        let mut order = 0_i32;
        assert_eq!(
            rkr_frame_bond_at(
                plain,
                0,
                &mut left,
                ptr::null_mut(),
                &mut has_order,
                &mut order
            ),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_bond_at(plain, 0, &mut left, &mut right, ptr::null_mut(), &mut order),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_bond_at(
                plain,
                0,
                &mut left,
                &mut right,
                &mut has_order,
                ptr::null_mut()
            ),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let force = [0.1_f64, 0.0, 0.0];
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
            rkr_frame_add_atom(builder, ptr::null(), 0.0, 0.0, 0.0, false, 1, 1.0),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_last_force(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_last_velocity(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_last_displacement(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_last_spread(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_add_atom(builder, copper.as_ptr(), 0.0, 0.0, 0.0, false, 1, 63.5),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_last_force(builder, force.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_forces_from_flat(builder, ptr::null(), 3),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_positions_from_flat(builder, ptr::null(), 3),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_metadata_json(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_units(builder, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut tensor = ptr::null_mut();
        assert_eq!(
            rkr_frame_builder_forces_dlpack(builder, &mut tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        rkr_dlpack_delete(tensor);
        assert_eq!(
            rkr_frame_builder_displacements_dlpack(builder, &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_spreads_dlpack(builder, &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_atom_energies_dlpack(builder, &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        let lanes = RKRDlpackExportOptions {
            dtype: RKRDLDataType {
                code: rkr_dl_type_code::RKR_DL_FLOAT,
                bits: 64,
                lanes: 2,
            },
            device: RKRDLDevice {
                device_type: rkr_dl_device_type::RKR_DL_CPU,
                device_id: 0,
            },
        };
        assert_eq!(
            rkr_frame_builder_positions_dlpack_ex(builder, &lanes, &mut tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        let cuda = RKRDlpackExportOptions {
            dtype: RKRDLDataType {
                code: rkr_dl_type_code::RKR_DL_FLOAT,
                bits: 64,
                lanes: 1,
            },
            device: RKRDLDevice {
                device_type: rkr_dl_device_type::RKR_DL_CUDA,
                device_id: 0,
            },
        };
        assert_eq!(
            rkr_frame_builder_positions_dlpack_ex(builder, &cuda, &mut tensor),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert!(rkr_frame_builder_velocities_data(builder).is_null());
        assert!(!rkr_frame_builder_positions_data(builder).is_null());
        let mut has = false;
        assert_eq!(
            rkr_frame_builder_get_atom_position(builder, 0, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_velocity(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_force(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_mass(builder, 0, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_positions_dlpack(builder, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_velocities_dlpack(builder, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_forces_dlpack(builder, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_xyz_view(plain, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_atom_ids_view(plain, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_positions_dlpack(plain, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_velocities_dlpack(plain, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut sel = ptr::null_mut();
        assert_eq!(
            rkr_frame_select(plain, ptr::null(), &mut sel),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let header = rkr_frame_get_header_line_cpp(plain, true, 0);
        assert!(!header.is_null());
        rkr_free_string(header);
        let absent = rkr_frame_new(
            cell.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert!(absent.is_null());
        let rich = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert!(!rich.is_null());
        let velocity = [0.2_f64, 0.0, 0.0];
        let disp = [0.01_f64, 0.0, 0.0];
        assert_eq!(
            rkr_frame_add_atom_with_velocity(
                rich,
                copper.as_ptr(),
                0.4,
                0.0,
                0.0,
                false,
                3,
                63.5,
                velocity[0],
                velocity[1],
                velocity[2]
            ),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_last_force(rich, force.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_last_displacement(rich, disp.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_last_spread(rich, disp.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_last_energy(rich, 0.3),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let rich_frame = rkr_frame_builder_build(rich);
        assert!(!rich_frame.is_null());
        let mut wide = [0.0_f64; 6];
        let mut narrow = [0.0_f64; 1];
        assert_eq!(
            rkr_frame_copy_velocities(rich_frame, narrow.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_velocities(rich_frame, wide.as_mut_ptr(), 3),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!((wide[0] - 0.2).abs() < 1e-12);
        assert_eq!(
            rkr_frame_copy_forces(rich_frame, narrow.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_forces(rich_frame, wide.as_mut_ptr(), 3),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_copy_displacements(rich_frame, narrow.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_displacements(rich_frame, wide.as_mut_ptr(), 3),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_copy_spreads(rich_frame, narrow.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_spreads(rich_frame, wide.as_mut_ptr(), 3),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_copy_atom_energies(rich_frame, narrow.as_mut_ptr(), 0),
            RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(
            rkr_frame_copy_atom_energies(rich_frame, wide.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!((wide[0] - 0.3).abs() < 1e-12);
        let mut count = 0_usize;
        assert!(!rkr_frame_velocities_f64(rich_frame, &mut count).is_null());
        assert_eq!(count, 1);
        assert!(!rkr_frame_forces_f64(rich_frame, &mut count).is_null());
        assert!(!rkr_frame_displacements_f64(rich_frame, &mut count).is_null());
        assert!(!rkr_frame_spreads_f64(rich_frame, &mut count).is_null());
        assert!(rkr_frame_velocities_f64(rich_frame, ptr::null_mut()).is_null() == false);
        free_rkr_frame(rich_frame);
        free_rkr_frame_builder(builder);
        assert_eq!(
            rkr_read_all_frames(path.as_ptr(), ptr::null_mut()),
            ptr::null_mut()
        );
        assert_eq!(
            rkr_read_all_frames_n_threads(path.as_ptr(), ptr::null_mut(), 1),
            ptr::null_mut()
        );
    }
    unsafe { free_rkr_frame(plain) };
}

#[test]
fn second_null_and_absent_section_return_status() {
    let cell = [4.0_f64, 5.0, 6.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let symbol = CString::new("Cu").unwrap();
    let key = CString::new("note").unwrap();
    let value = CString::new("kept").unwrap();
    let selection = CString::new("name Cu").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("second-null.con");
    let name = CString::new(path.to_str().unwrap()).unwrap();
    let bad_utf = [0xff_u8, 0];
    unsafe {
        assert_eq!(
            map_dlpack_err(crate::error::ParseError::ValidationError(
                "device mismatch on export".into(),
            )),
            RKRStatus::RKR_STATUS_DEVICE_MISMATCH
        );
        assert_eq!(
            map_dlpack_err(crate::error::ParseError::ValidationError(
                "no device allocator in this build".into(),
            )),
            RKRStatus::RKR_STATUS_DEVICE_ALLOC_UNSUPPORTED
        );
        assert_eq!(
            map_dlpack_err(crate::error::ParseError::ValidationError(
                "allocator refused the request".into(),
            )),
            RKRStatus::RKR_STATUS_DEVICE_ALLOC_UNSUPPORTED
        );
        assert_eq!(
            map_dlpack_err(crate::error::ParseError::ValidationError(
                "shape is not rectangular".into(),
            )),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        assert_eq!(
            map_dlpack_err(crate::error::ParseError::IncompleteHeader),
            RKRStatus::RKR_STATUS_INTERNAL_ERROR
        );

        let positions = ndarray::ArcArray2::from_shape_vec((1, 3), vec![0.1, 0.2, 0.3]).unwrap();
        let masses = ndarray::ArcArray1::from_vec(vec![63.546_f64]);
        let cuda = RKRDlpackExportOptions {
            dtype: RKRDLDataType {
                code: rkr_dl_type_code::RKR_DL_FLOAT,
                bits: 64,
                lanes: 1,
            },
            device: RKRDLDevice {
                device_type: rkr_dl_device_type::RKR_DL_CUDA,
                device_id: 0,
            },
        };
        let mut tensor = ptr::null_mut();
        assert_eq!(
            export_owned_array2_dlpack_opts(&positions, &cuda, &mut tensor),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert!(tensor.is_null());
        assert_eq!(
            export_owned_array1_f64_dlpack_opts(&masses, &cuda, &mut tensor),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert!(tensor.is_null());
        let other = RKRDlpackExportOptions {
            dtype: cuda.dtype,
            device: RKRDLDevice {
                device_type: 9,
                device_id: 0,
            },
        };
        assert_eq!(
            resolve_dlpack_opts(&other).unwrap_err(),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );

        let builder = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert!(!builder.is_null());
        assert_eq!(rkr_frame_builder_atom_count(builder), 0);
        assert_eq!(
            rkr_frame_add_atom(builder, symbol.as_ptr(), 0.1, 0.2, 0.3, false, 7, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(rkr_frame_builder_forces_data(builder).is_null());
        assert!(rkr_frame_builder_displacements_data(builder).is_null());
        assert!(rkr_frame_builder_spreads_data(builder).is_null());
        assert!(rkr_frame_builder_atom_energies_data(builder).is_null());
        assert_eq!(
            rkr_frame_builder_forces_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_forces_dlpack_ex(builder, ptr::null(), &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_displacements_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_displacements_dlpack_ex(builder, ptr::null(), &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_spreads_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_spreads_dlpack_ex(builder, ptr::null(), &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_atom_energies_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_atom_energies_dlpack_ex(builder, ptr::null(), &mut tensor),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        assert_eq!(
            rkr_frame_builder_masses_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_atom_ids_dlpack_ex(builder, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let uint64 = RKRDlpackExportOptions {
            dtype: RKRDLDataType {
                code: rkr_dl_type_code::RKR_DL_UINT,
                bits: 64,
                lanes: 1,
            },
            device: RKRDLDevice {
                device_type: rkr_dl_device_type::RKR_DL_CPU,
                device_id: 0,
            },
        };
        assert_eq!(
            rkr_frame_builder_atom_ids_dlpack_ex(builder, &uint64, &mut tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(!tensor.is_null());
        rkr_dlpack_delete(tensor);
        let uint32 = RKRDlpackExportOptions {
            dtype: RKRDLDataType {
                code: rkr_dl_type_code::RKR_DL_UINT,
                bits: 32,
                lanes: 1,
            },
            device: uint64.device,
        };
        assert_eq!(
            rkr_frame_builder_atom_ids_dlpack_ex(builder, &uint32, &mut tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        rkr_dlpack_delete(tensor);
        tensor = ptr::null_mut();
        assert_eq!(
            rkr_frame_builder_positions_dlpack_ex(builder, &other, &mut tensor),
            RKRStatus::RKR_STATUS_FEATURE_DISABLED
        );
        assert!(tensor.is_null());
        let flat = [0.4_f64, 0.0, 0.0];
        assert_eq!(
            rkr_frame_builder_set_atom_velocity(builder, 0, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_atom_displacement(builder, 0, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_atom_spread(builder, 0, ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_displacements_from_flat(builder, ptr::null(), 3),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_spreads_from_flat(builder, ptr::null(), 3),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_atom_energies_from_flat(builder, ptr::null(), 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_atom_velocity(builder, 0, flat.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_clear_atom_velocity(builder, 0),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_clear_atom_displacement(builder, 0),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_clear_atom_spread(builder, 0),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_clear_atom_energy(builder, 0),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let mut xyz = [0.0_f64; 3];
        let mut has = false;
        let mut energy = 0.0_f64;
        assert_eq!(
            rkr_frame_builder_get_atom_velocity(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_velocity(builder, 0, xyz.as_mut_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_force(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_force(builder, 0, xyz.as_mut_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_displacement(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_displacement(builder, 0, xyz.as_mut_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_spread(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_spread(builder, 0, xyz.as_mut_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_energy(builder, 0, ptr::null_mut(), &mut has),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_get_atom_energy(builder, 0, &mut energy, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_scalar_metadata(builder, ptr::null(), 1.0),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_scalar_metadata(builder, key.as_ptr(), 1.5),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_frame_builder_set_string_metadata(builder, ptr::null(), value.as_ptr()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_string_metadata(builder, key.as_ptr(), ptr::null()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_builder_set_string_metadata(builder, key.as_ptr(), value.as_ptr()),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        let mut header = [0_u8; 16];
        assert_eq!(
            rkr_frame_get_header_line(frame, false, 4, header.as_mut_ptr().cast(), header.len()),
            RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
        );
        assert!(rkr_frame_get_header_line_cpp(frame, false, 4).is_null());
        assert_eq!(
            rkr_frame_copy_atom_ids(frame, ptr::null_mut(), 4),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_copy_masses(frame, ptr::null_mut(), 4),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_positions_dlpack_ex(frame, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_forces_dlpack_ex(frame, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_displacements_dlpack_ex(frame, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_spreads_dlpack_ex(frame, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_atom_energies_dlpack_ex(frame, ptr::null(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_xyz_view(frame, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_forces_view(frame, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_masses_view(frame, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_frame_select(frame, selection.as_ptr(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let junk = [0_u8; 8];
        assert_eq!(
            rkr_unpack_rcso_natoms(junk.as_ptr(), junk.len(), ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_unpack_rcso_positions(junk.as_ptr(), junk.len(), ptr::null_mut(), 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_unpack_rcso_forces(junk.as_ptr(), junk.len(), ptr::null_mut(), 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert!(rkr_read_chemfiles_first(bad_utf.as_ptr().cast()).is_null());
        assert!(rkr_read_chemfiles_nth(bad_utf.as_ptr().cast(), 0).is_null());
        assert!(rkr_read_chemfiles_nth(name.as_ptr(), 0).is_null());
        assert_eq!(rkr_chemfiles_nsteps(bad_utf.as_ptr().cast()), usize::MAX);
        assert!(rkr_read_chemfiles(name.as_ptr(), ptr::null_mut()).is_null());
        let mut nframes = 0_usize;
        assert!(
            rkr_read_chemfiles_range(name.as_ptr(), 0, 1, 2, ptr::null(), 0, ptr::null_mut())
                .is_null()
        );
        assert!(
            rkr_read_chemfiles_range(name.as_ptr(), 0, 1, 2, ptr::null(), 0, &mut nframes)
                .is_null()
        );
        assert!(rkr_read_chemfiles_memory(ptr::null(), name.as_ptr(), &mut nframes).is_null());
        assert!(rkr_read_chemfiles_memory(name.as_ptr(), ptr::null(), &mut nframes).is_null());
        let writer = create_writer_from_path_c(name.as_ptr());
        assert!(!writer.is_null());
        assert_eq!(
            rkr_writer_extend_binding(writer, ptr::null(), 2),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        assert_eq!(
            rkr_writer_extend_binding(writer, ptr::null(), 0),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let null_frame: *const RKRConFrame = ptr::null();
        assert_eq!(
            rkr_writer_extend(writer, &null_frame, 1),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let frame_ptr = frame as *mut RKRConFrame;
        assert_eq!(
            rkr_writer_extend_binding(writer, &frame_ptr, 1),
            RKRStatus::RKR_STATUS_IO_ERROR
        );
        let err = rkr_last_error_message();
        assert!(!err.is_null());
        let err_text = std::ffi::CStr::from_ptr(err).to_str().unwrap();
        assert!(err_text.contains("units.time"));
        let plain = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert_eq!(
            rkr_frame_add_atom(plain, symbol.as_ptr(), 0.4, 0.0, 0.0, false, 8, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let plain_frame = rkr_frame_builder_build(plain);
        assert!(!plain_frame.is_null());
        let plain_ptr = plain_frame as *mut RKRConFrame;
        assert_eq!(
            rkr_writer_extend_binding(writer, &plain_ptr, 1),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let empty = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        let empty_frame = rkr_frame_builder_build(empty);
        assert!(!empty_frame.is_null());
        let empty_ptr = empty_frame as *mut RKRConFrame;
        let empty_status = rkr_writer_extend_binding(writer, &empty_ptr, 1);
        assert_eq!(empty_status, RKRStatus::RKR_STATUS_SUCCESS);
        rkr_writer_flush(writer);
        free_rkr_writer(writer);
        let traj = rkr_trajectory_open(name.as_ptr(), 0);
        assert!(!traj.is_null());
        assert_eq!(
            rkr_trajectory_read_frame(traj, 0, ptr::null_mut()),
            RKRStatus::RKR_STATUS_NULL_POINTER
        );
        let mut loaded = ptr::null_mut();
        assert_eq!(
            rkr_trajectory_read_frame(traj, 0, &mut loaded),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(rkr_frame_atom_count(loaded), 1);
        free_rkr_frame(loaded);
        rkr_trajectory_free(traj);
        let mut slots = vec![plain_frame, empty_frame, ptr::null_mut()];
        slots.shrink_to_fit();
        assert_eq!(slots.len(), slots.capacity());
        let slot_ptr = slots.as_mut_ptr();
        let slot_len = slots.len();
        std::mem::forget(slots);
        free_rkr_frame_array(slot_ptr, slot_len);
        let again = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert_eq!(rkr_frame_builder_atom_count(again), 0);
        free_rkr_frame_builder(again);
        free_rkr_frame(frame);
    }
}

#[test]
fn dlpack_shape_float32_and_absent_force_block() {
    let cell = [4.0_f64, 5.0, 6.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let symbol = CString::new("Cu").unwrap();
    let data = CString::new("3\n\nCu 0 0 0\n").unwrap();
    unsafe {
        let builder = rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        assert_eq!(
            rkr_frame_add_atom(builder, symbol.as_ptr(), 0.1, 0.2, 0.3, false, 1, 63.546),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let frame = rkr_frame_builder_build(builder);
        assert!(!frame.is_null());
        let opts = RKRDlpackExportOptions {
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
        let mut tensor = ptr::null_mut();
        assert_eq!(
            rkr_frame_positions_dlpack_ex(frame, &opts, &mut tensor),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let managed = &mut *tensor;
        let shape = std::slice::from_raw_parts_mut(managed.dl_tensor.shape, 2);
        shape[0] = 9;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        shape[0] = 1;
        shape[1] = 4;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        shape[1] = 3;
        let saved_ndim = managed.dl_tensor.ndim;
        managed.dl_tensor.ndim = 3;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        managed.dl_tensor.ndim = 1;
        *managed.dl_tensor.shape = 1;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
        *managed.dl_tensor.shape = 1;
        shape[1] = 3;
        managed.dl_tensor.ndim = saved_ndim;
        let saved_bits = managed.dl_tensor.dtype.bits;
        managed.dl_tensor.dtype.bits = 16;
        assert_eq!(
            rkr_frame_positions_from_dlpack(frame, tensor),
            RKRStatus::RKR_STATUS_VALIDATION_ERROR
        );
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
        let rewritten = rkr_frame_binding_rewrite(frame);
        assert!(!rewritten.is_null());
        assert!(rkr_frame_binding_rewrite(ptr::null()).is_null());
        free_rkr_frame(rewritten);
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
        let mut need = 0_usize;
        assert_eq!(
            rkr_pack_rcso(frame, ptr::null_mut(), 0, &mut need),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        let mut buf = vec![0_u8; need];
        assert_eq!(
            rkr_pack_rcso(frame, buf.as_mut_ptr(), buf.len(), &mut need),
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert_eq!(
            rkr_unpack_rcso_forces(buf.as_ptr(), need, copied.as_mut_ptr(), 1),
            RKRStatus::RKR_STATUS_SECTION_ABSENT
        );
        let mut nframes = 0_usize;
        assert!(rkr_read_chemfiles_memory(data.as_ptr(), ptr::null(), &mut nframes).is_null());
        let format = CString::new("XYZ").unwrap();
        assert!(
            rkr_read_chemfiles_memory(data.as_ptr(), format.as_ptr(), ptr::null_mut()).is_null()
        );
        let missing = CString::new("missing-topology.xyz").unwrap();
        let empty_topology = CString::new("").unwrap();
        let topology = CString::new("topology.xyz").unwrap();
        assert!(rkr_read_chemfiles_range(
            missing.as_ptr(),
            0,
            1,
            2,
            empty_topology.as_ptr(),
            0,
            &mut nframes
        )
        .is_null());
        assert!(rkr_read_chemfiles_range(
            missing.as_ptr(),
            0,
            1,
            2,
            topology.as_ptr(),
            0,
            &mut nframes
        )
        .is_null());
        let empty = crate::chemfiles_selection::SelectionResult {
            selection: "name Cu".to_string(),
            context_size: 1,
            matches: Vec::new(),
        };
        let empty_ptr = Box::into_raw(Box::new(empty)) as *mut RKRSelectionResult;
        let mut written = 9_u64;
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
        let hit = crate::chemfiles_selection::SelectionResult {
            selection: "name Cu".to_string(),
            context_size: 1,
            matches: vec![crate::chemfiles_selection::SelectionMatch {
                size: 1,
                atoms: [0, 0, 0, 0],
            }],
        };
        let hit_ptr = Box::into_raw(Box::new(hit)) as *mut RKRSelectionResult;
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
        free_rkr_frame(frame);
    }
}

#[test]
fn empty_builder_float_storage_and_null_deleter() {
    let cell = [10.0_f64, 10.0, 10.0];
    let angles = [90.0_f64, 90.0, 90.0];
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
    assert!(unsafe { rkr_frame_builder_masses_data(builder) }.is_null());
    assert!(unsafe { rkr_frame_builder_atom_ids_data(builder) }.is_null());
    unsafe { free_rkr_frame_builder(builder) };

    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/tiny_cuh2.con");
    let name = CString::new(path.to_str().unwrap()).unwrap();
    let frame = unsafe { rkr_read_first_frame(name.as_ptr()) };
    assert!(!frame.is_null());
    let mut view = RKRArrayView {
        data: ptr::null(),
        n: 1,
        cols: 1,
        dtype_code: 1,
        dtype_bits: 1,
    };
    assert_eq!(
        unsafe { rkr_frame_energies_view(frame, &mut view) },
        RKRStatus::RKR_STATUS_SECTION_ABSENT
    );
    assert_eq!(view.n, 0);
    unsafe { free_rkr_frame(frame) };

    let dir = tempfile::tempdir().unwrap();
    let con = dir.path().join("f32.con");
    std::fs::write(
        &con,
        "\
user
{\"con_spec_version\":2,\"storage_dtypes\":{\"positions\":\"float32\",\"masses\":\"float32\"}}
10.0 10.0 10.0
90.0 90.0 90.0
0 0
1 0
1
1
63.546
Cu
Coordinates of Component 1
   0.250000   0.500000   0.750000 0  0
",
    )
    .unwrap();
    let con_name = CString::new(con.to_str().unwrap()).unwrap();
    let stored = unsafe { rkr_read_first_frame(con_name.as_ptr()) };
    assert!(!stored.is_null());
    let mut pos = [0.0_f64; 3];
    let mut mass = [0.0_f64; 1];
    assert_eq!(
        unsafe { rkr_frame_copy_positions(stored, pos.as_mut_ptr(), pos.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((pos[0] - 0.25).abs() < 1e-5);
    assert_eq!(
        unsafe { rkr_frame_copy_masses(stored, mass.as_mut_ptr(), mass.len()) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!((mass[0] - 63.546).abs() < 1e-3);
    unsafe { free_rkr_frame(stored) };

    let symbol = CString::new("Cu").unwrap();
    let owned = unsafe {
        rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert_eq!(
        unsafe { rkr_frame_add_atom(owned, symbol.as_ptr(), 0.0, 0.0, 0.0, false, 1, 63.546,) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    let mut tensor = ptr::null_mut();
    assert_eq!(
        unsafe { rkr_frame_builder_positions_dlpack(owned, &mut tensor) },
        RKRStatus::RKR_STATUS_SUCCESS
    );
    assert!(!tensor.is_null());
    let saved = unsafe { (*tensor).deleter };
    unsafe { (*tensor).deleter = None };
    unsafe { rkr_dlpack_delete(tensor) };
    unsafe { (*tensor).deleter = saved };
    unsafe { rkr_dlpack_delete(tensor) };
    unsafe { rkr_dlpack_delete(ptr::null_mut()) };
    unsafe { free_rkr_frame_builder(owned) };
}
