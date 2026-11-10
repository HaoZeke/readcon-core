! Copyright (c) readcon developers
program h2h_fortran
  use readcon
  use, intrinsic :: iso_fortran_env, only: real64, int64, int32
  implicit none
  character(len=16) :: op
  character(len=32) :: threads_s, index_s
  character(len=4096) :: path, out
  integer :: threads, index, nframes, natoms, i, j, unit
  integer(int64) :: c0, c1, rate, pos_xor, bits
  real(real64) :: elapsed
  type(frame_t), allocatable :: frames(:)
  type(frame_t) :: one
  type(iterator_t) :: it
  type(catom_t) :: atom
  real(real64), allocatable :: pos(:,:,:)
  real(real64), allocatable :: lengths(:,:), angles(:,:)
  integer(int32), allocatable :: types(:,:)
  real(real64) :: cell(3), angs(3), row(3, 1024)
  integer :: st, nat

  if (command_argument_count() /= 5) then
    error stop "usage: h2h_fortran op path out threads index"
  end if
  call get_command_argument(1, op)
  call get_command_argument(2, path)
  call get_command_argument(3, out)
  call get_command_argument(4, threads_s)
  call get_command_argument(5, index_s)
  read(threads_s, *) threads
  read(index_s, *) index

  if (trim(op) == "read") then
    call system_clock(c0, rate)
    frames = read_all_frames(trim(path), threads)
    nframes = size(frames)
    natoms = 0
    if (nframes > 0) natoms = frames(1)%natoms()
    allocate(pos(3, natoms, nframes), lengths(3, nframes), angles(3, nframes), types(natoms, nframes))
    do i = 1, nframes
      st = frames(i)%copy_positions(pos(:, :, i))
      if (st /= 0) error stop "copy_positions"
      call frames(i)%cell_lengths(cell)
      call frames(i)%cell_angles(angs)
      lengths(:, i) = cell
      angles(:, i) = angs
      do j = 1, natoms
        atom = frames(i)%atom(j)
        types(j, i) = int(atom%atomic_number, int32)
      end do
    end do
    call system_clock(c1)
    elapsed = real(c1 - c0, real64) / real(rate, real64)
    call emit_counts(elapsed, nframes, natoms)
    call write_frames(trim(out), nframes, natoms, pos, lengths, angles, types)
    do i = 1, nframes
      call frames(i)%free()
    end do
  else if (trim(op) == "random") then
    call system_clock(c0, rate)
    one = read_nth_frame(trim(path), index)
    natoms = one%natoms()
    nframes = 1
    allocate(pos(3, natoms, 1), lengths(3, 1), angles(3, 1), types(natoms, 1))
    st = one%copy_positions(pos(:, :, 1))
    if (st /= 0) error stop "copy_positions"
    call one%cell_lengths(cell)
    call one%cell_angles(angs)
    lengths(:, 1) = cell
    angles(:, 1) = angs
    do j = 1, natoms
      atom = one%atom(j)
      types(j, 1) = int(atom%atomic_number, int32)
    end do
    call system_clock(c1)
    elapsed = real(c1 - c0, real64) / real(rate, real64)
    call emit_counts(elapsed, nframes, natoms)
    call write_frames(trim(out), nframes, natoms, pos, lengths, angles, types)
    call one%free()
  else if (trim(op) == "stream") then
    call system_clock(c0, rate)
    it = open_iterator(trim(path))
    nframes = 0
    natoms = 0
    pos_xor = 0_int64
    do
      one = it%next()
      if (.not. one%valid()) exit
      nat = one%natoms()
      if (nframes == 0) natoms = nat
      st = one%copy_positions(row(:, 1:nat))
      if (st /= 0) error stop "copy_positions"
      do j = 1, nat
        bits = transfer(row(1, j), bits)
        pos_xor = ieor(pos_xor, bits)
        bits = transfer(row(2, j), bits)
        pos_xor = ieor(pos_xor, bits)
        bits = transfer(row(3, j), bits)
        pos_xor = ieor(pos_xor, bits)
      end do
      call one%free()
      nframes = nframes + 1
    end do
    call system_clock(c1)
    elapsed = real(c1 - c0, real64) / real(rate, real64)
    call it%free()
    call emit_counts(elapsed, nframes, natoms)
    print '(A,I0)', "pos_xor ", pos_xor
  else
    error stop "unknown op"
  end if

contains

  subroutine emit_counts(seconds, n_frames, n_atoms)
    real(real64), intent(in) :: seconds
    integer, intent(in) :: n_frames, n_atoms
    integer :: hwm
    print '(A,ES24.16E3)', "elapsed_s ", seconds
    print '(A,I0)', "n_frames ", n_frames
    print '(A,I0)', "n_atoms ", n_atoms
    hwm = vm_hwm()
    print '(A,I0)', "VmHWM_KiB ", hwm
  end subroutine

  integer function vm_hwm() result(kib)
    character(len=256) :: line
    integer :: unit, ios
    kib = 0
    open(newunit=unit, file="/proc/self/status", status="old", action="read")
    do
      read(unit, '(A)', iostat=ios) line
      if (ios /= 0) exit
      if (line(1:6) == "VmHWM:") then
        read(line(7:), *) kib
        exit
      end if
    end do
    close(unit)
  end function

  subroutine write_frames(filename, n_frames, n_atoms, pos, lengths, angles, types)
    character(len=*), intent(in) :: filename
    integer, intent(in) :: n_frames, n_atoms
    real(real64), intent(in) :: pos(3, n_atoms, n_frames)
    real(real64), intent(in) :: lengths(3, n_frames), angles(3, n_frames)
    integer(int32), intent(in) :: types(n_atoms, n_frames)
    integer :: unit, i, j
    integer(int64) :: counts(2)
    open(newunit=unit, file=filename, access="stream", form="unformatted", status="replace")
    write(unit) "H2H1"
    counts(1) = int(n_frames, int64)
    counts(2) = int(n_atoms, int64)
    write(unit) counts
    do i = 1, n_frames
      do j = 1, n_atoms
        write(unit) pos(1, j, i), pos(2, j, i), pos(3, j, i)
      end do
    end do
    do i = 1, n_frames
      write(unit) lengths(1, i), lengths(2, i), lengths(3, i)
    end do
    do i = 1, n_frames
      write(unit) angles(1, i), angles(2, i), angles(3, i)
    end do
    do i = 1, n_frames
      do j = 1, n_atoms
        write(unit) types(j, i)
      end do
    end do
    close(unit)
  end subroutine
end program
