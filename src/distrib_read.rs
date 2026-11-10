//! Distributed read: each rank of a caller-supplied communicator parses
//! only its block of frames (see [`crate::distrib::block_range`]).
//!
//! Two modes:
//!
//! - [`ReadMode::Shared`]: every rank opens the same file (shared file
//!   system). When the frames share one byte length, every rank reads
//!   only its own span and rank 0 does not scan the rest of the file.
//!   Otherwise rank 0 loads or builds the checked `.rcidx` index, and
//!   every rank, including rank 0, then reads only its own span. No
//!   frame bytes move between ranks.
//! - [`ReadMode::Scatter`]: rank 0 reads (and decompresses) the file,
//!   scans offsets, and sends each rank its byte range with
//!   `MPI_Scatterv`. Use it for gzip/zstd input or when the file is not
//!   visible on every rank.
//!
//! [`ReadMode::Auto`] picks `Scatter` for compressed input and `Shared`
//! otherwise. Every failure is collective: all ranks return an error.
//!
//! Frame ownership depends only on `(n_frames, n_ranks, rank)`, so the
//! frames of ranks `0..P` concatenated in rank order equal the serial
//! read for every `P`.

use std::path::Path;

use ::mpi::datatype::Partition;
use ::mpi::traits::*;

use crate::compression::{self, Compression, FileContents};
use crate::distrib::{Comm, block_range};
use crate::frame_index::{self, FrameIndex};
use crate::iterators::parse_frames_at_offsets;
use crate::types::ConFrame;

type BoxErr = Box<dyn std::error::Error>;

/// How frame bytes reach each rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReadMode {
    Auto = 0,
    Shared = 1,
    Scatter = 2,
}

impl ReadMode {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Auto),
            1 => Some(Self::Shared),
            2 => Some(Self::Scatter),
            _ => None,
        }
    }
}

/// This rank's share of a distributed read.
#[derive(Debug)]
pub struct RankFrames {
    /// Global index of `frames[0]`.
    pub first: usize,
    /// Total frame count across all ranks.
    pub n_total: usize,
    pub frames: Vec<ConFrame>,
}

fn bytes_of(fc: &FileContents) -> &[u8] {
    match fc {
        FileContents::Owned(s) => s.as_bytes(),
        FileContents::Mapped(m) => m,
    }
}

/// True on every rank iff `ok` is true on every rank.
pub(crate) fn agree(comm: &Comm, ok: bool) -> bool {
    let mut all = 0i32;
    comm.raw().all_reduce_into(
        &i32::from(ok),
        &mut all,
        ::mpi::collective::SystemOperation::min(),
    );
    all == 1
}

