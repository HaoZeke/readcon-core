program test_iterator_stream
  use, intrinsic :: iso_c_binding, only: c_size_t
  use, intrinsic :: iso_fortran_env, only: real64
  use readcon
  implicit none
  call assert_missing_file()
  call assert_null_iterator_errors()
  call assert_empty_path()
  call assert_stream_matches_read_all()
  call assert_truncated_last_frame()
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
end program
