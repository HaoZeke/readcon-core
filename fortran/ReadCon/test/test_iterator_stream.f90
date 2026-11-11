program test_iterator_stream
  use, intrinsic :: iso_c_binding, only: c_size_t, c_double, c_ptr, c_null_ptr, c_char, c_null_char, c_associated
  use, intrinsic :: iso_fortran_env, only: real64, int64
  use readcon
  implicit none
  call assert_missing_file()
  call assert_null_iterator_errors()
  call assert_empty_path()
  call assert_stream_matches_read_all()
  call assert_truncated_last_frame()
  call assert_public_readers()
  call assert_remaining_procedures()
  call assert_failure_returns()
  call assert_every_public_reader()
  call assert_missing_read_and_handle_array()
  call assert_blank_handles()
  call assert_source_conditions()
contains

  subroutine assert_missing_file()
    type(iterator_t) :: it
    integer :: status
    it = open_iterator("no-such-readcon-iterator.con")
    if (it%valid()) error stop "missing file must not open"
    status = it%forward()
    if (status /= rkr_status_null_pointer) error stop "missing iterator forward"
    call it%free()
  end subroutine

  subroutine assert_null_iterator_errors()
    type(iterator_t) :: it
    type(frame_t) :: frame
    integer :: status
    integer(c_size_t) :: skipped
    frame = it%next()
    if (frame%valid()) error stop "null iterator next"
    status = it%forward()
    if (status /= rkr_status_null_pointer) error stop "null iterator forward"
    skipped = it%skip(1)
    if (skipped /= 0_c_size_t) error stop "null iterator skip"
    frame = it%nth(0)
    if (frame%valid()) error stop "null iterator nth"
    call frame%free()
    if (frame%valid()) error stop "freed null frame stays invalid"
    call it%free()
  end subroutine

  subroutine assert_empty_path()
    type(iterator_t) :: it
    type(frame_t) :: frame
    integer :: status
    integer(c_size_t) :: skipped
    it = open_iterator("")
    if (it%valid()) error stop "empty path must not open"
    frame = it%next()
    if (frame%valid()) error stop "empty path next"
    call frame%free()
    if (frame%valid()) error stop "freed empty frame"
    status = it%forward()
    if (status /= rkr_status_null_pointer) error stop "empty path forward"
    skipped = it%skip(0)
    if (skipped /= 0_c_size_t) error stop "empty path skip zero"
    frame = it%nth(0)
    if (frame%valid()) error stop "empty path nth"
    call frame%free()
    call it%free()
    if (it%valid()) error stop "freed empty iterator"
  end subroutine

  subroutine assert_stream_matches_read_all()
    type(iterator_t) :: it
    type(frame_t), allocatable :: batch(:)
    type(frame_t) :: streamed, past
    type(catom_t) :: got_atom, expect_atom
    integer :: i, n, atoms, status
    real(real64), allocatable :: got(:,:), expect(:,:)
    real(real64) :: dx, cell(3), angles(3), batch_cell(3), batch_angles(3)
    character(len=*), parameter :: path = "iterator_stream_two.con"
    integer :: unit
    open(newunit=unit, file=path, status="replace")
    write(unit, "(A)") "user"
    write(unit, "(A)") '{"con_spec_version":2}'
    write(unit, "(A)") "12.0 12.0 12.0"
    write(unit, "(A)") "90.0 90.0 90.0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "1"
    write(unit, "(A)") "2"
    write(unit, "(A)") "63.546"
    write(unit, "(A)") "Cu"
    write(unit, "(A)") "Coordinates of Component 1"
    write(unit, "(A)") "1.0 0.0 0.0 0 0"
    write(unit, "(A)") "2.0 0.0 0.0 0 1"
    write(unit, "(A)") "user"
    write(unit, "(A)") '{"con_spec_version":2}'
    write(unit, "(A)") "14.0 14.0 14.0"
    write(unit, "(A)") "90.0 90.0 90.0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "1"
    write(unit, "(A)") "2"
    write(unit, "(A)") "63.546"
    write(unit, "(A)") "Cu"
    write(unit, "(A)") "Coordinates of Component 1"
    write(unit, "(A)") "3.0 0.0 0.0 0 0"
    write(unit, "(A)") "4.0 0.0 0.0 0 1"
    close(unit)
    batch = read_all_frames(path)
    if (size(batch) /= 2) error stop "read_all_frames count"
    it = open_iterator(path)
    if (.not. it%valid()) error stop "open stream"
    n = 0
    do
      streamed = it%next()
      if (.not. streamed%valid()) exit
      n = n + 1
      if (n > size(batch)) error stop "iterator extra frame"
      atoms = streamed%natoms()
      if (atoms /= batch(n)%natoms()) error stop "atom count"
      allocate(got(3, atoms), expect(3, atoms))
      status = streamed%copy_positions(got)
      if (status /= 0) error stop "stream positions"
      status = batch(n)%copy_positions(expect)
      if (status /= 0) error stop "batch positions"
      dx = 0.0_real64
      do i = 1, atoms
        dx = max(dx, abs(got(1, i) - expect(1, i)))
        dx = max(dx, abs(got(2, i) - expect(2, i)))
        dx = max(dx, abs(got(3, i) - expect(3, i)))
        got_atom = streamed%atom(i)
        expect_atom = batch(n)%atom(i)
        if (got_atom%atomic_number /= expect_atom%atomic_number) error stop "atomic number"
      end do
      if (dx /= 0.0_real64) error stop "max dx"
      call streamed%cell_lengths(cell)
      call streamed%cell_angles(angles)
      call batch(n)%cell_lengths(batch_cell)
      call batch(n)%cell_angles(batch_angles)
      if (any(cell /= batch_cell) .or. any(angles /= batch_angles)) error stop "cell"
      deallocate(got, expect)
      call streamed%free()
    end do
    if (n /= 2) error stop "iterator count"
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen"
    if (it%skip(-1) /= 0_c_size_t) error stop "negative skip"
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen skip zero"
    if (it%skip(0) /= 0_c_size_t) error stop "skip zero"
    streamed = it%next()
    if (.not. streamed%valid()) error stop "frame after skip zero"
    if (streamed%natoms() /= batch(1)%natoms()) error stop "skip zero atoms"
    call streamed%free()
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen skip one"
    if (it%skip(1) /= 1_c_size_t) error stop "skip one"
    streamed = it%next()
    if (.not. streamed%valid()) error stop "frame after skip"
    if (streamed%natoms() /= batch(2)%natoms()) error stop "skipped atom count"
    call streamed%free()
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen nth"
    streamed = it%nth(-1)
    if (.not. streamed%valid()) error stop "nth negative"
    if (streamed%natoms() /= batch(1)%natoms()) error stop "nth negative atoms"
    call streamed%free()
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen nth"
    streamed = it%nth(1)
    if (.not. streamed%valid()) error stop "nth frame"
    if (streamed%natoms() /= batch(2)%natoms()) error stop "nth atom count"
    call streamed%free()
    past = it%nth(4)
    if (past%valid()) error stop "nth past end"
    call it%free()
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "reopen eof"
    do
      streamed = it%next()
      if (.not. streamed%valid()) exit
      call streamed%free()
    end do
    past = it%next()
    if (past%valid()) error stop "eof next"
    status = it%forward()
    if (status /= rkr_status_index_out_of_bounds) error stop "eof forward"
    call it%free()
    do i = 1, size(batch)
      call batch(i)%free()
    end do
  end subroutine

  subroutine assert_truncated_last_frame()
    type(iterator_t) :: it
    type(frame_t) :: frame
    integer :: status, unit
    integer(c_size_t) :: skipped
    character(len=*), parameter :: path = "iterator_stream_cut.con"
    open(newunit=unit, file=path, status="replace")
    write(unit, "(A)") "user"
    write(unit, "(A)") '{"con_spec_version":2}'
    write(unit, "(A)") "10.0"
    close(unit)
    it = open_iterator(path)
    if (.not. it%valid()) error stop "truncated file must open"
    frame = it%next()
    if (frame%valid()) error stop "truncated frame must fail"
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "truncated reopen"
    status = it%forward()
    if (status /= rkr_status_io_error) error stop "truncated forward"
    call it%free()
    it = open_iterator(path)
    if (.not. it%valid()) error stop "truncated reopen skip"
    skipped = it%skip(1)
    ! con_frame_iterator_skip returns UINTPTR_MAX. This compiler's c_size_t is signed, so that bit pattern is -1.
    if (skipped /= -1_c_size_t) error stop "truncated skip"
    call it%free()
  end subroutine

  subroutine assert_public_readers()
    type(frame_t) :: frame, built, empty
    type(frame_t), allocatable :: batch(:)
    type(builder_t) :: builder
    type(writer_t) :: writer
    type(catom_t) :: atom
    integer :: status, unit, i, j, order, nframes
    logical :: has_order
    real(real64) :: cell(3), angles(3), pos(3, 2)
    real(c_double), pointer :: xyz(:,:)
    character(len=*), parameter :: path = "public_interface.con"
    character(len=*), parameter :: out = "public_interface_out.con"
    open(newunit=unit, file=path, status="replace")
    write(unit, "(A)") "user"
    write(unit, "(A)") '{"con_spec_version":2}'
    write(unit, "(A)") "8.0 9.0 10.0"
    write(unit, "(A)") "90.0 90.0 90.0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "1"
    write(unit, "(A)") "2"
    write(unit, "(A)") "63.546"
    write(unit, "(A)") "Cu"
    write(unit, "(A)") "Coordinates of Component 1"
    write(unit, "(A)") "1.0 0.0 0.0 0 0"
    write(unit, "(A)") "0.0 2.0 0.0 0 1"
    close(unit)
    if (len_trim(library_version()) == 0) error stop "library version"
    if (len_trim(status_message(rkr_status_success)) == 0) error stop "status text"
    if (symbol_to_z("Cu") /= 29_int64) error stop "copper number"
    if (len_trim(z_to_symbol(1_int64)) == 0) error stop "hydrogen symbol"
    nframes = count_frames(path)
    if (nframes /= 1) error stop "frame count"
    frame = read_first_frame(path)
    if (.not. frame%valid()) error stop "first frame"
    if (frame%atom_count() /= 2_c_size_t) error stop "atom count"
    call frame%cell_lengths(cell)
    if (abs(cell(1) - 8.0_real64) > 1.0e-8_real64) error stop "cell"
    call frame%cell_angles(angles)
    if (abs(angles(1) - 90.0_real64) > 1.0e-6_real64) error stop "angle"
    status = frame%copy_positions(pos)
    if (status /= 0) error stop "copy positions"
    atom = frame%atom(1)
    if (len_trim(frame%composition_formula()) == 0) error stop "formula"
    if (frame%total_mass() <= 0.0_real64) error stop "mass"
    if (frame%cell_volume() <= 0.0_real64) error stop "volume"
    if (len_trim(frame%metadata_json()) == 0) error stop "metadata"
    if (frame%bond_count() < 0) error stop "bonds"
    status = frame%bond_at(0, i, j, order, has_order)
    if (frame%spec_version() < 0) error stop "spec"
    status = frame%write_path(out)
    if (status /= 0) error stop "write path"
    call frame%free()
    batch = read_all_frames(path)
    if (size(batch) /= 1) error stop "all frames"
    call batch(1)%free()
    batch = read_all_frames(path, n_threads=1)
    if (size(batch) /= 1) error stop "threaded read"
    call batch(1)%free()
    frame = read_nth_frame(path, 0)
    if (.not. frame%valid()) error stop "nth frame"
    call frame%xyz_ptr(xyz, i)
    if (i /= 2) error stop "xyz pointer"
    call frame%free()
    cell = [4.0_real64, 4.0_real64, 4.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    builder = new_builder(cell, angles, header="user")
    if (.not. builder%valid()) error stop "builder"
    status = builder%add_atom("Cu", 0.0_real64, 0.0_real64, 0.0_real64, &
         0_int64, 63.546_real64, .false., .false., .false.)
    if (status /= 0) error stop "add atom"
    built = builder%build()
    if (.not. built%valid()) error stop "built frame"
    writer = open_writer("builder_out.con")
    if (.not. writer%valid()) error stop "writer"
    status = writer%extend_one(built)
    if (status /= 0) error stop "extend"
    call writer%free()
    call built%free()
    call builder%free()
    if (empty%valid()) error stop "default frame is empty"
    if (empty%atom_count() /= 0_c_size_t) error stop "empty atom count"
    call empty%cell_lengths(cell)
    call empty%free()
  end subroutine

  subroutine assert_remaining_procedures()
    type(frame_t) :: frame, built, chem
    type(frame_t), allocatable :: owned(:)
    type(builder_t) :: builder
    type(writer_t) :: writer
    type(c_ptr) :: tensor, block, raw
    integer :: status, i, j, order, nmatch, nwritten, spec
    integer(int64) :: indices(4)
    logical :: has_order
    real(real64) :: cell(3), angles(3), pos(3, 2), vel(3, 2), frc(3, 2), masses(2)
    real(c_double), pointer :: xyz(:,:)
    character(len=*), parameter :: path = "remaining.con"
    status = 0
    i = 0
    j = 0
    order = 0
    has_order = .false.
    if (con_spec_version() < 1) error stop "spec"
    if (len_trim(readcon_version) == 0) error stop "version"
    if (len_trim(last_error_message()) < 0) error stop "last error"
    if (has_chemfiles_support()) status = 0
    cell = [6.0_real64, 6.0_real64, 6.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    builder = new_builder(cell, angles, header="user")
    status = builder%add_atom_full("Cu", 0.0_real64, 0.0_real64, 0.0_real64, &
         1_int64, 63.546_real64, .false., .false., .false., &
         velocity=[0.1_real64, 0.0_real64, 0.0_real64], &
         force=[0.2_real64, 0.0_real64, 0.0_real64])
    if (status /= 0) error stop "full atom"
    status = builder%set_energy(1.25_real64)
    status = builder%set_metadata_json('{"con_spec_version":2}')
    status = builder%set_frame_index(3_int64)
    status = builder%copy_positions(pos)
    status = builder%copy_masses(masses)
    status = builder%positions_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    status = builder%velocities_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    status = builder%forces_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    status = builder%atom_energies_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    status = builder%masses_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    status = builder%atom_ids_dlpack(tensor)
    call builder%dlpack_delete(tensor)
    built = builder%build()
    if (.not. built%valid()) error stop "remaining build"
    if (.not. built%has_velocities()) error stop "velocities"
    if (.not. built%has_forces()) error stop "forces"
    status = built%copy_velocities(vel)
    status = built%copy_forces(frc)
    status = built%copy_masses(masses)
    call built%velocities_ptr(xyz, i)
    call built%forces_ptr(xyz, i)
    spec = int(built%index_energy())
    spec = int(built%fmax())
    spec = built%sections_mask()
    spec = built%index_natoms()
    if (len_trim(built%index_projection_json()) < 0) error stop "projection"
    if (len_trim(built%potential_type()) < 0) error stop "potential"
    spec = int(built%frame_index())
    spec = int(built%sim_time())
    spec = int(built%timestep())
    spec = int(built%neb_bead())
    spec = int(built%neb_band())
    if (built%atom_index_by_id(1_int64) < -1) error stop "id"
    status = built%select("name Cu", nmatch)
    status = built%select_primary("name Cu", indices, nwritten)
    raw = built%handle_ptr()
    raw = frame_c_handle(built)
    status = frame_metatensor_positions_block(built, block)
    call mts_block_free_rkr(block)
    status = frame_metatensor_velocities_block(built, block)
    call mts_block_free_rkr(block)
    status = frame_metatensor_forces_block(built, block)
    call mts_block_free_rkr(block)
    status = frame_metatensor_atom_energies_block(built, block)
    call mts_block_free_rkr(block)
    status = built%write_path(path)
    writer = open_writer_gzip("remaining.gz")
    if (writer%valid()) then
      status = writer%extend_one(built)
      status = writer%set_canonical(.true.)
      if (writer%is_canonical()) status = 0
      call writer%free()
    end if
    writer = open_writer_zstd("remaining.zst")
    if (writer%valid()) call writer%free()
    writer = open_writer_gzip_with_precision("remaining-p.gz", 8)
    if (writer%valid()) call writer%free()
    writer = open_writer_zstd_with_precision("remaining-p.zst", 8)
    if (writer%valid()) call writer%free()
    chem = read_chemfiles_first(path)
    call chem%free()
    chem = read_chemfiles_nth(path, 0)
    call chem%free()
    status = chemfiles_nsteps(path)
    owned = frames_from_handle_array(c_null_ptr, 0_c_size_t)
    if (size(owned) /= 0) error stop "empty handle array"
    call built%free()
    status = status + i + j + order
    has_order = has_order
  end subroutine

  subroutine assert_failure_returns()
    type(frame_t) :: blank, loaded
    type(builder_t) :: builder, bblank
    type(writer_t) :: wblank
    type(iterator_t) :: iblank
    real(real64) :: cell(3), angles(3), one(3, 1), none(3, 0), masses(0), energy
    integer :: status, nmatch, nwritten, ndim, bits, i, j, order
    integer(int64) :: s0, s1, z, indices(1)
    logical :: ok, has_order
    type(c_ptr) :: tensor, data
    character(len=:), allocatable :: text
    cell = [4.0_real64, 4.0_real64, 4.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    energy = blank%energy()
    text = blank%metadata_json()
    text = blank%composition_formula()
    text = blank%potential_type()
    text = blank%index_projection_json()
    status = int(blank%atom_count())
    status = blank%copy_positions(one)
    status = blank%copy_velocities(one)
    status = blank%copy_forces(one)
    status = blank%copy_masses(masses)
    status = blank%bond_count()
    status = blank%bond_at(0, i, j, order, has_order)
    status = blank%select("name Cu", nmatch)
    status = blank%select_primary("name Cu", indices, nwritten)
    status = blank%spec_version()
    status = int(blank%frame_index())
    if (blank%valid()) status = 1
    status = bblank%add_atom("Cu", 0.0_real64, 0.0_real64, 0.0_real64, &
         1_int64, 1.0_real64, .false., .false., .false.)
    status = bblank%set_energy(1.0_real64)
    status = bblank%copy_positions(one)
    if (wblank%valid()) status = 1
    if (iblank%valid()) status = 1
    status = iblank%forward()
    z = symbol_to_z("Cu")
    text = z_to_symbol(z)
    loaded = read_first_frame("no-such-readcon-frame.con")
    if (loaded%valid()) error stop "missing frame"
    status = count_frames("no-such-readcon-frame.con")
    call dlpack_inspect(c_null_ptr, ndim, s0, s1, bits, ok)
    data = dlpack_data_ptr(c_null_ptr)
    builder = new_builder(cell, angles)
    status = builder%add_atom("Cu", 0.1_real64, 0.0_real64, 0.0_real64, &
         1_int64, 63.5_real64, .false., .false., .false.)
    status = builder%add_atom("H", 0.2_real64, 0.0_real64, 0.0_real64, &
         2_int64, 1.0_real64, .false., .false., .false.)
    status = builder%positions_dlpack(tensor)
    call dlpack_inspect(tensor, ndim, s0, s1, bits, ok)
    data = dlpack_data_ptr(tensor)
    call builder%dlpack_delete(tensor)
    loaded = builder%build()
    status = loaded%copy_positions(none)
    energy = loaded%energy()
    status = loaded%bond_count()
    status = loaded%bond_at(0, i, j, order, has_order)
    call loaded%free()
    call builder%free()
    if (z < 0) status = status + ndim + bits
    status = status + int(energy)
    ok = ok .or. has_order
  end subroutine

  subroutine assert_every_public_reader()
    type(frame_t) :: built, nth, missing
    type(frame_t), allocatable :: frames(:)
    type(builder_t) :: builder
    type(writer_t) :: writer
    type(iterator_t) :: iter
    real(real64) :: cell(3), angles(3), pos(3, 2), one(3, 1)
    integer :: status, nmatch, i, j, order, nframes
    integer(c_size_t) :: skipped
    integer(int64) :: z
    logical :: has_order
    character(len=*), parameter :: path = "public-reader.con"
    cell = [4.0_real64, 5.0_real64, 6.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    builder = new_builder(cell, angles)
    status = builder%add_atom("Cu", 0.1_real64, 0.2_real64, 0.3_real64, &
         1_int64, 63.546_real64, .false., .false., .false.)
    if (status /= 0) error stop "public add Cu"
    status = builder%add_atom("H", 1.0_real64, 0.0_real64, 0.0_real64, &
         2_int64, 1.008_real64, .false., .false., .false.)
    if (status /= 0) error stop "public add H"
    status = builder%set_metadata_json('{"bonds":[[0,1],{"i":1,"j":0,"order":2}]}')
    if (status /= 0) error stop "public bonds"
    built = builder%build()
    if (.not. built%valid()) error stop "public build"
    if (built%bond_count() /= 2) error stop "bond count"
    status = built%bond_at(0, i, j, order, has_order)
    if (status /= 0 .or. has_order) error stop "bond 0"
    status = built%bond_at(1, i, j, order, has_order)
    if (status /= 0 .or. .not. has_order .or. order /= 2) error stop "bond 1"
    status = built%bond_at(4, i, j, order, has_order)
    if (status == 0) error stop "bond past the end"
    writer = open_writer(path)
    if (.not. writer%valid()) error stop "public writer"
    status = writer%extend_one(built)
    if (status /= 0) error stop "public extend"
    call writer%free()
    nframes = count_frames(path)
    if (nframes /= 1) error stop "public count"
    frames = read_all_frames(path)
    if (size(frames) /= 1) error stop "public read all"
    if (.not. frames(1)%valid()) error stop "public frame"
    frames = read_all_frames(path, n_threads=2)
    if (size(frames) /= 1) error stop "public threaded read"
    nth = read_nth_frame(path, 0)
    if (.not. nth%valid()) error stop "public nth"
    status = nth%copy_positions(pos)
    if (status /= 0) error stop "public positions"
    status = nth%copy_positions(one)
    if (status == 0) error stop "public short positions"
    missing = read_nth_frame("no-such-public.con", 0)
    if (missing%valid()) error stop "public missing nth"
    if (count_frames("no-such-public.con") /= -1) error stop "public missing count"
    iter = open_iterator(path)
    if (.not. iter%valid()) error stop "public iterator"
    skipped = iter%skip(0)
    if (skipped < 0_c_size_t) error stop "public skip"
    call iter%free()
    z = symbol_to_z("Cu")
    if (z /= 29_int64) error stop "public symbol"
    call nth%free()
    call built%free()
    call builder%free()
  end subroutine

  subroutine assert_missing_read_and_handle_array()
    interface
      function rkr_read_all_frames(fn, nout) bind(C, name="rkr_read_all_frames")
        import :: c_char, c_ptr, c_size_t
        character(kind=c_char), intent(in) :: fn(*)
        integer(c_size_t), intent(out) :: nout
        type(c_ptr) :: rkr_read_all_frames
      end function
    end interface
    type(builder_t) :: builder
    type(writer_t) :: writer
    type(frame_t) :: built
    type(frame_t), allocatable :: empty(:), owned(:)
    type(c_ptr) :: arr
    integer(c_size_t) :: nframes
    integer :: status, i, n, nwritten
    integer(int64) :: indices(4)
    real(real64) :: cell(3), angles(3)
    character(len=*), parameter :: path = "handle-array.con"
    character(kind=c_char), allocatable :: cpath(:)
    cell = [4.0_real64, 5.0_real64, 6.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    empty = read_all_frames("no-such-readcon-all.con")
    if (size(empty) /= 0) error stop "missing read_all_frames"
    builder = new_builder(cell, angles)
    status = builder%add_atom("Cu", 0.1_real64, 0.2_real64, 0.3_real64, &
         1_int64, 63.546_real64, .false., .false., .false.)
    if (status /= 0) error stop "handle array add"
    built = builder%build()
    if (.not. built%valid()) error stop "handle array build"
    status = built%select_primary("name Cu", indices, nwritten)
    if (status == 0) then
      if (nwritten < 1) error stop "select wrote no indices"
    else
      if (nwritten /= 0) error stop "failed select must write nothing"
    end if
    writer = open_writer(path)
    if (.not. writer%valid()) error stop "handle array writer"
    status = writer%extend_one(built)
    if (status /= 0) error stop "handle array extend"
    call writer%free()
    n = len_trim(path)
    allocate(cpath(n + 1))
    do i = 1, n
      cpath(i) = path(i:i)
    end do
    cpath(n + 1) = c_null_char
    arr = rkr_read_all_frames(cpath, nframes)
    if (.not. c_associated(arr) .or. nframes /= 1_c_size_t) error stop "c read"
    owned = frames_from_handle_array(arr, nframes)
    if (size(owned) /= 1) error stop "owned size"
    if (.not. owned(1)%valid()) error stop "owned frame"
    if (int(owned(1)%atom_count()) /= 1) error stop "owned atoms"
    call owned(1)%free()
    call built%free()
    call builder%free()
  end subroutine

  subroutine assert_blank_handles()
    type(frame_t) :: blank, nxt
    type(builder_t) :: bblank
    type(writer_t) :: wblank
    type(iterator_t) :: iblank
    type(c_ptr) :: tensor, block, raw
    real(real64) :: one(3, 1), masses(1), cell(3), angles(3)
    real(c_double), pointer :: xyz(:, :)
    integer :: status, nmatch, nwritten, i, j, order, n
    type(catom_t) :: atom
    integer(int64) :: indices(1)
    logical :: has_order
    character(len=:), allocatable :: text
    if (blank%valid()) error stop "blank frame is valid"
    if (int(blank%atom_count()) /= 0) error stop "blank atom count"
    text = blank%metadata_json()
    if (len(text) /= 0) error stop "blank metadata"
    text = blank%composition_formula()
    if (len(text) /= 0) error stop "blank formula"
    text = blank%potential_type()
    if (len(text) /= 0) error stop "blank potential"
    text = blank%index_projection_json()
    if (text /= "{}") error stop "blank projection"
    if (blank%energy() /= 0.0_real64) error stop "blank energy"
    if (blank%index_energy() /= 0.0_real64) error stop "blank index energy"
    if (blank%total_mass() /= 0.0_real64) error stop "blank mass"
    if (blank%cell_volume() /= 0.0_real64) error stop "blank volume"
    if (blank%fmax() /= 0.0_real64) error stop "blank fmax"
    if (blank%sim_time() /= 0.0_real64) error stop "blank time"
    if (blank%timestep() /= 0.0_real64) error stop "blank timestep"
    if (blank%sections_mask() /= 0) error stop "blank sections"
    if (blank%index_natoms() /= 0) error stop "blank index natoms"
    if (blank%frame_index() /= -1_int64) error stop "blank frame index"
    if (blank%neb_bead() /= -1_int64) error stop "blank bead"
    if (blank%neb_band() /= -1_int64) error stop "blank band"
    if (blank%spec_version() /= 0) error stop "blank spec"
    if (blank%bond_count() /= 0) error stop "blank bonds"
    if (blank%atom_index_by_id(1_int64) /= -1_int64) error stop "blank id"
    if (blank%has_velocities()) error stop "blank velocities"
    if (blank%has_forces()) error stop "blank forces"
    call blank%cell_lengths(cell)
    if (any(cell /= 0.0_real64)) error stop "blank cell"
    call blank%cell_angles(angles)
    if (any(angles /= 0.0_real64)) error stop "blank angles"
    status = blank%copy_positions(one)
    if (status /= rkr_status_null_pointer) error stop "blank positions"
    status = blank%copy_velocities(one)
    if (status /= rkr_status_null_pointer) error stop "blank copy vel"
    status = blank%copy_forces(one)
    if (status /= rkr_status_null_pointer) error stop "blank copy frc"
    status = blank%copy_masses(masses)
    if (status /= rkr_status_null_pointer) error stop "blank copy mass"
    status = blank%bond_at(0, i, j, order, has_order)
    if (status /= rkr_status_null_pointer) error stop "blank bond"
    status = blank%select("name Cu", nmatch)
    if (status /= rkr_status_null_pointer .or. nmatch /= 0) error stop "blank select"
    status = blank%select_primary("name Cu", indices, nwritten)
    if (status /= rkr_status_null_pointer .or. nwritten /= 0) error stop "blank primary"
    status = blank%write_path("blank-unassociated.con")
    if (status /= rkr_status_null_pointer) error stop "blank write"
    call blank%xyz_ptr(xyz, n)
    if (n /= 0) error stop "blank xyz ptr"
    call blank%velocities_ptr(xyz, n)
    if (n /= 0) error stop "blank vel ptr"
    call blank%forces_ptr(xyz, n)
    if (n /= 0) error stop "blank frc ptr"
    raw = blank%handle_ptr()
    if (c_associated(raw)) error stop "blank handle"
    status = frame_metatensor_positions_block(blank, block)
    if (status /= rkr_status_null_pointer) error stop "blank mts pos"
    call mts_block_free_rkr(block)
    status = frame_metatensor_velocities_block(blank, block)
    if (status /= rkr_status_null_pointer) error stop "blank mts vel"
    call mts_block_free_rkr(block)
    status = frame_metatensor_forces_block(blank, block)
    if (status /= rkr_status_null_pointer) error stop "blank mts frc"
    call mts_block_free_rkr(block)
    status = frame_metatensor_atom_energies_block(blank, block)
    if (status /= rkr_status_null_pointer) error stop "blank mts energy"
    call mts_block_free_rkr(block)
    status = bblank%add_atom("Cu", 0.0_real64, 0.0_real64, 0.0_real64, &
         1_int64, 63.5_real64, .false., .false., .false.)
    if (status /= rkr_status_null_pointer) error stop "blank add"
    status = bblank%set_energy(1.0_real64)
    if (status /= rkr_status_null_pointer) error stop "blank set energy"
    status = bblank%set_metadata_json("{}")
    if (status /= rkr_status_null_pointer) error stop "blank meta"
    status = bblank%copy_positions(one)
    if (status /= rkr_status_null_pointer) error stop "blank builder positions"
    status = bblank%copy_masses(masses)
    if (status /= rkr_status_null_pointer) error stop "blank builder masses"
    status = bblank%positions_dlpack(tensor)
    if (status /= rkr_status_null_pointer .or. c_associated(tensor)) error stop "blank dlpack"
    status = bblank%velocities_dlpack(tensor)
    if (status /= rkr_status_null_pointer) error stop "blank vel dlpack"
    status = bblank%forces_dlpack(tensor)
    if (status /= rkr_status_null_pointer) error stop "blank frc dlpack"
    status = bblank%atom_energies_dlpack(tensor)
    if (status /= rkr_status_null_pointer) error stop "blank energy dlpack"
    status = bblank%masses_dlpack(tensor)
    if (status /= rkr_status_null_pointer) error stop "blank mass dlpack"
    status = bblank%atom_ids_dlpack(tensor)
    if (status /= rkr_status_null_pointer) error stop "blank id dlpack"
    nxt = bblank%build()
    if (nxt%valid()) error stop "blank build"
    if (bblank%valid()) error stop "blank builder valid"
    if (wblank%valid()) error stop "blank writer valid"
    if (wblank%is_canonical()) error stop "blank canonical"
    status = wblank%extend_one(blank)
    if (status /= rkr_status_null_pointer) error stop "blank extend"
    status = wblank%set_canonical(.false.)
    if (status /= rkr_status_null_pointer) error stop "blank set canonical"
    if (iblank%valid()) error stop "blank iterator"
    nxt = iblank%next()
    if (nxt%valid()) error stop "blank next"
    status = iblank%forward()
    if (status /= rkr_status_null_pointer) error stop "blank forward"
    if (iblank%skip(1) /= 0_c_size_t) error stop "blank skip"
    nxt = iblank%nth(0)
    if (nxt%valid()) error stop "blank nth"
    if (blank%natoms() /= 0) error stop "blank natoms"
    atom = blank%atom(1)
    if (int(atom%atomic_number) /= 0) error stop "blank atom"
    status = bblank%set_frame_index(3_int64)
    if (status /= rkr_status_null_pointer) error stop "blank frame index set"
    call bblank%free()
    if (bblank%valid()) error stop "freed blank builder"
    call wblank%free()
    if (wblank%valid()) error stop "freed blank writer"
    if (count_frames("no-such-blank-count.con") /= -1) error stop "missing count"
    call iblank%free()
  end subroutine

  subroutine assert_source_conditions()
    interface
      function rkr_read_all_frames(fn, nout) bind(C, name="rkr_read_all_frames")
        import :: c_char, c_ptr, c_size_t
        character(kind=c_char), intent(in) :: fn(*)
        integer(c_size_t), intent(out) :: nout
        type(c_ptr) :: rkr_read_all_frames
      end function
    end interface
    type(frame_t) :: built, empty_frame, plain
    type(frame_t), allocatable :: owned(:)
    type(builder_t) :: live, empty_b, with_atom
    type(writer_t) :: writer
    type(catom_t) :: atom
    type(c_ptr) :: arr, tensor, block
    real(real64) :: cell(3), angles(3)
    real(real64) :: wide(3, 1), shortp(3, 0), masses(0), mass_wide(1)
    real(c_double), pointer :: xyz(:,:)
    integer :: status, n, ndim, bits, unit, i
    integer(int64) :: s0, s1, found
    integer(c_size_t) :: nframes
    logical :: ok
    character(len=:), allocatable :: text, pad, big
    character(kind=c_char), allocatable :: cpath(:)
    character(len=*), parameter :: empty_path = "source-empty.con"
    character(len=*), parameter :: zero_path = "source-zero.con"
    cell = [4.0_real64, 5.0_real64, 6.0_real64]
    angles = [90.0_real64, 90.0_real64, 90.0_real64]
    live = new_builder(cell, angles)
    if (.not. live%valid()) error stop "live builder"
    call live%free()
    if (live%valid()) error stop "freed live builder"
    empty_b = new_builder(cell, angles)
    status = empty_b%copy_positions(wide)
    if (status /= rkr_status_null_pointer) error stop "empty builder positions"
    with_atom = new_builder(cell, angles)
    status = with_atom%add_atom("Cu", 0.1_real64, 0.2_real64, 0.3_real64, &
         7_int64, 63.546_real64, .false., .false., .false.)
    if (status /= 0) error stop "source add"
    status = with_atom%copy_positions(shortp)
    if (status /= rkr_status_buffer_too_small) error stop "short builder positions"
    status = with_atom%copy_masses(masses)
    if (status /= rkr_status_buffer_too_small) error stop "short builder masses"
    status = with_atom%masses_dlpack(tensor)
    if (status /= 0 .or. .not. c_associated(tensor)) error stop "masses dlpack"
    call dlpack_inspect(tensor, ndim, s0, s1, bits, ok)
    if (ndim /= 1) error stop "masses tensor rank"
    call with_atom%dlpack_delete(tensor)
    status = with_atom%atom_energies_dlpack(tensor)
    if (c_associated(tensor)) then
      call dlpack_inspect(tensor, ndim, s0, s1, bits, ok)
      call with_atom%dlpack_delete(tensor)
    end if
    plain = with_atom%build()
    if (.not. plain%valid()) error stop "plain build"
    call plain%xyz_ptr(xyz, n)
    if (n /= 1) error stop "plain xyz count"
    call plain%velocities_ptr(xyz, n)
    if (n /= 0 .or. associated(xyz)) error stop "absent velocity pointer"
    call plain%forces_ptr(xyz, n)
    if (n /= 0 .or. associated(xyz)) error stop "absent force pointer"
    atom = plain%atom(0)
    if (int(atom%atomic_number) /= 0) error stop "atom zero"
    found = plain%atom_index_by_id(huge(0_int64))
    if (found /= -1_int64) error stop "missing atom id"
    status = plain%copy_velocities(shortp)
    if (status /= rkr_status_buffer_too_small) error stop "short velocities"
    status = plain%copy_velocities(wide)
    if (status == 0) error stop "absent velocities copied"
    status = plain%copy_forces(shortp)
    if (status /= rkr_status_buffer_too_small) error stop "short forces"
    status = plain%copy_forces(wide)
    if (status == 0) error stop "absent forces copied"
    status = plain%copy_masses(masses)
    if (status /= rkr_status_buffer_too_small) error stop "short frame masses"
    status = plain%copy_masses(mass_wide)
    if (status /= 0) error stop "frame masses"
    status = plain%write_path("no-such-readcon-dir/out.con")
    if (status == 0) error stop "missing directory write"
    writer = open_writer("source-canonical.con")
    if (.not. writer%valid()) error stop "canonical writer"
    status = writer%set_canonical(.false.)
    if (status /= 0) error stop "set canonical false"
    if (writer%is_canonical()) error stop "canonical stayed on"
    call writer%free()
    status = frame_metatensor_positions_block(plain, block)
    if (c_associated(block)) error stop "metatensor block without the feature"
    call mts_block_free_rkr(block)
    pad = repeat("x", 65536)
    big = '{"pad":"' // pad // '"}'
    empty_b = new_builder(cell, angles)
    status = empty_b%add_atom("Cu", 0.1_real64, 0.2_real64, 0.3_real64, &
         7_int64, 63.546_real64, .false., .false., .false.)
    if (status /= 0) error stop "long meta add"
    status = empty_b%set_metadata_json(big)
    if (status /= 0) error stop "long meta set"
    built = empty_b%build()
    if (.not. built%valid()) error stop "long meta build"
    text = built%metadata_json(stat=status)
    if (status /= rkr_status_buffer_too_small) error stop "long meta status"
    if (len(text) /= 0) error stop "long meta text"
    call built%free()
    call plain%free()
    open(newunit=unit, file=zero_path, status="replace")
    write(unit, "(A)") "user"
    write(unit, "(A)") '{"con_spec_version":2}'
    write(unit, "(A)") "4.0 5.0 6.0"
    write(unit, "(A)") "90.0 90.0 90.0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "0 0"
    write(unit, "(A)") "0"
    close(unit)
    empty_frame = read_first_frame(zero_path)
    if (.not. empty_frame%valid()) error stop "zero atom frame"
    call empty_frame%xyz_ptr(xyz, n)
    if (n /= 0) error stop "zero atom xyz"
    call empty_frame%free()
    open(newunit=unit, file=empty_path, status="replace")
    close(unit)
    n = len_trim(empty_path)
    allocate(cpath(n + 1))
    do i = 1, n
      cpath(i) = empty_path(i:i)
    end do
    cpath(n + 1) = c_null_char
    arr = rkr_read_all_frames(cpath, nframes)
    if (.not. c_associated(arr) .or. nframes /= 0_c_size_t) error stop "empty file handle"
    owned = frames_from_handle_array(arr, nframes)
    if (size(owned) /= 0) error stop "empty handle array size"
    if (count_frames("missing.con") /= -1) error stop "missing.con count"
  end subroutine
end program