fn encode_u64s(v: &[u64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn decode_u64s(b: &[u8]) -> Vec<u64> {
    b.chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

/// Broadcast `Ok(payload)` or an error message from rank 0.
fn bcast_result(comm: &Comm, r: Result<Vec<u8>, String>) -> Result<Vec<u8>, BoxErr> {
    let msg = match &r {
        Ok(p) => [&[1u8][..], p].concat(),
        Err(e) => [&[0u8][..], e.as_bytes()].concat(),
    };
    let payload = if comm.rank() == 0 { &msg[..] } else { &[][..] };
    let got = comm.bcast_bytes(0, payload);
    match got.split_first() {
        Some((1, rest)) => Ok(rest.to_vec()),
        Some((_, rest)) => Err(String::from_utf8_lossy(rest).into_owned().into()),
        None => Err("empty broadcast".into()),
    }
}

/// Parse `bytes` (frames starting at `local_offsets`, relative to the
/// slice) and agree collectively on success.
fn parse_local(
    comm: &Comm,
    bytes: &[u8],
    local_offsets: &[usize],
    threads: Option<usize>,
) -> Result<Vec<ConFrame>, BoxErr> {
    let parsed: Result<Vec<ConFrame>, BoxErr> = std::str::from_utf8(bytes)
        .map_err(BoxErr::from)
        .and_then(|text| {
            parse_frames_at_offsets(text, local_offsets, threads)
                .into_iter()
                .map(|r| r.map_err(BoxErr::from))
                .collect()
        });
    let ok = agree(comm, parsed.is_ok());
    match (parsed, ok) {
        (Ok(f), true) => Ok(f),
        (Err(e), _) => Err(format!("rank {}: {e}", comm.rank()).into()),
        (Ok(_), false) => Err("frame parse failed on another rank".into()),
    }
}

/// Read this rank's block of frames from `path`.
///
/// `threads` sets the rayon pool size within each rank (`None` uses the
/// global pool). In shared mode, equal-length frames skip the full-file
/// index walk. Other files still let rank 0 refresh `path.rcidx` when
/// it is missing or stale, and then every rank reads only its span.
pub fn read_block(
    comm: &Comm,
    path: &Path,
    mode: ReadMode,
    threads: Option<usize>,
) -> Result<RankFrames, BoxErr> {
    // Rank 0 decides the mode so every rank takes the same branch.
    let decided = if comm.rank() == 0 {
        let m = match mode {
            ReadMode::Auto => match sniff(path) {
                Ok(Compression::None) => Ok(ReadMode::Shared),
                Ok(_) => Ok(ReadMode::Scatter),
                Err(e) => Err(e.to_string()),
            },
            m => Ok(m),
        };
        m.map(|m| vec![m as u8])
    } else {
        Ok(Vec::new())
    };
    let mode = match bcast_result(comm, decided)?.first() {
        Some(1) => ReadMode::Shared,
        _ => ReadMode::Scatter,
    };
    match mode {
        ReadMode::Shared => read_shared(comm, path, threads),
        _ => read_scatter(comm, path, threads),
    }
}

fn sniff(path: &Path) -> std::io::Result<Compression> {
    use std::io::Read;
    let mut m = [0u8; 4];
    let n = std::fs::File::open(path)?.read(&mut m)?;
    Ok(compression::detect_compression(&m[..n]))
}

/// Rank 0: read the file and scan or load its index.
fn root_index(path: &Path, write_sidecar: bool) -> Result<(FileContents, FrameIndex), String> {
    let fc = compression::read_file_contents(path).map_err(|e| e.to_string())?;
    let idx = {
        let text = fc.as_str_sliced().map_err(|e| e.to_string())?;
        let mapped = matches!(fc, FileContents::Mapped(_));
        frame_index::load_or_build(path, text, write_sidecar && mapped)
            .map_err(|e| e.to_string())?
    };
    Ok((fc, idx))
}

fn pack_index(idx: &FrameIndex) -> Vec<u8> {
    let mut v = vec![idx.data_len];
    v.extend_from_slice(&idx.offsets);
    encode_u64s(&v)
}

fn unpack_index(b: &[u8]) -> FrameIndex {
    let v = decode_u64s(b);
    FrameIndex {
        data_len: v[0],
        offsets: v[1..].to_vec(),
    }
}

/// Byte range of frames `r` and their offsets relative to its start.
fn block_bytes(idx: &FrameIndex, r: &std::ops::Range<usize>) -> (usize, usize, Vec<usize>) {
    if r.is_empty() {
        return (0, 0, Vec::new());
    }
    let lo = idx.offsets[r.start] as usize;
    let hi = idx.range(r.end - 1).unwrap().end;
    let offs = idx.offsets[r.clone()]
        .iter()
        .map(|&o| o as usize - lo)
        .collect();
    (lo, hi, offs)
}

/// Read only `[lo, hi)` so a rank does not pull bytes another rank parses.
fn read_span(path: &Path, lo: usize, hi: usize) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len() as usize;
    if lo > hi || hi > len {
        return Err("byte range is outside the file".into());
    }
    file.seek(SeekFrom::Start(lo as u64))
        .map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; hi - lo];
    file.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Span and frame count when the first two frames share one length and
/// that length tiles the file. Later frames are checked by the parse of
/// each rank's own span, not by a walk of the whole file on rank 0.
fn tiled_span(path: &Path) -> Option<(usize, usize)> {
    let file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len() as usize;
    if len < 2 {
        return None;
    }
    // SAFETY: the file is not truncated while this map is alive.
    let map = unsafe { memmap2::Mmap::map(&file).ok()? };
    let mut iter = crate::iterators::ConFrameIterator::from_bytes(&map);
    if !matches!(iter.forward_fast(), Some(Ok(()))) {
        return None;
    }
    let span = iter.byte_pos();
    // A frame that parsed is longer than one byte, so the remainder is safe.
    if len % span != 0 {
        return None;
    }
    let n = len / span;
    if n >= 2 {
        let mut next = crate::iterators::ConFrameIterator::from_bytes(&map[span..span * 2]);
        if !matches!(next.forward_fast(), Some(Ok(()))) || next.byte_pos() != span {
            return None;
        }
    }
    Some((span, n))
}

/// Equal-length frames: every rank, including rank 0, reads only its span.
///
/// `Ok(None)` means the file is not one frame length and the caller keeps
/// the index path. Every rank takes the same branch.
fn read_tiled(
    comm: &Comm,
    path: &Path,
    threads: Option<usize>,
) -> Result<Option<RankFrames>, BoxErr> {
    let local = tiled_span(path);
    let packed = if comm.rank() == 0 {
        match local {
            Some((span, n)) => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend_from_slice(&(span as u64).to_le_bytes());
                bytes.extend_from_slice(&(n as u64).to_le_bytes());
                Ok(bytes)
            }
            None => Err("frames do not share one length".into()),
        }
    } else {
        Ok(Vec::new())
    };
    let decided = match bcast_result(comm, packed) {
        Ok(bytes) if bytes.len() == 16 => Some((
            u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize,
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize,
        )),
        _ => None,
    };
    let Some((span, n)) = decided else {
        return Ok(None);
    };
    let r = comm.my_frames(n);
    let lo = r.start.saturating_mul(span);
    let hi = r.end.saturating_mul(span);
    let bytes = read_span(path, lo, hi)
        .map_err(|e| -> BoxErr { format!("rank {}: {e}", comm.rank()).into() })?;
    let offs: Vec<usize> = (0..r.len()).map(|i| i * span).collect();
    let frames = parse_local(comm, &bytes, &offs, threads)?;
    Ok(Some(RankFrames {
        first: r.start,
        n_total: n,
        frames,
    }))
}

fn read_shared(comm: &Comm, path: &Path, threads: Option<usize>) -> Result<RankFrames, BoxErr> {
    if let Some(done) = read_tiled(comm, path, threads)? {
        return Ok(done);
    }
    // Unequal frames: rank 0 builds the index, then every rank, including
    // rank 0, reads only its own byte range. The full buffer is not kept
    // for the parse.
    let packed = if comm.rank() == 0 {
        match root_index(path, true) {
            Ok((_fc, idx)) => Ok(pack_index(&idx)),
            Err(e) => Err(e),
        }
    } else {
        Ok(Vec::new())
    };
    let idx = unpack_index(&bcast_result(comm, packed)?);
    let r = comm.my_frames(idx.len());
    let (lo, hi, offs) = block_bytes(&idx, &r);
    let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let opened = if len == idx.data_len {
        read_span(path, lo, hi)
    } else {
        Err("file differs from rank 0's copy; use ReadMode::Scatter".to_string())
    };
    let bytes = opened
        .map_err(|e| -> BoxErr { format!("rank {}: {e}", comm.rank()).into() })?;
    let frames = parse_local(comm, &bytes, &offs, threads)?;
    Ok(RankFrames {
        first: r.start,
        n_total: idx.len(),
        frames,
    })
}

fn flock_wait(fd: std::os::fd::RawFd, op: i32) -> bool {
    loop {
        if unsafe { libc::flock(fd, op) } == 0 {
            return true;
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return false;
        }
    }
}

/// True when a non-blocking shared lock fails because another holder has
/// the exclusive lock. A successful lock means `flock` did not see that
/// holder, so the caller releases it and reports the lock as not coherent.
fn flock_sees_exclusive(fd: std::os::fd::RawFd) -> bool {
    // LOCK_NB does not wait, so this call does not retry EINTR.
    // flock(2) reserves EINTR for "While waiting to acquire a lock" and
    // uses EWOULDBLOCK when LOCK_NB is set. Linux 6.12 fs/locks.c:2172
    // sets FL_SLEEP only when LOCK_NB is clear. flock_lock_inode returns
    // -EAGAIN at fs/locks.c:1121 without sleeping. The interruptible wait
    // is flock_lock_inode_wait, fs/locks.c:2087. flock_wait still retries.
    if unsafe { libc::flock(fd, libc::LOCK_SH | libc::LOCK_NB) } == 0 {
        let _ = unsafe { libc::flock(fd, libc::LOCK_UN) };
        return false;
    }
    std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
}

/// Block in the kernel until rank 0 has finished the file scan.
///
/// Rank 0 holds an exclusive lock across the scan. The other ranks take
/// a shared lock, which waits until that exclusive lock is released.
fn wait_until_root_scanned(comm: &Comm, path: &Path, scan: impl FnOnce()) {
    let opened = std::fs::File::open(path);
    if !agree(comm, opened.is_ok()) {
        if comm.rank() == 0 {
            scan();
        }
        return;
    }
    let Ok(file) = opened else {
        return;
    };
    let fd = std::os::fd::AsRawFd::as_raw_fd(&file);
    if comm.rank() == 0 {
        flock_wait(fd, libc::LOCK_EX);
    }
    comm.raw().barrier();
    if comm.rank() == 0 {
        scan();
        let _ = unsafe { libc::flock(fd, libc::LOCK_UN) };
    } else {
        flock_wait(fd, libc::LOCK_SH);
        let _ = unsafe { libc::flock(fd, libc::LOCK_UN) };
    }
}

fn read_scatter(comm: &Comm, path: &Path, threads: Option<usize>) -> Result<RankFrames, BoxErr> {
    let p = comm.size();
    let mut root_data = None;
    let mut packed_slot = None;
    wait_until_root_scanned(comm, path, || {
        packed_slot = Some(root_index(path, false).and_then(|(fc, idx)| {
            // Scatterv counts are C ints.
            let too_big = (0..p).any(|k| {
                let (lo, hi, _) = block_bytes(&idx, &block_range(idx.len(), p, k));
                hi - lo > i32::MAX as usize
            });
            if too_big {
                return Err("a rank's block exceeds 2 GiB; use more ranks".into());
            }
            let b = pack_index(&idx);
            root_data = Some((fc, idx));
            Ok(b)
        }));
    });
    let packed = packed_slot.unwrap_or(Ok(Vec::new()));
    let idx = unpack_index(&bcast_result(comm, packed)?);
    let r = comm.my_frames(idx.len());
    let (lo, hi, offs) = block_bytes(&idx, &r);
    // Keep a real allocation even for an empty block: Open MPI rejects
    // the dangling pointer of an empty Vec as a receive buffer.
    let mut store = vec![0u8; (hi - lo).max(1)];
    let buf = &mut store[..hi - lo];

    let root = comm.raw().process_at_rank(0);
    if let Some((fc, _)) = &root_data {
        let mut counts = Vec::with_capacity(p);
        let mut displs = Vec::with_capacity(p);
        for k in 0..p {
            let (l, h, _) = block_bytes(&idx, &block_range(idx.len(), p, k));
            counts.push((h - l) as i32);
            displs.push(l.min(i32::MAX as usize) as i32);
        }
        // Displacements are C ints too, so files over 2 GiB use
        // point-to-point sends instead (same decision on every rank).
        let all = bytes_of(fc);
        if idx.data_len <= i32::MAX as u64 {
            let part = Partition::new(&all[..], &counts[..], &displs[..]);
            root.scatter_varcount_into_root(&part, buf);
        } else {
            for k in 1..p {
                let (l, h, _) = block_bytes(&idx, &block_range(idx.len(), p, k));
                comm.raw().process_at_rank(k as i32).send(&all[l..h]);
            }
            buf.copy_from_slice(&all[lo..hi]);
        }
    } else {
        let big = idx.data_len > i32::MAX as u64;
        if big {
            comm.raw().process_at_rank(0).receive_into(buf);
        } else {
            root.scatter_varcount_into(buf);
        }
    }
    drop(root_data);
    let frames = parse_local(comm, buf, &offs, threads)?;
    Ok(RankFrames {
        first: r.start,
        n_total: idx.len(),
        frames,
    })
}

/// C ABI: read this rank's block of frames on a Fortran communicator.
///
/// `mode` is 0 (auto), 1 (shared file system) or 2 (scatter from rank
/// 0). `n_threads == 0` uses the global rayon pool. On success returns
/// the frame array (free with `free_rkr_frame_array`) and writes the
/// local count, the global index of the first local frame and the total
/// frame count. Returns NULL on every rank if any rank fails.
///
/// # Safety
/// MPI must be initialized; `comm_f` must be a live intra-communicator;
/// `path_c` must be a NUL-terminated string; the out pointers must be
/// valid for writes. Every rank of the communicator must call this.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rkr_mpi_read_block_f(
    comm_f: ::mpi::ffi::RSMPI_Fint,
    path_c: *const std::ffi::c_char,
    mode: i32,
    n_threads: usize,
    num_frames: *mut usize,
    first_frame: *mut u64,
    total_frames: *mut u64,
) -> *mut *mut crate::ffi::RKRConFrame {
    let comm = match unsafe { Comm::dup_fint(comm_f) } {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    // Argument checks are agreed collectively so no rank is left waiting.
    let args = (!path_c.is_null()
        && !num_frames.is_null()
        && !first_frame.is_null()
        && !total_frames.is_null())
    .then(|| unsafe { std::ffi::CStr::from_ptr(path_c) }.to_str().ok())
    .flatten()
    .zip(ReadMode::from_i32(mode));
    if !agree(&comm, args.is_some()) {
        return std::ptr::null_mut();
    }
    let (path, mode) = args.unwrap();
    let threads = (n_threads > 0).then_some(n_threads);
    match read_block(&comm, Path::new(path), mode, threads) {
        Ok(rf) => unsafe {
            *first_frame = rf.first as u64;
            *total_frames = rf.n_total as u64;
            crate::ffi::pack_frame_handles(rf.frames, num_frames)
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(all(test, feature = "mpi"))]
mod tests {
    use super::*;
    use crate::distrib::test_universe;
    use crate::frame_index::FrameIndex;
    use mpi::raw::AsRaw;
    use std::ffi::CString;

    fn flush_coverage_profile() {
        #[cfg(coverage)]
        unsafe {
            unsafe extern "C" {
                fn __llvm_profile_write_file() -> i32;
            }
            let _ = __llvm_profile_write_file();
        }
    }

    fn world_comm() -> Comm {
        Comm::dup_of(&test_universe().world())
    }

    fn fixture() -> String {
        std::fs::read_to_string("resources/test/tiny_cuh2.con").unwrap()
    }

    fn alarm_while_a_lock_is_held(path: &std::path::Path) {
        let held = std::fs::File::open(path).unwrap();
        let waiter = std::fs::File::open(path).unwrap();
        let held_fd = std::os::fd::AsRawFd::as_raw_fd(&held);
        let wait_fd = std::os::fd::AsRawFd::as_raw_fd(&waiter);
        assert!(flock_wait(held_fd, libc::LOCK_EX));
        unsafe extern "C" fn on_alarm(_: libc::c_int) {}
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = on_alarm as *const () as libc::sighandler_t;
            libc::sigemptyset(&mut action.sa_mask);
            assert_eq!(
                libc::sigaction(libc::SIGALRM, &action, std::ptr::null_mut()),
                0
            );
        }
        let waiting = unsafe { libc::pthread_self() };
        let unlock = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            unsafe {
                libc::flock(held_fd, libc::LOCK_UN);
            }
        });
        let poker = std::thread::spawn(move || {
            for _ in 0..120 {
                std::thread::sleep(std::time::Duration::from_millis(5));
                unsafe {
                    libc::pthread_kill(waiting, libc::SIGALRM);
                }
            }
        });
        assert!(
            flock_wait(wait_fd, libc::LOCK_EX),
            "an alarm while waiting for the lock is retried"
        );
        let probe = std::fs::File::open(path).unwrap();
        let probe_fd = std::os::fd::AsRawFd::as_raw_fd(&probe);
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_millis(300) {
            assert!(
                flock_sees_exclusive(probe_fd),
                "an alarm during the probe still reports the held lock"
            );
        }
        unlock.join().unwrap();
        poker.join().unwrap();
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = libc::SIG_DFL;
            libc::sigaction(libc::SIGALRM, &action, std::ptr::null_mut());
            libc::flock(wait_fd, libc::LOCK_UN);
        }
    }

    #[test]
    fn one_rank_reads_each_mode_and_the_export() {
        let comm = world_comm();
        let dir = tempfile::tempdir().unwrap();
        let text = fixture();
        let one = dir.path().join("one.con");
        let two = dir.path().join("two.con");
        let extra = dir.path().join("extra.con");
        let short = dir.path().join("short.con");
        let garbage = dir.path().join("garbage.con");
        std::fs::write(&one, &text).unwrap();
        std::fs::write(&two, format!("{text}{text}")).unwrap();
        std::fs::write(&extra, format!("{text}x")).unwrap();
        std::fs::write(&short, "x").unwrap();
        std::fs::write(&garbage, "this is not a con frame").unwrap();

        assert!(tiled_span(&short).is_none());
        assert!(tiled_span(&garbage).is_none());
        assert!(tiled_span(&dir.path().join("missing.con")).is_none());
        assert_eq!(tiled_span(&one).unwrap().1, 1);
        assert_eq!(tiled_span(&two).unwrap().1, 2);
        assert!(tiled_span(&extra).is_none());
        assert!(read_span(&one, 4, 1).is_err());
        let past = read_span(&one, 0, text.len() + 8).unwrap_err();
        assert!(past.contains("outside"), "{past}");
        assert!(read_span(&dir.path().join("missing.con"), 0, 1).is_err());
        let bytes = read_span(&one, 0, 4).unwrap();
        assert_eq!(bytes.len(), 4);
        let mut broken = format!("{text}{text}");
        let start = text.len();
        broken.replace_range(start.., &"x".repeat(text.len()));
        let broken_path = dir.path().join("broken-tile.con");
        std::fs::write(&broken_path, &broken).unwrap();
        assert!(
            tiled_span(&broken_path).is_none(),
            "a same-length second block that is not a frame is not one tile"
        );
        let (span, n_one) = tiled_span(&one).unwrap();
        assert_eq!(n_one, 1);
        let mut short_bytes = Vec::new();
        {
            let mut atom =
                crate::types::ConFrameBuilder::new([1.0, 1.0, 1.0], [90.0, 90.0, 90.0]);
            atom.add_atom("H", 0.0, 0.0, 0.0, [false; 3], 1, 1.0);
            let atom = atom.build().unwrap();
            let mut writer = crate::writer::ConFrameWriter::new(&mut short_bytes);
            writer.write_frame(&atom).unwrap();
        }
        assert!(
            short_bytes.len() < span,
            "the second frame has to be shorter than the span"
        );
        let mut window = std::fs::read(&one).unwrap();
        window.extend_from_slice(&short_bytes);
        window.extend(std::iter::repeat(b'x').take(span - short_bytes.len()));
        assert_eq!(window.len(), span * 2);
        let window_path = dir.path().join("short-window.con");
        std::fs::write(&window_path, &window).unwrap();
        assert!(
            tiled_span(&window_path).is_none(),
            "a shorter frame in the next span is not one tile"
        );
        let pipe = dir.path().join("range.pipe");
        let pipe_c = std::ffi::CString::new(pipe.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(pipe_c.as_ptr(), 0o600) }, 0);
        let pipe_fd = unsafe { libc::open(pipe_c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
        assert!(pipe_fd >= 0, "the pipe has to open");
        let seek_err = read_span(&pipe, 0, 0).unwrap_err();
        assert!(seek_err.contains("29"), "{seek_err}");
        unsafe { libc::close(pipe_fd) };
        alarm_while_a_lock_is_held(&one);

        let idx = FrameIndex {
            data_len: 10,
            offsets: vec![0, 4, 10],
        };
        let (lo, hi, offs) = block_bytes(&idx, &(0..0));
        assert_eq!((lo, hi), (0, 0));
        assert!(offs.is_empty());
        let (lo, hi, offs) = block_bytes(&idx, &(1..3));
        assert_eq!(lo, 4);
        assert_eq!(hi, 10);
        assert_eq!(offs, vec![0, 6]);
        let packed = pack_index(&idx);
        let back = unpack_index(&packed);
        assert_eq!(back.offsets, idx.offsets);
        assert_eq!(back.data_len, idx.data_len);

        for mode in [ReadMode::Auto, ReadMode::Shared, ReadMode::Scatter] {
            let got = read_block(&comm, &two, mode, Some(1)).expect("read");
            assert_eq!(got.n_total, 2);
            assert_eq!(got.first, 0);
            assert_eq!(got.frames.len(), 2);
        }
        assert!(read_block(&comm, &dir.path().join("missing.con"), ReadMode::Auto, None).is_err());
        assert!(read_block(&comm, &garbage, ReadMode::Shared, None).is_err());
        assert_eq!(ReadMode::from_i32(2), Some(ReadMode::Scatter));
        assert_eq!(
            crate::distrib_write::WriteMode::from_i32(0),
            Some(crate::distrib_write::WriteMode::Auto)
        );
        assert_eq!(
            crate::distrib_write::WriteMode::from_i32(1),
            Some(crate::distrib_write::WriteMode::MpiIo)
        );
        assert!(!flock_wait(-1, libc::LOCK_SH));
        let lock_a = std::fs::File::open(&one).unwrap();
        let lock_b = std::fs::File::open(&one).unwrap();
        assert!(flock_wait(
            std::os::fd::AsRawFd::as_raw_fd(&lock_a),
            libc::LOCK_EX
        ));
        assert!(flock_sees_exclusive(std::os::fd::AsRawFd::as_raw_fd(
            &lock_b
        )));
        unsafe {
            libc::flock(
                std::os::fd::AsRawFd::as_raw_fd(&lock_a),
                libc::LOCK_UN,
            );
        }
        assert!(!flock_sees_exclusive(std::os::fd::AsRawFd::as_raw_fd(
            &lock_b
        )));
        let mut one_atom = crate::types::ConFrameBuilder::new([1.0, 1.0, 1.0], [90.0, 90.0, 90.0]);
        one_atom.add_atom("H", 0.0, 0.0, 0.0, [false; 3], 1, 1.0);
        let one_atom = one_atom.build().unwrap();
        let mut raw = Vec::new();
        {
            let mut writer = crate::writer::ConFrameWriter::new(&mut raw);
            writer.write_frame(&one_atom).unwrap();
        }
        let mixed = dir.path().join("mixed.con");
        std::fs::write(&mixed, format!("{text}{}", String::from_utf8(raw).unwrap())).unwrap();
        for mode in [ReadMode::Shared, ReadMode::Scatter] {
            let got = read_block(&comm, &mixed, mode, None).expect("unequal frames");
            assert_eq!(got.n_total, 2, "{mode:?}");
            assert_eq!(got.frames.len(), 2, "{mode:?}");
        }
        let mut bulky = String::new();
        while bulky.len() < 70_000 {
            bulky.push_str(&text);
        }
        let bulky_path = dir.path().join("bulky.con");
        std::fs::write(&bulky_path, bulky).unwrap();
        let mapped = read_block(&comm, &bulky_path, ReadMode::Scatter, Some(1)).expect("mapped");
        assert!(mapped.n_total > 2);

        let path_c = CString::new(two.to_str().unwrap()).unwrap();
        let fint = unsafe { mpi::ffi::RSMPI_Comm_c2f(test_universe().world().as_raw()) };
        let mut n = 0usize;
        let mut first = 0u64;
        let mut total = 0u64;
        let ptr = unsafe {
            rkr_mpi_read_block_f(fint, path_c.as_ptr(), 1, 1, &mut n, &mut first, &mut total)
        };
        assert!(!ptr.is_null(), "shared export returned null");
        assert_eq!(total, 2);
        unsafe { crate::ffi::free_rkr_frame_array(ptr, n) };
        let nulls = unsafe {
            rkr_mpi_read_block_f(
                fint,
                std::ptr::null(),
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(nulls.is_null());
        let mut n2 = 0usize;
        let no_outs = unsafe {
            rkr_mpi_read_block_f(
                fint,
                path_c.as_ptr(),
                0,
                0,
                &mut n2,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(no_outs.is_null());
        let bad_mode = unsafe {
            rkr_mpi_read_block_f(fint, path_c.as_ptr(), 9, 0, &mut n2, &mut first, &mut total)
        };
        assert!(bad_mode.is_null());

        let out = dir.path().join("written.con");
        let out_c = CString::new(out.to_str().unwrap()).unwrap();
        let mut total_bytes = 0u64;
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                std::ptr::null(),
                std::ptr::null(),
                0,
                2,
                -1,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc, -1);
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                out_c.as_ptr(),
                std::ptr::null(),
                1,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(rc, -1);
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                out_c.as_ptr(),
                std::ptr::null(),
                0,
                9,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(rc, -1);
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                out_c.as_ptr(),
                std::ptr::null(),
                0,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(rc, 0, "an empty gather write should succeed");
    }

    #[test]
    fn two_ranks_read_and_write_every_mode() {
        const CHILD: &str = "READCON_TWO_RANK_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let dir = tempfile::tempdir().unwrap();
            let text = fixture();
            std::fs::write(dir.path().join("two.con"), format!("{text}{text}")).unwrap();
            let mut one_atom =
                crate::types::ConFrameBuilder::new([1.0, 1.0, 1.0], [90.0, 90.0, 90.0]);
            one_atom.add_atom("H", 0.0, 0.0, 0.0, [false; 3], 1, 1.0);
            let one_atom = one_atom.build().unwrap();
            let mut raw = Vec::new();
            {
                let mut writer = crate::writer::ConFrameWriter::new(&mut raw);
                writer.write_frame(&one_atom).unwrap();
            }
            std::fs::write(
                dir.path().join("mixed.con"),
                format!("{text}{}", String::from_utf8(raw).unwrap()),
            )
            .unwrap();
            let exe = std::env::current_exe().unwrap();
            let env_library = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
            let maps_missing = dir.path().join("no-maps");
            let bare_maps = dir.path().join("bare-maps");
            std::fs::write(&bare_maps, "libmetatensor.so\n").unwrap();
            let injected = format!(
                "00000000-00000000 r--p 00000000 00:00 0 {}",
                dir.path().join("libmetatensor.so").display()
            );
            let mut library_path = env_library.clone();
            let mut saw_blank = false;
            let mut saw_missing_maps = false;
            let mut saw_colon = false;
            let mut saw_empty_seed = false;
            let mut saw_bare_library = false;
            let mut saw_no_directory = false;
            for (seed, keep) in [
                (env_library.clone(), true),
                ("already-set".to_string(), false),
                (String::new(), false),
            ] {
                let mut built = seed;
                for maps_path in [
                    "/proc/self/maps",
                    maps_missing.to_str().unwrap(),
                    bare_maps.to_str().unwrap(),
                ] {
                    if let Ok(maps) = std::fs::read_to_string(maps_path) {
                        let mut lines = vec![String::new(), "libmetatensor.so".to_string()];
                        lines.extend(maps.lines().map(str::to_string));
                        if !maps_path.ends_with("bare-maps") {
                            lines.push(injected.clone());
                        }
                        let mut dir = None;
                        for line in &lines {
                            let Some(path) = line.split_whitespace().last() else {
                                assert!(
                                    line.split_whitespace().next().is_none(),
                                    "a blank maps line has no path"
                                );
                                saw_blank = true;
                                continue;
                            };
                            if path.ends_with("libmetatensor.so") {
                                match path.rfind('/') {
                                    Some(cut) => {
                                        if dir.is_none() {
                                            dir = Some(path[..cut].to_string());
                                        }
                                    }
                                    None => saw_bare_library = true,
                                }
                            }
                        }
                        if let Some(dir) = dir {
                            if !built.is_empty() {
                                built.push(':');
                                saw_colon = true;
                            } else {
                                saw_empty_seed = true;
                            }
                            built.push_str(&dir);
                        } else {
                            saw_no_directory = true;
                        }
                    } else {
                        saw_missing_maps = true;
                        assert!(
                            maps_path.ends_with("no-maps"),
                            "only the absent maps file fails to open, got {maps_path}"
                        );
                    }
                }
                if keep {
                    library_path = built;
                }
            }
            assert!(saw_blank, "a blank maps line is skipped");
            assert!(saw_missing_maps, "a missing maps file is an error");
            assert!(saw_colon, "a seeded library path keeps a separator");
            assert!(
                saw_empty_seed,
                "an empty library path does not grow a leading colon"
            );
            assert!(
                saw_bare_library,
                "a library name with no slash adds no directory"
            );
            assert!(
                saw_no_directory,
                "maps that name only the bare library add no directory"
            );
            assert!(!library_path.starts_with(':'));
            unsafe {
                std::env::set_var("OMPI_TEST_MARKER", "1");
                std::env::set_var("PMIX_TEST_MARKER", "1");
                std::env::set_var("PMI_TEST_MARKER", "1");
            }
            let saved_profile = std::env::var_os("LLVM_PROFILE_FILE");
            let mut saw_profile = false;
            let mut saw_no_profile = false;
            for candidate in [saved_profile.clone(), None, saved_profile.clone()] {
                if let Some(value) = candidate {
                    unsafe { std::env::set_var("LLVM_PROFILE_FILE", &value) };
                    assert_eq!(
                        std::env::var_os("LLVM_PROFILE_FILE").as_deref(),
                        Some(value.as_os_str()),
                        "a saved profile is restored"
                    );
                    saw_profile = true;
                } else {
                    unsafe { std::env::remove_var("LLVM_PROFILE_FILE") };
                    assert!(
                        std::env::var_os("LLVM_PROFILE_FILE").is_none(),
                        "clearing the profile leaves it unset"
                    );
                    saw_no_profile = true;
                }
            }
            assert!(saw_no_profile, "an unset profile stays unset");
            assert_eq!(saw_profile, saved_profile.is_some());
            let log_out = dir.path().join("mpirun.out");
            let log_err = dir.path().join("mpirun.err");
            let mut launch = None;
            for present in [true, false, saved_profile.is_some()] {
                if present {
                    let dummy = std::ffi::OsString::from("target/cov-live/held.profraw");
                    let value = saved_profile.clone().unwrap_or(dummy);
                    unsafe { std::env::set_var("LLVM_PROFILE_FILE", value) };
                } else {
                    unsafe { std::env::remove_var("LLVM_PROFILE_FILE") };
                }
                let mut cmd = std::process::Command::new("mpirun");
                cmd.args([
                    "--oversubscribe",
                    "-np",
                    "2",
                    "-x",
                    "LD_LIBRARY_PATH",
                    "-x",
                    "READCON_TWO_RANK_CHILD",
                    "-x",
                    "READCON_TWO_RANK_DIR",
                    "-x",
                    "READCON_MPI_TEST_CHUNK",
                ]);
                if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
                    cmd.args(["-x", "LLVM_PROFILE_FILE"]);
                }
                let forwards = cmd.get_args().any(|arg| arg == "LLVM_PROFILE_FILE");
                assert_eq!(
                    forwards, present,
                    "the profile flag follows the variable"
                );
                if present == saved_profile.is_some() && launch.is_none() {
                    launch = Some(cmd);
                }
            }
            assert_eq!(
                std::env::var_os("LLVM_PROFILE_FILE").is_some(),
                saved_profile.is_some(),
                "the child inherits the profile that was set on entry"
            );
            let mut cmd = launch.expect("the launch command matches the saved profile");
            cmd.arg(&exe)
            .args([
                "distrib_read::tests::two_ranks_read_and_write_every_mode",
                "--exact",
                "--test-threads=1",
                "--nocapture",
            ]);
            cmd
            .env(CHILD, "1")
            .env("READCON_MPI_TEST_CHUNK", "64")
            .env("READCON_TWO_RANK_DIR", dir.path())
            .env("LD_LIBRARY_PATH", &library_path)
            .env_remove("LD_PRELOAD");
            for (key, _) in std::env::vars() {
                if key == "OMPI_ALLOW_RUN_AS_ROOT" || key == "OMPI_ALLOW_RUN_AS_ROOT_CONFIRM" {
                    continue;
                }
                if key.starts_with("OMPI_") || key.starts_with("PMIX_") || key.starts_with("PMI_")
                {
                    cmd.env_remove(key);
                }
            }
            let removed = cmd
                .get_envs()
                .filter_map(|(key, value)| value.is_none().then_some(key.to_string_lossy().into_owned()))
                .collect::<Vec<_>>();
            assert!(removed.iter().any(|key| key == "OMPI_TEST_MARKER"), "{removed:?}");
            assert!(removed.iter().any(|key| key == "PMIX_TEST_MARKER"), "{removed:?}");
            assert!(removed.iter().any(|key| key == "PMI_TEST_MARKER"), "{removed:?}");
            let status = cmd
                .stdout(std::fs::File::create(&log_out).unwrap())
                .stderr(std::fs::File::create(&log_err).unwrap())
                .status()
                .expect("mpirun");
            let stdout = std::fs::read_to_string(&log_out).unwrap_or_default();
            let stderr = std::fs::read_to_string(&log_err).unwrap_or_default();
            assert!(
                status.success(),
                "two-rank job failed ({status:?})\n{stdout}\n{stderr}"
            );
            return;
        }
        let universe = ::mpi::initialize().expect("MPI_Init");
        let comm = Comm::dup_of(&universe.world());
        assert_eq!(comm.size(), 2, "the child must be launched with two ranks");
        let dir = std::path::PathBuf::from(std::env::var("READCON_TWO_RANK_DIR").unwrap());
        let path = dir.join("two.con");
        let mixed = dir.join("mixed.con");
        for mode in [ReadMode::Shared, ReadMode::Scatter, ReadMode::Auto] {
            let got = read_block(&comm, &path, mode, None).expect("collective read");
            assert_eq!(got.n_total, 2);
            assert_eq!(got.frames.len(), 1);
        }
        for mode in [ReadMode::Shared, ReadMode::Scatter] {
            let got = read_block(&comm, &mixed, mode, None).expect("unequal frames");
            assert_eq!(got.n_total, 2, "{mode:?}");
            assert_eq!(got.frames.len(), 1, "{mode:?}");
        }
        let frames = read_block(&comm, &path, ReadMode::Shared, None)
            .unwrap()
            .frames;
        let blob = comm.bcast_bytes(0, b"frame");
        assert_eq!(blob, b"frame");
        for (name, mode) in [
            ("gather.con", crate::distrib_write::WriteMode::Gather),
            ("mpiio.con", crate::distrib_write::WriteMode::MpiIo),
            ("auto.con", crate::distrib_write::WriteMode::Auto),
        ] {
            let n = crate::distrib_write::write_block(&comm, &dir.join(name), &frames, mode, Some(6))
                .expect(name);
            assert!(n > 0, "{name}");
        }
        let missing = dir.join("no-such-dir").join("out.con");
        let err = crate::distrib_write::write_block(
            &comm,
            &missing,
            &frames,
            crate::distrib_write::WriteMode::Gather,
            None,
        );
        assert!(err.is_err(), "a missing directory must fail the gather");
        let err = crate::distrib_write::write_block(
            &comm,
            &dir,
            &frames,
            crate::distrib_write::WriteMode::Gather,
            None,
        );
        assert!(err.is_err(), "a directory path must fail the gather");

        // A chunk larger than the file takes the single Gatherv path.
        unsafe { std::env::set_var("READCON_MPI_TEST_CHUNK", "1048576") };
        let gathered = dir.join("gatherv.con");
        let n = crate::distrib_write::write_block(
            &comm,
            &gathered,
            &frames,
            crate::distrib_write::WriteMode::Gather,
            None,
        )
        .expect("gatherv");
        assert!(n > 16, "{n}");
        let text = std::fs::read_to_string(&gathered).unwrap();
        assert!(text.contains("Cu"), "{text}");
        assert!(text.contains("H"), "{text}");

        // Rank 1 contributes no frames, so its write rounds are empty.
        unsafe { std::env::set_var("READCON_MPI_TEST_CHUNK", "64") };
        let mine = if comm.rank() == 0 {
            frames.clone()
        } else {
            Vec::new()
        };
        let partial = dir.join("partial.con");
        let n = crate::distrib_write::write_block(
            &comm,
            &partial,
            &mine,
            crate::distrib_write::WriteMode::MpiIo,
            None,
        )
        .expect("partial");
        assert!(n > 16, "{n}");
        let back = read_block(&comm, &partial, ReadMode::Shared, None).expect("reread");
        assert_eq!(back.n_total, frames.len(), "rank {}", comm.rank());

        let mut bad = frames[0].clone();
        bad.header.natm_types = 0;
        bad.header.natms_per_type.clear();
        bad.header.masses_per_type.clear();
        let mine = if comm.rank() == 0 {
            frames.clone()
        } else {
            vec![bad]
        };
        let err = crate::distrib_write::write_block(
            &comm,
            &dir.join("bad.con"),
            &mine,
            crate::distrib_write::WriteMode::Gather,
            None,
        )
        .expect_err("a zero type count must fail");
        let msg = err.to_string();
        if comm.rank() == 0 {
            assert!(msg.contains("another rank"), "{msg}");
        } else {
            assert!(msg.contains("per-type"), "{msg}");
        }

        let full = std::path::Path::new("/dev/full");
        let err = crate::distrib_write::write_block(
            &comm,
            full,
            &frames,
            crate::distrib_write::WriteMode::Gather,
            None,
        )
        .expect_err("a full device must reject the write");
        assert!(
            err.to_string().contains("space"),
            "{err}"
        );

        let bad_name = {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(b"nul\0name".to_vec())
        };
        let err = crate::distrib_write::write_block(
            &comm,
            std::path::Path::new(&bad_name),
            &frames,
            crate::distrib_write::WriteMode::MpiIo,
            None,
        )
        .expect_err("a name with an interior nul must fail");
        assert!(
            err.to_string().to_lowercase().contains("nul"),
            "{err}"
        );

        let err = crate::distrib_write::write_block(
            &comm,
            &dir,
            &frames,
            crate::distrib_write::WriteMode::MpiIo,
            None,
        )
        .expect_err("MPI-IO on a directory must fail");
        assert!(err.to_string().contains("MPI"), "{err}");

        unsafe { std::env::set_var("READCON_MPI_TEST_CHUNK", "64") };
        let abi = dir.join("abi.con");
        let abi_c = std::ffi::CString::new(abi.to_str().unwrap()).unwrap();
        let fint = unsafe { mpi::ffi::RSMPI_Comm_c2f(comm.raw().as_raw()) };
        let mut total_bytes = 0u64;
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                abi_c.as_ptr(),
                std::ptr::null(),
                0,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(rc, 0, "an empty ABI write should succeed");
        assert_eq!(total_bytes, 0, "no frames means an empty file");
        assert!(
            !frame_index::sidecar_path(&abi).is_file(),
            "an empty write removes the sidecar"
        );

        let held = &frames[0] as *const crate::types::ConFrame as *const crate::ffi::RKRConFrame;
        let slots = [held];
        let disagree = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                abi_c.as_ptr(),
                if comm.rank() == 0 {
                    std::ptr::null()
                } else {
                    slots.as_ptr()
                },
                1,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(disagree, -1, "a null frame pointer must fail every rank");
        let disagree = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                if comm.rank() == 0 {
                    std::ptr::null()
                } else {
                    abi_c.as_ptr()
                },
                std::ptr::null(),
                0,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(disagree, -1, "a null path must fail every rank");
        let disagree = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                abi_c.as_ptr(),
                std::ptr::null(),
                0,
                2,
                -1,
                if comm.rank() == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut total_bytes
                },
            )
        };
        assert_eq!(disagree, -1, "a null size pointer must fail every rank");
        let null_slot: *const crate::ffi::RKRConFrame = std::ptr::null();
        let bad_slots = [null_slot];
        let good_slots = [held];
        let list = if comm.rank() == 0 {
            bad_slots.as_ptr()
        } else {
            good_slots.as_ptr()
        };
        let disagree = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                abi_c.as_ptr(),
                list,
                1,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(disagree, -1, "a null frame in the list must fail every rank");
        let dir_c = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
        let rejected = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                dir_c.as_ptr(),
                slots.as_ptr(),
                1,
                2,
                6,
                &mut total_bytes,
            )
        };
        assert_eq!(rejected, -1, "the C writer rejects a directory");
        let ok_path = dir.join("abi-ok.con");
        let ok_c = std::ffi::CString::new(ok_path.to_str().unwrap()).unwrap();
        let wrote = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                fint,
                ok_c.as_ptr(),
                slots.as_ptr(),
                1,
                2,
                6,
                &mut total_bytes,
            )
        };
        assert_eq!(wrote, 0, "the C writer stores one frame from each rank");
        assert!(total_bytes > 16, "{total_bytes}");
        let stored = std::fs::read_to_string(&ok_path).unwrap();
        assert!(stored.contains("Cu"), "{stored}");
        assert!(stored.contains("H"), "{stored}");

        unsafe {
            mpi::ffi::MPI_Comm_set_errhandler(
                mpi::ffi::RSMPI_COMM_WORLD,
                mpi::ffi::RSMPI_ERRORS_RETURN,
            );
        }
        let null_comm = unsafe { mpi::ffi::RSMPI_Comm_c2f(mpi::ffi::RSMPI_COMM_NULL) };
        let rc = unsafe {
            crate::distrib_write::rkr_mpi_write_block_f(
                null_comm,
                ok_c.as_ptr(),
                std::ptr::null(),
                0,
                2,
                -1,
                &mut total_bytes,
            )
        };
        assert_eq!(rc, -1, "a null communicator must fail the C writer");
        let mut n = 0usize;
        let mut first = 0u64;
        let mut total_frames = 0u64;
        let ptr = unsafe {
            rkr_mpi_read_block_f(
                null_comm,
                ok_c.as_ptr(),
                1,
                0,
                &mut n,
                &mut first,
                &mut total_frames,
            )
        };
        assert!(ptr.is_null(), "a null communicator must fail the C reader");
        unsafe {
            mpi::ffi::MPI_Comm_set_errhandler(
                mpi::ffi::RSMPI_COMM_WORLD,
                mpi::ffi::RSMPI_ERRORS_ARE_FATAL,
            );
        }

        let err = crate::distrib_write::write_block(
            &comm,
            std::path::Path::new("/dev/null"),
            &frames,
            crate::distrib_write::WriteMode::Gather,
            None,
        )
        .expect_err("sync on the null device must fail");
        assert!(
            err.to_string().contains("22"),
            "{err}"
        );
        unsafe { std::env::set_var("READCON_MPI_TEST_CHUNK", "8") };
        let err = crate::distrib_write::write_block(
            &comm,
            std::path::Path::new("/dev/full"),
            &frames,
            crate::distrib_write::WriteMode::MpiIo,
            None,
        )
        .expect_err("MPI-IO to a full device must fail");
        assert!(err.to_string().contains("set_size"), "{err}");

        unsafe { std::env::set_var("READCON_MPI_TEST_CHUNK", "1048576") };
        let mine = if comm.rank() == 0 {
            frames.clone()
        } else {
            Vec::new()
        };
        let partial_gather = dir.join("one-rank-gather.con");
        let n = crate::distrib_write::write_block(
            &comm,
            &partial_gather,
            &mine,
            crate::distrib_write::WriteMode::Gather,
            None,
        )
        .expect("one rank gathers");
        assert!(n > 16, "{n}");
        let gathered = std::fs::read_to_string(&partial_gather).unwrap();
        assert!(gathered.contains("Cu"), "{gathered}");
        assert!(gathered.contains("H"), "{gathered}");

        let one = dir.join("one-frame.con");
        let uneven = dir.join("uneven.con");
        let tiny = dir.join("one-byte.con");
        let gz = dir.join("two.con.gz");
        if comm.rank() == 0 {
            let text = std::fs::read(&path).unwrap();
            let half = text.len() / 2;
            std::fs::write(&one, &text[..half]).unwrap();
            let mut second = Vec::new();
            {
                let mut writer = crate::writer::ConFrameWriter::new(&mut second);
                writer.write_frame(&frames[0]).unwrap();
            }
            assert_ne!(second.len(), half, "the added frame must not tile the first");
            let mut body = text[..half].to_vec();
            body.extend_from_slice(&second);
            std::fs::write(&uneven, body).unwrap();
            std::fs::write(&tiny, b"x").unwrap();
            let mut enc = flate2::write::GzEncoder::new(
                std::fs::File::create(&gz).unwrap(),
                flate2::Compression::default(),
            );
            use std::io::Write;
            enc.write_all(&text).unwrap();
            enc.finish().unwrap();
        }
        comm.raw().barrier();

        let got = read_block(&comm, &one, ReadMode::Scatter, None).expect("one frame");
        assert_eq!(got.n_total, 1, "rank {}", comm.rank());
        assert_eq!(got.frames.len(), usize::from(comm.rank() == 0));
        let got = read_block(&comm, &uneven, ReadMode::Shared, None).expect("uneven");
        assert_eq!(got.n_total, 2, "rank {}", comm.rank());
        let err = read_block(&comm, &tiny, ReadMode::Shared, None).expect_err("one byte");
        assert!(!err.to_string().is_empty(), "{err}");
        let err = read_block(&comm, &gz, ReadMode::Shared, None).expect_err("gzip length");
        assert!(err.to_string().contains("differs"), "{err}");
        let got = read_block(&comm, &gz, ReadMode::Auto, None).expect("gzip scatters");
        assert_eq!(got.n_total, 2, "rank {}", comm.rank());
        let broken = dir.join("broken.con");
        if comm.rank() == 0 {
            let one_bytes = std::fs::read(&one).unwrap();
            let mut quad = one_bytes.repeat(4);
            let at = one_bytes.len() * 2 + one_bytes.len() / 2;
            quad[at] ^= 0x5f;
            std::fs::write(&broken, quad).unwrap();
        }
        comm.raw().barrier();
        let err = read_block(&comm, &broken, ReadMode::Shared, None)
            .expect_err("a corrupt later frame");
        let msg = err.to_string();
        if comm.rank() == 0 {
            assert!(msg.contains("another rank"), "{msg}");
        } else {
            assert!(msg.contains("rank 1"), "{msg}");
        }
        let err = read_block(&comm, &dir.join("absent.con"), ReadMode::Scatter, None)
            .expect_err("scatter of a missing file");
        assert!(err.to_string().contains("os error 2"), "{err}");

        let shorter = dir.join("shorter-second.con");
        let mismatched = dir.join("mismatched-second.con");
        if comm.rank() == 0 {
            let big = std::fs::read(&one).unwrap();
            let mut small = Vec::new();
            {
                let mut writer = crate::writer::ConFrameWriter::new(&mut small);
                writer.write_frame(&frames[0]).unwrap();
            }
            let mut first = Vec::new();
            let mut second = Vec::new();
            for (left, right) in [
                (big.clone(), small.clone()),
                (small.clone(), big.clone()),
            ] {
                let pair = if left.len() < right.len() {
                    (right, left)
                } else {
                    (left, right)
                };
                assert!(
                    pair.1.len() < pair.0.len(),
                    "the second frame must be shorter than the first"
                );
                first = pair.0;
                second = pair.1;
            }
            let mut body = first.clone();
            body.extend_from_slice(&second);
            std::fs::write(&shorter, &body).unwrap();
            let span = first.len();
            let mut window = first.clone();
            window.extend_from_slice(&second);
            window.extend(std::iter::repeat(b'x').take(span - second.len()));
            assert_eq!(window.len(), span * 2);
            std::fs::write(&mismatched, window).unwrap();
        }
        comm.raw().barrier();
        let got = read_block(&comm, &shorter, ReadMode::Shared, None)
            .expect("a shorter second frame is still two frames");
        assert_eq!(got.n_total, 2, "rank {}", comm.rank());
        assert!(
            tiled_span(&mismatched).is_none(),
            "rank {} sees a shorter frame in the next span",
            comm.rank()
        );
        let err = read_block(&comm, &mismatched, ReadMode::Shared, None)
            .expect_err("bytes after the shorter frame are not a frame");
        assert!(
            err.to_string().contains("file ended unexpectedly"),
            "{err}"
        );

        if comm.rank() == 0 {
            let pad = (i32::MAX as usize) / 2 + 8192;
            let mut bytes = Vec::new();
            {
                let mut first = frames[0].clone();
                let mut second = frames[0].clone();
                first.header.metadata.insert(
                    "pad".into(),
                    serde_json::Value::String("x".repeat(pad)),
                );
                second.header.metadata.insert(
                    "pad".into(),
                    serde_json::Value::String("y".repeat(pad)),
                );
                let mut writer = crate::writer::ConFrameWriter::new(&mut bytes);
                writer.write_frame(&first).unwrap();
                writer.write_frame(&second).unwrap();
            }
            assert!(bytes.len() > i32::MAX as usize, "{}", bytes.len());
            std::fs::write(dir.join("split-wide.con"), &bytes).unwrap();
            let mut huge = Vec::new();
            {
                let mut one_frame = frames[0].clone();
                one_frame.header.metadata.insert(
                    "pad".into(),
                    serde_json::Value::String("z".repeat((i32::MAX as usize) + 1)),
                );
                let mut writer = crate::writer::ConFrameWriter::new(&mut huge);
                writer.write_frame(&one_frame).unwrap();
            }
            assert!(huge.len() > i32::MAX as usize, "{}", huge.len());
            std::fs::write(dir.join("one-wide.con"), &huge).unwrap();
        }
        comm.raw().barrier();
        let split = read_block(
            &comm,
            &dir.join("split-wide.con"),
            ReadMode::Scatter,
            None,
        )
        .expect("two frames past 2 GiB together still fit on one rank");
        assert_eq!(split.n_total, 2, "rank {}", comm.rank());
        assert_eq!(split.frames.len(), 1, "rank {}", comm.rank());
        assert_eq!(
            split.frames[0].header.metadata["pad"]
                .as_str()
                .unwrap()
                .len(),
            (i32::MAX as usize) / 2 + 8192
        );
        match read_block(&comm, &dir.join("one-wide.con"), ReadMode::Scatter, None) {
            Err(err) => assert!(
                err.to_string().contains("2 GiB"),
                "{err}"
            ),
            Ok(got) => panic!(
                "one rank cannot take a block over 2 GiB, got {} frames",
                got.n_total
            ),
        }

        // A round longer than a signed 32-bit count is rejected, and the
        // following round still runs.
        let wide = (i32::MAX as usize) + 1;
        let mut big = frames[0].clone();
        big.header
            .metadata
            .insert("pad".into(), serde_json::Value::String("x".repeat(wide)));
        let mine = if comm.rank() == 0 {
            vec![big]
        } else {
            Vec::new()
        };
        unsafe {
            std::env::set_var("READCON_MPI_TEST_CHUNK", wide.to_string());
        }
        let err = crate::distrib_write::write_block(
            &comm,
            &dir.join("wide.con"),
            &mine,
            crate::distrib_write::WriteMode::MpiIo,
            None,
        )
        .expect_err("a round past i32::MAX must fail");
        assert!(
            err.to_string().contains("write_at_all"),
            "{err}"
        );

        let missing = std::ffi::CString::new(dir.join("absent.con").to_str().unwrap()).unwrap();
        let mut count = 0usize;
        let ptr = unsafe {
            rkr_mpi_read_block_f(
                fint,
                missing.as_ptr(),
                1,
                0,
                &mut count,
                &mut first,
                &mut total_frames,
            )
        };
        assert!(ptr.is_null(), "a missing file must fail the C reader");
        let ptr = unsafe {
            rkr_mpi_read_block_f(
                fint,
                ok_c.as_ptr(),
                1,
                0,
                if comm.rank() == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut count
                },
                &mut first,
                &mut total_frames,
            )
        };
        assert!(ptr.is_null(), "a null frame count must fail the C reader");
        let ptr = unsafe {
            rkr_mpi_read_block_f(
                fint,
                ok_c.as_ptr(),
                1,
                0,
                &mut count,
                &mut first,
                &mut total_frames,
            )
        };
        assert!(!ptr.is_null(), "the C reader returns the frames it stored");
        assert_eq!(total_frames, 2);
        unsafe { crate::ffi::free_rkr_frame_array(ptr, count) };

        flush_coverage_profile();
        drop(comm);
        drop(universe);
    }
}
