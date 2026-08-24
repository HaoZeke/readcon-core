! Phase A corpus lock for the Fortran bindings.
! Valid fixtures match golden JSON (symbols, atom_ids, fixed, positions).
! Invalid fixtures fail to parse (read_first_frame returns an invalid handle).
module conformance_goldens
  use readcon
  use, intrinsic :: iso_fortran_env, only: real64, int64
  implicit none
  private
  public :: run_conformance_goldens

  integer, parameter :: max_cases = 32
  integer, parameter :: max_atoms = 64

  type :: case_t
    character(len=16) :: kind = ""
    character(len=128) :: id = ""
    character(len=256) :: path = ""
  end type

  type :: golden_t
    character(len=128) :: id = ""
    integer :: n_atoms = 0
    integer :: spec_version = 0
    character(len=8) :: symbols(max_atoms) = ""
    integer(int64) :: atom_ids(max_atoms) = 0_int64
    logical :: fixed(3, max_atoms) = .false.
    real(real64) :: positions(3, max_atoms) = 0.0_real64
  end type

contains

  integer function run_conformance_goldens(root) result(nfail)
    character(len=*), intent(in) :: root
    character(len=:), allocatable :: man_text, man_path
    type(case_t) :: cases(max_cases)
    integer :: n, i, nvalid, ninvalid
    logical :: ok

    nfail = 0
    nvalid = 0
    ninvalid = 0
    man_path = trim(root) // "/resources/conformance/manifest.toml"
    inquire(file=man_path, exist=ok)
    if (.not. ok) then
      print *, "missing ", man_path
      nfail = nfail + 1
      return
    end if
    man_text = slurp(man_path)
    n = parse_manifest(man_text, cases)
    if (n <= 0) then
      print *, "manifest lists no cases"
      nfail = nfail + 1
      return
    end if
    do i = 1, n
      if (trim(cases(i)%kind) == "valid") then
        nvalid = nvalid + 1
        nfail = nfail + check_valid(root, cases(i))
      else
        ninvalid = ninvalid + 1
        nfail = nfail + check_invalid(root, cases(i))
      end if
    end do
    if (nvalid == 0 .or. ninvalid == 0) then
      print *, "manifest missing valid or invalid cases"
      nfail = nfail + 1
    end if
    print *, "conformance goldens valid=", nvalid, " invalid=", ninvalid, &
         " nfail=", nfail
  end function

  integer function check_valid(root, c) result(nf)
    character(len=*), intent(in) :: root
    type(case_t), intent(in) :: c
    character(len=:), allocatable :: fixture, gpath, gtext
    type(golden_t) :: g
    type(frame_t) :: fr
    type(catom_t) :: at
    character(len=:), allocatable :: sym
    integer :: i
    logical :: ok

    nf = 0
    fixture = trim(root) // "/resources/conformance/" // trim(c%path)
    gpath = trim(root) // "/resources/conformance/golden/" // trim(c%id) // ".json"
    inquire(file=gpath, exist=ok)
    if (.not. ok) then
      print *, trim(c%id), ": missing golden JSON"
      nf = 1
      return
    end if
    gtext = slurp(gpath)
    if (.not. parse_golden(gtext, g)) then
      print *, trim(c%id), ": golden JSON parse failed"
      nf = 1
      return
    end if
    if (trim(g%id) /= trim(c%id)) then
      print *, trim(c%id), ": golden id mismatch"
      nf = nf + 1
    end if
    fr = read_first_frame(fixture)
    if (.not. fr%valid()) then
      print *, trim(c%id), ": valid fixture failed to parse"
      nf = nf + 1
      return
    end if
    if (int(fr%atom_count()) /= g%n_atoms) then
      print *, trim(c%id), ": n_atoms mismatch"
      nf = nf + 1
    end if
    if (int(fr%spec_version()) /= g%spec_version) then
      print *, trim(c%id), ": spec_version mismatch"
      nf = nf + 1
    end if
    do i = 1, g%n_atoms
      at = fr%atom(i)
      sym = z_to_symbol(int(at%atomic_number, int64))
      if (trim(sym) /= trim(g%symbols(i))) then
        print *, trim(c%id), ": symbol mismatch"
        nf = nf + 1
      end if
      if (at%atom_id /= g%atom_ids(i)) then
        print *, trim(c%id), ": atom_id mismatch"
        nf = nf + 1
      end if
      if (logical(at%fixed_x) .neqv. g%fixed(1, i) .or. &
          logical(at%fixed_y) .neqv. g%fixed(2, i) .or. &
          logical(at%fixed_z) .neqv. g%fixed(3, i)) then
        print *, trim(c%id), ": fixed mismatch"
        nf = nf + 1
      end if
      if (at%x /= g%positions(1, i) .or. at%y /= g%positions(2, i) .or. &
          at%z /= g%positions(3, i)) then
        print *, trim(c%id), ": position mismatch"
        nf = nf + 1
      end if
    end do
    call fr%free()
  end function

  integer function check_invalid(root, c) result(nf)
    character(len=*), intent(in) :: root
    type(case_t), intent(in) :: c
    character(len=:), allocatable :: fixture, gpath
    type(frame_t) :: fr
    logical :: ok

    nf = 0
    fixture = trim(root) // "/resources/conformance/" // trim(c%path)
    gpath = trim(root) // "/resources/conformance/golden/" // trim(c%id) // ".json"
    inquire(file=gpath, exist=ok)
    if (ok) then
      print *, trim(c%id), ": invalid case must not have a golden"
      nf = nf + 1
    end if
    fr = read_first_frame(fixture)
    if (fr%valid()) then
      print *, trim(c%id), ": invalid fixture parsed"
      nf = nf + 1
      call fr%free()
    end if
  end function

  function slurp(path) result(text)
    character(len=*), intent(in) :: path
    character(len=:), allocatable :: text
    integer :: u, n, ios
    inquire(file=path, size=n)
    if (n < 0) then
      text = ""
      return
    end if
    allocate(character(len=n) :: text)
    open(newunit=u, file=path, status="old", access="stream", &
         form="unformatted", action="read", iostat=ios)
    if (ios /= 0) then
      text = ""
      return
    end if
    read(u, iostat=ios) text
    close(u)
    if (ios /= 0) text = ""
  end function

  integer function parse_manifest(text, cases) result(n)
    character(len=*), intent(in) :: text
    type(case_t), intent(out) :: cases(:)
    integer :: pos, eol, eq
    character(len=:), allocatable :: line, key, val

    n = 0
    pos = 1
    do while (pos <= len(text))
      eol = index(text(pos:), new_line("a"))
      if (eol == 0) then
        line = text(pos:)
        pos = len(text) + 1
      else
        line = text(pos:pos + eol - 2)
        pos = pos + eol
      end if
      line = trim(adjustl(line))
      if (len_trim(line) == 0) cycle
      if (line(1:1) == "#") cycle
      if (line == "[[valid]]" .or. line == "[[invalid]]") then
        n = n + 1
        if (n > size(cases)) then
          n = size(cases)
          return
        end if
        cases(n)%kind = merge("valid  ", "invalid", line == "[[valid]]")
        cases(n)%id = ""
        cases(n)%path = ""
        cycle
      end if
      if (n == 0) cycle
      eq = index(line, "=")
      if (eq <= 0) cycle
      key = trim(adjustl(line(:eq - 1)))
      val = unquote(trim(adjustl(line(eq + 1:))))
      if (key == "id") then
        cases(n)%id = val
      else if (key == "path") then
        cases(n)%path = val
      end if
    end do
  end function

  logical function parse_golden(text, g) result(ok)
    character(len=*), intent(in) :: text
    type(golden_t), intent(out) :: g
    character(len=:), allocatable :: blob
    integer :: i, p

    ok = .false.
    g = golden_t()
    if (.not. json_string_field(text, "id", g%id)) return
    if (.not. json_int_field(text, "n_atoms", g%n_atoms)) return
    if (.not. json_int_field(text, "spec_version", g%spec_version)) return
    if (g%n_atoms < 1 .or. g%n_atoms > max_atoms) return

    if (.not. json_array_blob(text, "symbols", blob)) return
    p = after_open_bracket(blob)
    if (p <= 0) return
    do i = 1, g%n_atoms
      if (.not. next_json_string(blob, p, g%symbols(i))) return
    end do

    if (.not. json_array_blob(text, "atom_ids", blob)) return
    p = after_open_bracket(blob)
    if (p <= 0) return
    do i = 1, g%n_atoms
      if (.not. next_json_int64(blob, p, g%atom_ids(i))) return
    end do

    if (.not. json_array_blob(text, "fixed", blob)) return
    p = after_open_bracket(blob)
    if (p <= 0) return
    do i = 1, g%n_atoms
      if (.not. next_bool3(blob, p, g%fixed(:, i))) return
    end do

    if (.not. json_array_blob(text, "positions", blob)) return
    p = after_open_bracket(blob)
    if (p <= 0) return
    do i = 1, g%n_atoms
      if (.not. next_real3(blob, p, g%positions(:, i))) return
    end do
    ok = .true.
  end function

  function unquote(raw) result(s)
    character(len=*), intent(in) :: raw
    character(len=:), allocatable :: s
    integer :: n
    s = trim(adjustl(raw))
    n = len_trim(s)
    if (n >= 2 .and. s(1:1) == '"' .and. s(n:n) == '"') then
      s = s(2:n - 1)
    end if
  end function

  logical function json_string_field(text, key, out) result(ok)
    character(len=*), intent(in) :: text, key
    character(len=*), intent(out) :: out
    integer :: p
    ok = .false.
    out = ""
    p = key_colon(text, key)
    if (p <= 0) return
    ok = next_json_string(text, p, out)
  end function

  logical function json_int_field(text, key, out) result(ok)
    character(len=*), intent(in) :: text, key
    integer, intent(out) :: out
    integer :: p
    integer(int64) :: v
    ok = .false.
    out = 0
    p = key_colon(text, key)
    if (p <= 0) return
    if (.not. next_json_int64(text, p, v)) return
    out = int(v)
    ok = .true.
  end function

  integer function key_colon(text, key) result(p)
    character(len=*), intent(in) :: text, key
    character(len=:), allocatable :: pat
    integer :: i
    pat = '"' // trim(key) // '"'
    i = index(text, pat)
    if (i <= 0) then
      p = 0
      return
    end if
    p = i + len(pat)
    p = skip_ws(text, p)
    if (p > len(text) .or. text(p:p) /= ":") then
      p = 0
      return
    end if
    p = skip_ws(text, p + 1)
  end function

  logical function json_array_blob(text, key, blob) result(ok)
    character(len=*), intent(in) :: text, key
    character(len=:), allocatable, intent(out) :: blob
    integer :: p, depth, i, start
    ok = .false.
    blob = ""
    p = key_colon(text, key)
    if (p <= 0) return
    if (p > len(text) .or. text(p:p) /= "[") return
    depth = 0
    start = p
    do i = p, len(text)
      if (text(i:i) == "[") then
        depth = depth + 1
      else if (text(i:i) == "]") then
        depth = depth - 1
        if (depth == 0) then
          blob = text(start:i)
          ok = .true.
          return
        end if
      end if
    end do
  end function

  integer function after_open_bracket(text) result(p)
    character(len=*), intent(in) :: text
    p = skip_ws(text, 1)
    if (p > len(text) .or. text(p:p) /= "[") then
      p = 0
      return
    end if
    p = p + 1
  end function

  integer function skip_ws(text, p0) result(p)
    character(len=*), intent(in) :: text
    integer, intent(in) :: p0
    p = p0
    do while (p <= len(text))
      if (text(p:p) /= " " .and. text(p:p) /= achar(9) .and. &
          text(p:p) /= achar(10) .and. text(p:p) /= achar(13)) exit
      p = p + 1
    end do
  end function

  logical function next_json_string(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    character(len=*), intent(out) :: out
    integer :: q
    ok = .false.
    out = ""
    p = skip_ws(text, p)
    if (p <= len(text) .and. text(p:p) == ",") p = skip_ws(text, p + 1)
    if (p > len(text) .or. text(p:p) /= '"') return
    q = p + 1
    do while (q <= len(text) .and. text(q:q) /= '"')
      q = q + 1
    end do
    if (q > len(text)) return
    out = text(p + 1:q - 1)
    p = q + 1
    ok = .true.
  end function

  logical function next_json_int64(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    integer(int64), intent(out) :: out
    integer :: q, ios
    ok = .false.
    out = 0_int64
    p = skip_ws(text, p)
    if (p <= len(text) .and. (text(p:p) == "," .or. text(p:p) == "[")) then
      p = skip_ws(text, p + 1)
    end if
    q = p
    if (q <= len(text) .and. text(q:q) == "-") q = q + 1
    do while (q <= len(text) .and. text(q:q) >= "0" .and. text(q:q) <= "9")
      q = q + 1
    end do
    if (q == p) return
    read(text(p:q - 1), *, iostat=ios) out
    if (ios /= 0) return
    p = q
    ok = .true.
  end function

  logical function next_json_real(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    real(real64), intent(out) :: out
    integer :: q, ios
    character(len=1) :: ch
    ok = .false.
    out = 0.0_real64
    p = skip_ws(text, p)
    if (p <= len(text) .and. (text(p:p) == "," .or. text(p:p) == "[")) then
      p = skip_ws(text, p + 1)
    end if
    q = p
    if (q <= len(text) .and. (text(q:q) == "-" .or. text(q:q) == "+")) q = q + 1
    do while (q <= len(text))
      ch = text(q:q)
      if ((ch >= "0" .and. ch <= "9") .or. ch == "." .or. ch == "e" .or. &
          ch == "E" .or. ch == "+" .or. ch == "-") then
        q = q + 1
      else
        exit
      end if
    end do
    if (q == p) return
    read(text(p:q - 1), *, iostat=ios) out
    if (ios /= 0) return
    p = q
    ok = .true.
  end function

  logical function next_json_bool(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    logical, intent(out) :: out
    ok = .false.
    out = .false.
    p = skip_ws(text, p)
    if (p <= len(text) .and. (text(p:p) == "," .or. text(p:p) == "[")) then
      p = skip_ws(text, p + 1)
    end if
    if (p + 3 <= len(text) .and. text(p:p + 3) == "true") then
      out = .true.
      p = p + 4
      ok = .true.
    else if (p + 4 <= len(text) .and. text(p:p + 4) == "false") then
      out = .false.
      p = p + 5
      ok = .true.
    end if
  end function

  logical function next_bool3(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    logical, intent(out) :: out(3)
    integer :: j
    ok = .false.
    out = .false.
    p = skip_ws(text, p)
    if (p <= len(text) .and. text(p:p) == ",") p = skip_ws(text, p + 1)
    if (p > len(text) .or. text(p:p) /= "[") return
    p = p + 1
    do j = 1, 3
      if (.not. next_json_bool(text, p, out(j))) return
    end do
    p = skip_ws(text, p)
    if (p > len(text) .or. text(p:p) /= "]") return
    p = p + 1
    ok = .true.
  end function

  logical function next_real3(text, p, out) result(ok)
    character(len=*), intent(in) :: text
    integer, intent(inout) :: p
    real(real64), intent(out) :: out(3)
    integer :: j
    ok = .false.
    out = 0.0_real64
    p = skip_ws(text, p)
    if (p <= len(text) .and. text(p:p) == ",") p = skip_ws(text, p + 1)
    if (p > len(text) .or. text(p:p) /= "[") return
    p = p + 1
    do j = 1, 3
      if (.not. next_json_real(text, p, out(j))) return
    end do
    p = skip_ws(text, p)
    if (p > len(text) .or. text(p:p) /= "]") return
    p = p + 1
    ok = .true.
  end function

end module conformance_goldens
