// Copyright (c) readcon-core developers
//! Extra calls into library lines the default suite leaves cold.
//! Behaviour of the library is unchanged.

use std::ffi::CString;
use std::io::Write;
use std::ptr;

use readcon_core::compression;
use readcon_core::ffi::*;
use readcon_core::iterators::read_positions_with_threads;
use readcon_core::iterators::{
    count_frames, measure_newline_scan, pack_position_slab, read_all_frames_phased,
    read_first_frame, read_nth_frame, read_nth_frame_from_text, touch_frame_checksum,
    touch_option_checksum,
};
#[cfg(feature = "parallel")]
use readcon_core::iterators::{
    parse_frames_at_offsets, parse_frames_parallel_with_threads, read_all_frames_indexed,
};
use readcon_core::parser::{self, validate_metadata_schema};
use readcon_core::types::{Bond, ConFrameBuilder, cell_vectors};
use serde_json::{Map, Value};

fn cstr(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn success(status: RKRStatus) {
    assert_eq!(status, RKRStatus::RKR_STATUS_SUCCESS);
}

fn builder() -> *mut RKRConFrameBuilder {
    let cell = [12.0_f64, 11.0, 10.0];
    let angles = [90.0_f64, 90.0, 90.0];
    let user = cstr("line coverage");
    let handle = unsafe {
        rkr_frame_new(
            cell.as_ptr(),
            angles.as_ptr(),
            user.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    };
    assert!(!handle.is_null());
    handle
}

#[test]
fn builder_mutation_round_trip_covers_the_c_setters() {
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
    let handle = builder();
    assert_eq!(
        unsafe { rkr_frame_builder_set_last_velocity(ptr::null_mut(), ptr::null()) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    let symbol = cstr("Cu");
    let velocity = [0.1_f64, 0.2, 0.3];
    let force = [1.0_f64, 0.0, -1.0];
    success(unsafe {
        rkr_frame_add_atom_full(
            handle,
            symbol.as_ptr(),
            1.0,
            2.0,
            3.0,
            true,
            false,
            true,
            7,
            63.5,
            velocity.as_ptr(),
            force.as_ptr(),
        )
    });
    let displacement = [0.01_f64, 0.02, 0.03];
    let spread = [0.04_f64, 0.05, 0.06];
    success(unsafe { rkr_frame_builder_set_last_displacement(handle, displacement.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_last_spread(handle, spread.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_last_energy(handle, -1.5) });
    success(unsafe { rkr_frame_builder_set_atom_position(handle, 0, 1.0, 2.0, 3.0) });
    success(unsafe { rkr_frame_builder_set_atom_velocity(handle, 0, velocity.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_atom_force(handle, 0, force.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_atom_displacement(handle, 0, displacement.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_atom_spread(handle, 0, spread.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_atom_energy(handle, 0, -2.0) });
    let flat = [1.0_f64, 2.0, 3.0];
    success(unsafe { rkr_frame_builder_set_positions_from_flat(handle, flat.as_ptr(), 3) });
    success(unsafe { rkr_frame_builder_set_forces_from_flat(handle, force.as_ptr(), 3) });
    success(unsafe {
        rkr_frame_builder_set_displacements_from_flat(handle, displacement.as_ptr(), 3)
    });
    success(unsafe { rkr_frame_builder_set_spreads_from_flat(handle, spread.as_ptr(), 3) });
    success(unsafe { rkr_frame_builder_set_atom_energies_from_flat(handle, &-2.0, 1) });
    let mut xyz = [0.0_f64; 3];
    let mut flag = false;
    success(unsafe { rkr_frame_builder_get_atom_position(handle, 0, xyz.as_mut_ptr()) });
    success(unsafe { rkr_frame_builder_get_atom_velocity(handle, 0, xyz.as_mut_ptr(), &mut flag) });
    assert!(flag);
    success(unsafe { rkr_frame_builder_get_atom_force(handle, 0, xyz.as_mut_ptr(), &mut flag) });
    success(unsafe {
        rkr_frame_builder_get_atom_displacement(handle, 0, xyz.as_mut_ptr(), &mut flag)
    });
    success(unsafe { rkr_frame_builder_get_atom_spread(handle, 0, xyz.as_mut_ptr(), &mut flag) });
    let mut energy = 0.0;
    success(unsafe { rkr_frame_builder_get_atom_energy(handle, 0, &mut energy, &mut flag) });
    assert!(flag);
    let mut mass = 0.0;
    success(unsafe { rkr_frame_builder_get_atom_mass(handle, 0, &mut mass) });
    assert_eq!(
        unsafe { rkr_frame_builder_get_atom_position(handle, 9, xyz.as_mut_ptr()) },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    success(unsafe { rkr_frame_builder_clear_atom_velocity(handle, 0) });
    success(unsafe { rkr_frame_builder_clear_atom_force(handle, 0) });
    success(unsafe { rkr_frame_builder_clear_atom_displacement(handle, 0) });
    success(unsafe { rkr_frame_builder_clear_atom_spread(handle, 0) });
    success(unsafe { rkr_frame_builder_clear_atom_energy(handle, 0) });
    let meta = cstr(r#"{"energy":1.25,"generator":"coverage","pbc":[true,true,true]}"#);
    success(unsafe { rkr_frame_builder_set_metadata_json(handle, meta.as_ptr()) });
    let units = cstr(r#"{"length":"angstrom","energy":"eV"}"#);
    success(unsafe { rkr_frame_builder_set_units(handle, units.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_con_spec_minor(handle, 2) });
    success(unsafe { rkr_frame_builder_set_energy(handle, 1.25) });
    success(unsafe { rkr_frame_builder_set_frame_index(handle, 3) });
    success(unsafe { rkr_frame_builder_set_time(handle, 0.5) });
    success(unsafe { rkr_frame_builder_set_timestep(handle, 0.001) });
    success(unsafe { rkr_frame_builder_set_neb_bead(handle, 1) });
    success(unsafe { rkr_frame_builder_set_neb_band(handle, 2) });
    let key = cstr("note");
    let value = cstr("kept");
    success(unsafe { rkr_frame_builder_set_string_metadata(handle, key.as_ptr(), value.as_ptr()) });
    success(unsafe { rkr_frame_builder_set_scalar_metadata(handle, key.as_ptr(), 4.0) });
    let mut tensor: *mut RKRDLManagedTensorVersioned = ptr::null_mut();
    success(unsafe { rkr_frame_builder_positions_dlpack(handle, &mut tensor) });
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_positions_dlpack_ex(handle, ptr::null(), &mut tensor) });
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_velocities_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_forces_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_displacements_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_spreads_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_atom_energies_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_builder_masses_dlpack(handle, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    assert!(!unsafe { rkr_frame_builder_positions_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_velocities_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_forces_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_displacements_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_spreads_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_atom_energies_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_masses_data(handle) }.is_null());
    assert!(!unsafe { rkr_frame_builder_atom_ids_data(handle) }.is_null());
    let clone = unsafe { rkr_frame_builder_clone(handle) };
    assert!(!clone.is_null());
    unsafe { free_rkr_frame_builder(clone) };
    let frame = unsafe { rkr_frame_builder_build(handle) };
    assert!(!frame.is_null());
    let mut buf = vec![0.0_f64; 12];
    success(unsafe { rkr_frame_copy_positions(frame, buf.as_mut_ptr(), buf.len()) });
    assert_eq!(&buf[..3], &[1.0, 2.0, 3.0]);
    success(unsafe { rkr_frame_copy_velocities(frame, buf.as_mut_ptr(), buf.len()) });
    success(unsafe { rkr_frame_copy_forces(frame, buf.as_mut_ptr(), buf.len()) });
    success(unsafe { rkr_frame_copy_displacements(frame, buf.as_mut_ptr(), buf.len()) });
    success(unsafe { rkr_frame_copy_spreads(frame, buf.as_mut_ptr(), buf.len()) });
    success(unsafe { rkr_frame_copy_atom_energies(frame, buf.as_mut_ptr(), buf.len()) });
    success(unsafe { rkr_frame_copy_masses(frame, buf.as_mut_ptr(), buf.len()) });
    let mut idbuf = vec![0_u64; 4];
    success(unsafe { rkr_frame_copy_atom_ids(frame, idbuf.as_mut_ptr(), idbuf.len()) });
    assert_eq!(idbuf[0], 7);
    success(unsafe { rkr_frame_positions_dlpack(frame, &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    success(unsafe { rkr_frame_positions_dlpack_ex(frame, ptr::null(), &mut tensor) });
    assert!(!tensor.is_null());
    unsafe { rkr_dlpack_delete(tensor) };
    let mut out_len = 0usize;
    success(unsafe { rkr_pack_rcso(frame, ptr::null_mut(), 0, &mut out_len) });
    let mut packed = vec![0_u8; out_len];
    success(unsafe { rkr_pack_rcso(frame, packed.as_mut_ptr(), packed.len(), &mut out_len) });
    assert_eq!(
        unsafe { rkr_pack_rcso(frame, packed.as_mut_ptr(), 1, &mut out_len) },
        RKRStatus::RKR_STATUS_BUFFER_TOO_SMALL
    );
    let mut natoms = 0u32;
    success(unsafe { rkr_unpack_rcso_natoms(packed.as_ptr(), packed.len(), &mut natoms) });
    assert_eq!(natoms, 1);
    let mut xyz_out = vec![0.0_f64; (natoms as usize) * 3];
    success(unsafe {
        rkr_unpack_rcso_positions(packed.as_ptr(), packed.len(), xyz_out.as_mut_ptr(), natoms)
    });
    assert_eq!(&xyz_out[..3], &[1.0, 2.0, 3.0]);
    success(unsafe {
        rkr_unpack_rcso_forces(packed.as_ptr(), packed.len(), xyz_out.as_mut_ptr(), natoms)
    });
    let path = cstr("resources/test/tiny_cuh2.con");
    let mut nframes = 0usize;
    assert!(unsafe { rkr_read_chemfiles_first(ptr::null()) }.is_null());
    assert!(
        unsafe { rkr_read_chemfiles_first(path.as_ptr()) }.is_null()
            || rkr_has_chemfiles_support() == 1
    );
    assert!(unsafe { rkr_read_chemfiles(ptr::null(), &mut nframes) }.is_null());
    assert!(unsafe { rkr_read_chemfiles_nth(ptr::null(), 0) }.is_null());
    assert_eq!(unsafe { rkr_chemfiles_nsteps(ptr::null()) }, usize::MAX);
    assert!(
        unsafe { rkr_read_chemfiles_range(ptr::null(), 0, 1, 1, ptr::null(), 0, &mut nframes) }
            .is_null()
    );
    let xyz = cstr("1\n\nCu 0 0 0\n");
    let fmt = cstr("XYZ");
    assert!(
        unsafe { rkr_read_chemfiles_memory(ptr::null(), fmt.as_ptr(), &mut nframes) }.is_null()
    );
    let mem =
        unsafe { rkr_read_chemfiles_memory(xyz.as_ptr(), fmt.as_ptr(), &mut nframes) };
    let mut selected: *mut RKRSelectionResult = ptr::null_mut();
    let query = cstr("name Cu");
    #[cfg(not(feature = "chemfiles"))]
    {
        assert!(mem.is_null());
        assert_eq!(rkr_has_chemfiles_support(), 0);
        assert_eq!(
            unsafe { rkr_frame_select(frame, query.as_ptr(), &mut selected) },
            RKRStatus::RKR_STATUS_SELECTION_ERROR
        );
        assert!(selected.is_null());
    }
    #[cfg(feature = "chemfiles")]
    {
        assert!(!mem.is_null());
        assert!(nframes >= 1);
        unsafe { free_rkr_frame_array(mem, nframes) };
        assert_eq!(rkr_has_chemfiles_support(), 1);
        assert_eq!(
            unsafe { rkr_frame_select(frame, query.as_ptr(), &mut selected) },
            RKRStatus::RKR_STATUS_SUCCESS
        );
        assert!(!selected.is_null());
        unsafe { rkr_selection_result_free(selected) };
    }
    unsafe { rkr_selection_result_free(ptr::null_mut()) };
    unsafe { free_rkr_frame(frame) };
    assert!(unsafe { rkr_frame_builder_build(ptr::null_mut()) }.is_null());
}

#[test]
fn trajectory_and_iterators_cover_index_errors() {
    assert!(unsafe { rkr_trajectory_open(ptr::null(), 0) }.is_null());
    let path = cstr("resources/test/tiny_multi_cuh2.con");
    let traj = unsafe { rkr_trajectory_open(path.as_ptr(), 1) };
    assert!(!traj.is_null());
    let len = unsafe { rkr_trajectory_len(traj) };
    assert!(len >= 1);
    let mut frame: *mut RKRConFrame = ptr::null_mut();
    success(unsafe { rkr_trajectory_read_frame(traj, 0, &mut frame) });
    assert!(!frame.is_null());
    let mut header_buf = [0_u8; 64];
    success(unsafe {
        rkr_frame_get_header_line(
            frame,
            true,
            0,
            header_buf.as_mut_ptr().cast(),
            header_buf.len(),
        )
    });
    assert!(header_buf.iter().any(|byte| *byte != 0));
    let header = unsafe { rkr_frame_get_header_line_cpp(frame, true, 0) };
    assert!(!header.is_null());
    unsafe { rkr_free_string(header) };
    unsafe { free_rkr_frame(frame) };
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(traj, len + 5, &mut frame) },
        RKRStatus::RKR_STATUS_INDEX_OUT_OF_BOUNDS
    );
    assert_eq!(
        unsafe { rkr_trajectory_read_frame(ptr::null(), 0, &mut frame) },
        RKRStatus::RKR_STATUS_NULL_POINTER
    );
    unsafe { rkr_trajectory_free(traj) };
    let missing = cstr("resources/test/does-not-exist.con");
    assert!(unsafe { rkr_read_first_frame(missing.as_ptr()) }.is_null());
    let mut n = 0usize;
    let frames = unsafe { rkr_read_all_frames_n_threads(path.as_ptr(), &mut n, 2) };
    assert!(!frames.is_null());
    assert!(n >= 2);
    unsafe { free_rkr_frame_array(frames, n) };
    let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
    let owned = cstr(&text);
    let iter = unsafe { read_con_string_iterator(owned.as_ptr()) };
    assert!(!iter.is_null());
    assert_eq!(unsafe { con_frame_iterator_skip(iter, 0) }, 0);
    let got = unsafe { con_frame_iterator_nth(iter, 0) };
    assert!(!got.is_null());
    unsafe { free_rkr_frame(got) };
    unsafe { free_con_frame_iterator(iter) };
    assert!(unsafe { read_con_file_iterator(ptr::null()) }.is_null());
}

#[test]
fn metadata_schema_rejects_each_bad_shape() {
    fn obj(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }
    let bad = [
        obj(&[("validate", Value::from("yes"))]),
        obj(&[("sections", Value::from(1))]),
        obj(&[("sections", Value::Array(vec![Value::from(1)]))]),
        obj(&[("validate", Value::from(true))]),
        obj(&[("con_spec_minor", Value::from(-1))]),
        obj(&[("energy", Value::from("nope"))]),
        obj(&[("frame_index", Value::from(1.5))]),
        obj(&[("generator", Value::from(1))]),
        obj(&[("units", Value::Array(vec![]))]),
        obj(&[(
            "pbc",
            Value::Array(vec![Value::from(true), Value::from(false)]),
        )]),
        obj(&[("bonds", Value::from("no"))]),
        obj(&[(
            "bonds",
            Value::Array(vec![Value::Array(vec![Value::from(1)])]),
        )]),
        obj(&[("bonds", Value::Array(vec![Value::from(1)]))]),
        obj(&[("converged", Value::from("yes"))]),
    ];
    for value in &bad {
        assert!(validate_metadata_schema(value).is_err(), "{value:?}");
    }
    let mut ok_map = Map::new();
    ok_map.insert("energy".into(), Value::from(1.0));
    ok_map.insert("sections".into(), Value::Array(vec![]));
    assert!(validate_metadata_schema(&ok_map).is_ok());
    let mut bonded = ok_map.clone();
    bonded.insert(
        "bonds".into(),
        Value::Array(vec![Value::Array(vec![Value::from(0), Value::from(1)])]),
    );
    assert!(validate_metadata_schema(&bonded).is_ok());
    assert!(parser::parse_line_of_n_f64("1.0 2.0 not", 3).is_err());
    assert_eq!(
        parser::parse_line_of_n_f64("1.0 2.0 3.0", 3).unwrap(),
        vec![1.0, 2.0, 3.0]
    );
}

#[test]
fn built_frame_exposes_every_section_view() {
    let mut built = ConFrameBuilder::new([8.0, 9.0, 10.0], [90.0, 90.0, 90.0]);
    built
        .add_atom("H", 0.0, 0.0, 0.0, [false, false, false], 1, 1.0)
        .with_velocity([0.1, 0.0, 0.0])
        .with_force([0.0, 0.2, 0.0])
        .with_displacement([0.0, 0.0, 0.3])
        .with_spread([0.4, 0.0, 0.0])
        .with_energy(-0.5)
        .add_bond(Bond::new(0, 0));
    assert_eq!(built.positions().len(), 3);
    built.positions_mut()[0] = 0.2;
    assert!(built.velocities().len() >= 3);
    assert!(built.forces().len() >= 3);
    assert_eq!(built.atom_energies().len(), 1);
    assert_eq!(built.masses().len(), 1);
    assert_eq!(built.positions_view().nrows(), 1);
    assert_eq!(built.positions_2d_ref().nrows(), 1);
    assert_eq!(built.velocities_2d_ref().nrows(), 1);
    assert_eq!(built.forces_2d_ref().nrows(), 1);
    assert!(built.positions_dlpack().is_ok());
    assert!(built.velocities_dlpack().unwrap().is_some());
    assert!(built.forces_dlpack().unwrap().is_some());
    assert!(built.atom_energies_dlpack().unwrap().is_some());
    assert!(built.masses_dlpack().is_ok());
    built.clear_forces_section();
    built.set_forces_from_flat(&[0.1, 0.2, 0.3]).unwrap();
    let _frame = built.build().unwrap();
    let path = std::path::Path::new("resources/test/tiny_cuh2.con");
    let block = read_positions_with_threads(path, Some(2)).unwrap();
    assert!(block.n_frames >= 1);
    assert!(block.values.len() >= 3);
    let empty = tempfile::NamedTempFile::new().unwrap();
    empty.as_file().write_all(b"   \n").unwrap();
    let empty_block = read_positions_with_threads(empty.path(), Some(1)).unwrap();
    assert_eq!(empty_block.n_frames, 0);
    assert!(empty_block.values.is_empty());
}

#[test]
fn parallel_advise_covers_the_short_map_branches() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mapped.con");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    file.set_len(1).unwrap();
    let map = unsafe { memmap2::Mmap::map(&file).unwrap() };
    compression::advise_dontneed_parallel(&map, 3);
    assert_eq!(map.len(), 1);
    assert_eq!(map[0], 0);
    drop(map);
    file.set_len(64 * 1024).unwrap();
    let map = unsafe { memmap2::Mmap::map(&file).unwrap() };
    compression::advise_dontneed_parallel(&map, 0);
    assert_eq!(map.len(), 64 * 1024);
    compression::advise_dontneed_parallel(&map, 1);
    assert_eq!(map[0], 0);
    compression::advise_dontneed_parallel(&map, 4);
    assert_eq!(map[map.len() - 1], 0);
    let text = std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap();
    let owned =
        compression::read_file_text(std::path::Path::new("resources/test/tiny_cuh2.con")).unwrap();
    assert_eq!(owned, text);
}

#[test]
fn parsed_frames_cover_the_public_readers() {
    let path = std::path::Path::new("resources/test/tiny_multi_cuh2.con");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(count_frames(path).unwrap() >= 2);
    let first = read_first_frame(path).unwrap();
    let second = read_nth_frame(path, 1).unwrap();
    assert_eq!(second.atom_data.len(), first.atom_data.len());
    assert!(read_nth_frame_from_text(&text, 3).is_err());
    #[cfg(feature = "parallel")]
    {
        let indexed = read_all_frames_indexed(path, Some(2)).unwrap();
        assert!(indexed.len() >= 2);
    }
    let phased = read_all_frames_phased(path, Some(2)).unwrap();
    assert!(phased.frames.len() >= 2);
    #[cfg(feature = "parallel")]
    {
        let parallel = parse_frames_parallel_with_threads(&text, Some(2));
        assert!(parallel.iter().all(|frame| frame.is_ok()));
        assert!(!parallel.is_empty());
        let bounds = readcon_core::iterators::frame_start_offsets(&text);
        let at_bounds = parse_frames_at_offsets(&text, &bounds, Some(1));
        assert_eq!(at_bounds.len(), bounds.len());
        assert!(at_bounds.iter().all(|frame| frame.is_ok()));
        assert!(parse_frames_at_offsets(&text, &[], None).is_empty());
    }
    let slab = pack_position_slab(std::slice::from_ref(&first), 2).unwrap();
    assert!(touch_frame_checksum(std::slice::from_ref(&first), &slab, 2).is_some());
    assert_eq!(touch_frame_checksum(&[], &[], 1), Some(0));
    assert!(touch_option_checksum(&[Some(first.clone())], &slab, 2).is_some());
    assert_eq!(touch_option_checksum(&[None], &[], 1), None);
    assert_eq!(touch_option_checksum(&[], &[], 1), Some(0));
    let (chunked, chunked_s) = measure_newline_scan(&text, 2, true);
    let (plain, plain_s) = measure_newline_scan(&text, 1, false);
    assert_eq!(chunked, plain);
    assert!(chunked_s >= 0.0 && plain_s >= 0.0);
    let vel = read_first_frame(std::path::Path::new(
        "resources/test/tiny_cuh2_vel_forces.con",
    ))
    .unwrap();
    let force =
        read_first_frame(std::path::Path::new("resources/test/tiny_cuh2_forces.con")).unwrap();
    let spread =
        read_first_frame(std::path::Path::new("resources/test/tiny_cuh2_spreads.con")).unwrap();
    let disp = read_first_frame(std::path::Path::new(
        "resources/test/tiny_cuh2_displacements.con",
    ))
    .unwrap();
    let extra = read_first_frame(std::path::Path::new(
        "resources/test/tiny_cuh2_charges_spins_magmoms.con",
    ))
    .unwrap();
    assert!(vel.velocities_as_dlpack(dlpk_cpu()).unwrap().is_some());
    assert!(force.forces_as_dlpack(dlpk_cpu()).unwrap().is_some());
    assert!(spread.forces_as_dlpack(dlpk_cpu()).is_ok());
    assert!(disp.atom_energies_as_dlpack(dlpk_cpu()).is_ok());
    assert!(extra.atom_energies_as_dlpack(dlpk_cpu()).is_ok());
    let cell = cell_vectors([1.0, 1.0, 1.0], [90.0, 90.0, 90.0]).unwrap();
    assert!(cell.iter().flatten().all(|value| value.is_finite()));
    assert_eq!(Bond::new(1, 0).with_order(2).order, Some(2));
}

fn dlpk_cpu() -> dlpk::sys::DLDevice {
    dlpk::sys::DLDevice {
        device_type: dlpk::sys::DLDeviceType::kDLCPU,
        device_id: 0,
    }
}

#[test]
fn builder_and_frame_sections_cover_the_empty_arms() {
    let mut built = ConFrameBuilder::new([8.0, 9.0, 10.0], [90.0, 90.0, 90.0]);
    built.with_velocity([0.0, 0.0, 0.0]);
    built.with_force([0.0, 0.0, 0.0]);
    built.with_displacement([0.0, 0.0, 0.0]);
    built.with_spread([0.0, 0.0, 0.0]);
    built.with_energy(0.0);
    built.with_charge(0.1);
    built.with_spin(0.5);
    built.with_magmom([0.0, 0.0, 1.0]);
    built.add_atom("H", 0.0, 0.0, 0.0, [true, true, true], 1, 1.0);
    assert!(built.velocities().is_empty());
    assert_eq!(built.velocities_mut().len(), 3);
    assert!(built.forces().is_empty());
    assert_eq!(built.forces_mut().len(), 3);
    assert_eq!(built.atom_energies_mut().len(), 1);
    assert!(built.set_atom_velocity(4, [1.0, 0.0, 0.0]).is_err());
    assert!(built.set_atom_force(4, [1.0, 0.0, 0.0]).is_err());
    assert!(built.set_atom_spread(4, [1.0, 0.0, 0.0]).is_err());
    assert!(built.set_atom_energy(4, 1.0).is_err());
    assert!(built.set_atom_fixed(4, [true, false, false]).is_err());
    assert!(built.set_atom_mass(4, 2.0).is_err());
    built.set_atom_velocity(0, [0.1, 0.0, 0.0]).unwrap();
    built.set_atom_force(0, [0.0, 0.1, 0.0]).unwrap();
    built.set_atom_spread(0, [0.0, 0.0, 0.1]).unwrap();
    built.set_atom_energy(0, -1.0).unwrap();
    built.clear_atom_velocity(0).unwrap();
    built.clear_atom_force(0).unwrap();
    built.clear_atom_displacement(0).unwrap();
    built.clear_atom_energy(0).unwrap();
    assert!(built.clear_atom_velocity(4).is_err());
    built.clear_energies_section();
    built.clear_displacements_section();
    built.clear_spreads_section();
    assert!(built.set_forces_from_flat(&[1.0]).is_err());
    assert!(built.set_atom_energies_from_flat(&[1.0, 2.0]).is_err());
    assert_eq!(built.get_atom_force(0).unwrap(), Some([0.0, 0.0, 0.0]));
    assert!(built.get_atom_force(4).is_err());
    assert_eq!(built.get_atom_energy(0).unwrap(), None);
    assert!(built.get_atom_energy(4).is_err());
    assert!(built.get_atom_mass(4).is_err());
    assert_eq!(built.get_atom_velocity(0).unwrap(), Some([0.0, 0.0, 0.0]));
    assert!(built.clear_atom_force(9).is_err());
    assert!(built.clear_atom_displacement(9).is_err());
    assert!(built.clear_atom_energy(9).is_err());
    assert!(built.atom_energies().is_empty());
    assert_eq!(built.velocities_view().ncols(), 3);
    assert_eq!(built.forces_view().ncols(), 3);
    assert!(built.velocities_dlpack().is_ok());
    assert!(built.forces_dlpack().is_ok());
    assert!(built.atom_energies_dlpack().unwrap().is_none());
    assert!(built.masses_dlpack().is_ok());
    assert!(built.atom_ids_dlpack().is_ok());
    assert!(built.positions_dlpack().is_ok());
    built.set_bonds(&[]);
    built.add_bond(Bond::new(0, 0).with_order(1));
    let mut frame = built.build().unwrap();
    frame.header.metadata.insert(
        "lattice_vectors".into(),
        serde_json::json!([[1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
    );
    assert!(frame.header.lattice_vectors().is_none());
    frame.header.add_bond(Bond::new(0, 0));
    frame.header.set_bonds(&[]);
    assert!(frame.atom_data[0].is_fully_fixed());
    frame.atom_data[0].set_velocity(Some([1.0, 0.0, 0.0]));
    frame.atom_data[0].set_force(Some([0.0, 1.0, 0.0]));
    frame.atom_data[0].set_energy(Some(-1.0));
    frame.atom_data[0].set_charge(Some(0.1));
    frame.atom_data[0].set_spin(Some(0.5));
    frame.atom_data[0].set_magmom(Some([0.0, 0.0, 1.0]));
    frame.atom_data[0].set_displacement(Some([0.1, 0.0, 0.0]));
    frame.atom_data[0].set_spread(Some([0.2, 0.0, 0.0]));
    frame.sync_arrays_from_atom_data();
    let dtypes = readcon_core::storage_dtype::StorageDtypes::default();
    frame.project_storage_dtypes(&dtypes);
    frame.atom_data[0].set_velocity(None);
    frame.atom_data[0].set_force(None);
    frame.atom_data[0].set_energy(None);
    frame.atom_data[0].set_charge(None);
    frame.atom_data[0].set_spin(None);
    frame.atom_data[0].set_magmom(None);
    frame.atom_data[0].set_displacement(None);
    frame.atom_data[0].set_spread(None);
    frame.sync_arrays_from_atom_data();
    frame.atom_data.clear();
    frame.sync_arrays_from_atom_data();
    assert!(cell_vectors([1.0, 1.0, 1.0], [0.0, 0.0, 90.0]).is_err());
    let mut bonds = serde_json::Map::new();
    bonds.insert("bonds".into(), serde_json::json!("no"));
    assert!(
        readcon_core::types::parse_bonds_from_metadata(&bonds.into_iter().collect()).is_empty()
    );
    let mut ordered = serde_json::Map::new();
    ordered.insert(
        "bonds".into(),
        serde_json::json!([{"i": 0, "j": 1, "order": 2}, [1]]),
    );
    let parsed = readcon_core::types::parse_bonds_from_metadata(&ordered.into_iter().collect());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].order, Some(2));
}
