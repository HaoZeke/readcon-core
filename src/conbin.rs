//! CON-bin: one page of header, then every frame's positions as one run
//! of little-endian values, then the velocities the same way.
//!
//! [`map_file`] maps that file and faults it in place. The runs are the
//! columns: the caller does not allocate a second buffer, so the read does
//! not take a write fault per destination page. [`read_into_threads`] is the
//! `pread` fill of a caller-owned slice, kept so a measurement can pair the
//! two on the same file.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::Path;

use crate::error::ParseError;
use crate::iterators::ConFrameIterator;

const MAGIC: &[u8; 8] = b"CONBIN01";
const VERSION: u32 = 1;
const FLAG_F32: u32 = 1;
const FLAG_VELOCITY: u32 = 2;

/// Magic, version, flags, frame count, atom count, payload offset.
/// 8 + 4 + 4 + 8 + 8 + 8.
const FIELD_BYTES: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConBinHeader {
    pub n_frames: u64,
    pub n_atoms: u64,
    pub binary32: bool,
    pub velocities: bool,
}

impl ConBinHeader {
    pub fn position_len(&self) -> Result<usize, ParseError> {
        (self.n_frames as usize)
            .checked_mul(self.n_atoms as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| ParseError::InvalidCount("CON-bin column length overflows".into()))
    }
}

/// How the mapped pages are made resident before the caller reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefetch {
    /// One `MADV_POPULATE_READ` per worker, covering that worker's span.
    Span,
    /// `MADV_POPULATE_READ` in queue-sized pieces of each worker's span.
    ChunkPopulate,
    /// `posix_fadvise(POSIX_FADV_WILLNEED)` of the next queue-sized piece
    /// on a private file descriptor, issued before the caller reads.
    ChunkWillneed,
    /// Will-need the next piece, then `MADV_POPULATE_READ` the current one.
    Hybrid,
}

/// Limits read from the block queue that holds `path`.
///
/// `max_request` is `max_sectors_kb` times 1024, the largest request that
/// queue accepts
/// (<https://docs.kernel.org/admin-guide/abi-stable.html#abi-sys-block-disk-queue-max-sectors-kb>).
/// `nr_requests` is how many requests the queue can hold
/// (<https://docs.kernel.org/admin-guide/abi-stable.html#abi-sys-block-disk-queue-nr-requests>).
/// `read_ahead` is `read_ahead_kb`
/// times 1024, the kernel readahead window. A partition is walked to its
/// parent disk, which is the sysfs node that holds `queue/`. A missing
/// queue file yields the page size, the unit `madvise` accepts, and a
/// missing `nr_requests` yields 1. `rotational` is recorded and does not
/// select the plan.
#[derive(Debug, Clone, Copy)]
pub struct QueueLimits {
    pub page: usize,
    pub logical_block: usize,
    pub max_request: usize,
    pub nr_requests: usize,
    pub read_ahead: usize,
    /// Hardware queues. blk-mq applies `nr_requests` to each one.
    pub nr_hw_queues: usize,
    /// `queue/rotational` is 1 when the queue file says so.
    /// The flag is recorded. It does not select the read plan.
    pub rotational: bool,
}

impl QueueLimits {
    /// Page-sized stand-in used when the file has no block queue.
    ///
    /// btrfs, tmpfs, overlayfs and NFS have no `queue/` directory.
    /// The page is the unit `madvise` accepts, so one request of one
    /// page is the stand-in.
    pub fn page_default(page: usize) -> Self {
        Self {
            page,
            logical_block: page,
            max_request: page,
            nr_requests: 1,
            read_ahead: page,
            nr_hw_queues: 1,
            rotational: false,
        }
    }

    pub fn of(path: &Path) -> Result<Self, ParseError> {
        let page = page_size()?;
        #[cfg(target_os = "linux")]
        {
            let file =
                File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
            let dev = std::os::unix::fs::MetadataExt::dev(
                &file
                    .metadata()
                    .map_err(|err| ParseError::ValidationError(err.to_string()))?,
            );
            Ok(Self::from_sysfs(dev, page))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = path;
            Ok(Self::page_default(page))
        }
    }

    #[cfg(target_os = "linux")]
    fn from_sysfs(dev: u64, page: usize) -> Self {
        let num =
            |name: &str| queue_text(dev, name).and_then(|text| text.trim().parse::<usize>().ok());
        let logical = num("logical_block_size").unwrap_or(page).max(1);
        let max_request = num("max_sectors_kb")
            .map(|kb| kb.saturating_mul(1024))
            .unwrap_or(page)
            .max(page);
        let nr_requests = num("nr_requests").unwrap_or(1).max(1);
        let read_ahead = num("read_ahead_kb")
            .map(|kb| kb.saturating_mul(1024))
            .unwrap_or(page)
            .max(page);
        let rotational = num("rotational").unwrap_or(0) != 0;
        Self {
            page,
            logical_block: logical,
            max_request,
            nr_requests,
            read_ahead,
            nr_hw_queues: hw_queue_count(dev),
            rotational,
        }
    }

    /// Bytes one worker submits while the others share the same queue.
    ///
    /// The queue holds `nr_requests` requests of `max_request` bytes.
    /// `workers` share it, so each gets `nr_requests / workers` requests,
    /// or one request when the queue is shorter than the worker count.
    /// The span is at least the kernel readahead window, because a shorter
    /// hint still pulls that window, and it is aligned down to the page.
    pub fn worker_span(&self, workers: usize) -> usize {
        let workers = workers.max(1);
        let requests = (self.nr_requests / workers).max(1);
        let queued = requests.saturating_mul(self.max_request);
        let span = queued.max(self.read_ahead).max(self.page);
        let aligned = span & !(self.page - 1);
        aligned.max(self.page)
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn dev_nums(dev: u64) -> (u64, u64) {
    // glibc `sys/sysmacros.h`, the encoding major(3) documents.
    let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0xfff);
    let minor = ((dev >> 12) & 0xffff_ff00) | (dev & 0xff);
    (major, minor)
}

/// `queue/nr_hw_queues` when the file exists. Otherwise the number of
/// directories under `mq/`, which blk-mq creates, one per hardware queue.
/// A missing tree means one queue.
#[cfg(target_os = "linux")]
fn hw_queue_count(dev: u64) -> usize {
    if let Some(text) = queue_text(dev, "nr_hw_queues") {
        if let Ok(count) = text.trim().parse::<usize>() {
            return count.max(1);
        }
    }
    let (major, minor) = dev_nums(dev);
    let mut dir = std::path::PathBuf::from(format!("/sys/dev/block/{major}:{minor}"));
    if dir.join("partition").is_file() {
        dir.push("..");
    }
    let mq = dir.join("mq");
    let count = std::fs::read_dir(&mq)
        .ok()
        .map(|entries| entries.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0);
    count.max(1)
}

/// Read one queue file. A partition node has `partition` and no `queue`.
/// Its parent is the disk, and that directory holds `queue/`.
/// `Documentation/admin-guide/devices.rst` describes the block sysfs tree.
#[cfg(target_os = "linux")]
fn queue_text(dev: u64, name: &str) -> Option<String> {
    let (major, minor) = dev_nums(dev);
    let dir = std::path::PathBuf::from(format!("/sys/dev/block/{major}:{minor}"));
    let own = dir.join("queue").join(name);
    if let Ok(text) = std::fs::read_to_string(&own) {
        return Some(text);
    }
    if dir.join("partition").is_file() {
        let parent = dir.join("..").join("queue").join(name);
        if let Ok(text) = std::fs::read_to_string(&parent) {
            return Some(text);
        }
    }
    None
}

/// Host page size. `madvise` rejects an address that is not a multiple of it.
fn page_size() -> Result<usize, ParseError> {
    #[cfg(unix)]
    let page = {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page <= 0 {
            return Err(ParseError::ValidationError(
                "sysconf(SC_PAGESIZE) failed".into(),
            ));
        }
        page as usize
    };
    #[cfg(windows)]
    let page = {
        // https://learn.microsoft.com/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut info = std::mem::MaybeUninit::<SYSTEM_INFO>::uninit();
        unsafe { GetSystemInfo(info.as_mut_ptr()) };
        unsafe { info.assume_init() }.dwPageSize as usize
    };
    // Zero is not a power of two, so the page-size check covers it.
    if !page.is_power_of_two() {
        return Err(ParseError::ValidationError(
            "the host page size is not a power of two".into(),
        ));
    }
    Ok(page)
}

fn header_fields(header: &ConBinHeader, payload_off: u64) -> [u8; FIELD_BYTES] {
    let mut out = [0u8; FIELD_BYTES];
    out[..8].copy_from_slice(MAGIC);
    out[8..12].copy_from_slice(&VERSION.to_le_bytes());
    let mut flags = 0u32;
    if header.binary32 {
        flags |= FLAG_F32;
    }
    if header.velocities {
        flags |= FLAG_VELOCITY;
    }
    out[12..16].copy_from_slice(&flags.to_le_bytes());
    out[16..24].copy_from_slice(&header.n_frames.to_le_bytes());
    out[24..32].copy_from_slice(&header.n_atoms.to_le_bytes());
    out[32..40].copy_from_slice(&payload_off.to_le_bytes());
    out
}

fn parse_fields(bytes: &[u8]) -> Result<(ConBinHeader, u64), ParseError> {
    if bytes.len() < FIELD_BYTES || &bytes[..8] != MAGIC {
        return Err(ParseError::ValidationError(
            "CON-bin header is not CONBIN01".into(),
        ));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(ParseError::ValidationError(format!(
            "CON-bin version {version} is not {VERSION}"
        )));
    }
    let flags = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let offset = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    Ok((
        ConBinHeader {
            n_frames: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            n_atoms: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            binary32: flags & FLAG_F32 != 0,
            velocities: flags & FLAG_VELOCITY != 0,
        },
        offset,
    ))
}

/// Payload start stored in the header. Any 8-byte alignment inside the
/// file is accepted, so a file written on a 4 KiB host still opens when
/// the host page is 16 KiB or 64 KiB. An offset of 0 is not a stand-in
/// for the host page.
fn checked_payload_offset(stored: u64, file_len: usize) -> Result<usize, ParseError> {
    // Every `u64` fits in `usize` on a 64-bit host. A narrower target
    // still rejects an offset that does not fit the address size.
    #[cfg(target_pointer_width = "64")]
    let off = stored as usize;
    #[cfg(not(target_pointer_width = "64"))]
    let off = usize::try_from(stored).map_err(|_| {
        ParseError::ValidationError("CON-bin payload offset does not fit".into())
    })?;
    if off < FIELD_BYTES || off > file_len || off % 8 != 0 {
        return Err(ParseError::ValidationError(
            "CON-bin payload offset is not an 8-byte aligned address inside the file".into(),
        ));
    }
    Ok(off)
}

#[derive(Clone, Copy)]
struct F64Payload {
    offset: usize,
    end: usize,
}

/// Binary64 columns with a velocity run. Binary32 and a missing velocity
/// run are errors: `sum_mapped` reads those two binary64 runs.
fn f64_payload(header_bytes: &[u8], file_len: usize) -> Result<F64Payload, ParseError> {
    let (header, stored) = parse_fields(header_bytes)?;
    if header.binary32 {
        return Err(ParseError::ValidationError(
            "CON-bin file is binary32; sum_mapped reads binary64".into(),
        ));
    }
    if !header.velocities {
        return Err(ParseError::ValidationError(
            "CON-bin file has no velocity run".into(),
        ));
    }
    let offset = checked_payload_offset(stored, file_len)?;
    let values = header
        .position_len()?
        .checked_mul(2)
        .ok_or_else(|| ParseError::InvalidCount("CON-bin column length overflows".into()))?;
    let nbytes = values
        .checked_mul(8)
        .ok_or_else(|| ParseError::InvalidCount("CON-bin column byte length overflows".into()))?;
    let end = offset
        .checked_add(nbytes)
        .ok_or_else(|| ParseError::InvalidCount("CON-bin payload offset overflows".into()))?;
    if end > file_len {
        return Err(ParseError::IncompleteFrame);
    }
    Ok(F64Payload { offset, end })
}

/// A mapped CON-bin. The position run and the velocity run are the file.
pub struct ConBinMap {
    /// `MAP_PRIVATE` copy-on-write (`MmapOptions::map_copy`). A store copies
    /// the page and does not write the file.
    pub map: memmap2::MmapMut,
    pub header: ConBinHeader,
    pub payload_off: usize,
}

impl ConBinMap {
    pub fn positions_f64(&self) -> Result<&[f64], ParseError> {
        self.run(std::mem::size_of::<f64>(), 0)
    }

    pub fn positions_f64_mut(&mut self) -> Result<&mut [f64], ParseError> {
        self.run_mut(std::mem::size_of::<f64>(), 0)
    }

    pub fn velocities_f64(&self) -> Result<&[f64], ParseError> {
        let n = self.header.position_len()?;
        self.run(std::mem::size_of::<f64>(), n)
    }

    pub fn positions_f32(&self) -> Result<&[f32], ParseError> {
        self.run(std::mem::size_of::<f32>(), 0)
    }

    pub fn velocities_f32(&self) -> Result<&[f32], ParseError> {
        let n = self.header.position_len()?;
        self.run(std::mem::size_of::<f32>(), n)
    }

    fn run<T>(&self, width: usize, column: usize) -> Result<&[T], ParseError> {
        let n = self.header.position_len()?;
        let start = self
            .payload_off
            .checked_add(column.checked_mul(width).ok_or_else(|| {
                ParseError::InvalidCount("CON-bin column byte length overflows".into())
            })?)
            .ok_or_else(|| ParseError::InvalidCount("CON-bin payload offset overflows".into()))?;
        let nbytes = n.checked_mul(width).ok_or_else(|| {
            ParseError::InvalidCount("CON-bin column byte length overflows".into())
        })?;
        let end = start.checked_add(nbytes).ok_or_else(|| {
            ParseError::InvalidCount("CON-bin column extends past the file".into())
        })?;
        if end > self.map.len() {
            return Err(ParseError::IncompleteFrame);
        }
        let bytes = &self.map[start..end];
        #[cfg(not(target_endian = "little"))]
        {
            let _ = bytes;
            return Err(ParseError::ValidationError(
                "CON-bin is little-endian; this host cannot view it in place".into(),
            ));
        }
        if bytes.as_ptr().align_offset(std::mem::align_of::<T>()) != 0 {
            return Err(ParseError::ValidationError(
                "CON-bin payload is not aligned for its element type".into(),
            ));
        }
        // SAFETY: `bytes` is inside the map, its length is a multiple of `T`,
        // and the pointer is aligned for `T`. The file is little-endian and
        // the host is little-endian, so the bit pattern is the value.
        // The map stays borrowed for the returned slice.
        Ok(unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<T>(), n) })
    }

    fn run_mut<T>(&mut self, width: usize, column: usize) -> Result<&mut [T], ParseError> {
        let n = self.header.position_len()?;
        let start = self
            .payload_off
            .checked_add(column.checked_mul(width).ok_or_else(|| {
                ParseError::InvalidCount("CON-bin column byte length overflows".into())
            })?)
            .ok_or_else(|| ParseError::InvalidCount("CON-bin payload offset overflows".into()))?;
        let nbytes = n.checked_mul(width).ok_or_else(|| {
            ParseError::InvalidCount("CON-bin column byte length overflows".into())
        })?;
        let end = start.checked_add(nbytes).ok_or_else(|| {
            ParseError::InvalidCount("CON-bin column extends past the file".into())
        })?;
        if end > self.map.len() {
            return Err(ParseError::IncompleteFrame);
        }
        #[cfg(not(target_endian = "little"))]
        {
            return Err(ParseError::ValidationError(
                "CON-bin is little-endian; this host cannot view it in place".into(),
            ));
        }
        let ptr = self.map[start..end].as_mut_ptr();
        if ptr.align_offset(std::mem::align_of::<T>()) != 0 {
            return Err(ParseError::ValidationError(
                "CON-bin payload is not aligned for its element type".into(),
            ));
        }
        // SAFETY: same alignment and length rules as `run`. The map is
        // `MAP_PRIVATE`, so stores copy a page and do not write the file.
        Ok(unsafe { std::slice::from_raw_parts_mut(ptr.cast::<T>(), n) })
    }
}

/// Sum both binary64 columns of `path`.
///
/// The default is one `readahead` stream.
/// Two sessions on one boot compared one stream with a stream per span.
/// Neither plan won at every thread count in both.
/// The host cache was not controlled.
/// The default stays one stream, the simpler plan.
/// `nr_requests` is the depth of each hardware queue (`nr_hw_queues`
/// of them, blk-mq), not one device-wide pool.
/// Workers sum a piece after that piece is faulted in, and the piece
/// sums are added in file order. The header's payload offset, length
/// and dtype select the bytes. Binary32 and a missing velocity run are
/// errors. Other platforms map the file and sum it with no `readahead`.
pub fn sum_mapped(path: &Path, threads: usize) -> Result<f64, ParseError> {
    #[cfg(target_os = "linux")]
    {
        let limits = QueueLimits::of(path)?;
        sequential_resident_sum(path, threads.max(1), &limits)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = threads;
        plain_sum(path)
    }
}

#[cfg(not(target_os = "linux"))]
fn plain_sum(path: &Path) -> Result<f64, ParseError> {
    let file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let map = unsafe { memmap2::Mmap::map(&file) }
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    if map.len() < FIELD_BYTES {
        return Err(ParseError::ValidationError(
            "CON-bin header is not CONBIN01".into(),
        ));
    }
    let payload = f64_payload(&map[..FIELD_BYTES], map.len())?;
    let bytes = &map[payload.offset..payload.end];
    #[cfg(not(target_endian = "little"))]
    {
        let _ = bytes;
        return Err(ParseError::ValidationError(
            "CON-bin is little-endian; this host cannot view it in place".into(),
        ));
    }
    let ptr = bytes.as_ptr();
    if ptr.align_offset(std::mem::align_of::<f64>()) != 0 {
        return Err(ParseError::ValidationError(
            "CON-bin payload is not aligned for its element type".into(),
        ));
    }
    let data = unsafe { std::slice::from_raw_parts(ptr.cast::<f64>(), bytes.len() / 8) };
    Ok(data.iter().copied().sum())
}

#[cfg(target_os = "linux")]
fn sequential_resident_sum(
    path: &Path,
    threads: usize,
    limits: &QueueLimits,
) -> Result<f64, ParseError> {
    let owned = open_map(path)?;
    let page = limits.page;
    let len = owned.len;
    let base = owned.ptr as usize;
    let header =
        unsafe { std::slice::from_raw_parts(owned.ptr.cast::<u8>(), FIELD_BYTES.min(len)) };
    let payload = f64_payload(header, len)?;
    let request = (limits.max_request & !(page - 1)).max(page);
    let pieces = piece_list(0, len, request);
    if pages_all_resident(owned.ptr, len, page)? {
        return Ok(sum_pieces_in_order(
            base,
            payload.offset,
            payload.end,
            threads,
            &pieces,
        ));
    }
    // One stream fills the read side of the queue.
    let depth = limits.nr_requests.max(1);
    let (tx, rx) = std::sync::mpsc::sync_channel::<usize>(depth);
    let (err_tx, err_rx) = std::sync::mpsc::channel::<String>();
    let rx = std::sync::Arc::new(std::sync::Mutex::new(rx));
    let addr = owned.ptr as usize;
    let found = std::thread::scope(|scope| {
        let queued = pieces.clone();
        let err_tx_worker = err_tx.clone();
        scope.spawn(move || {
            let fd = match File::open(path) {
                Ok(fd) => fd,
                Err(err) => {
                    let _ = err_tx_worker.send(err.to_string());
                    return;
                }
            };
            // Keep `depth` requests in flight. Submit the next piece only
            // after the oldest one has been waited on, so the count does not
            // become depth + 1.
            let mut submitted = 0usize;
            while submitted < depth.min(queued.len()) {
                if let Err(err) = advise_readahead(&fd, queued[submitted].0, queued[submitted].1) {
                    let _ = err_tx_worker.send(err);
                    return;
                }
                submitted += 1;
            }
            for i in 0..queued.len() {
                let (off, n) = queued[i];
                if let Err(err) = wait_resident(addr, off, n, page) {
                    let _ = err_tx_worker.send(err);
                    return;
                }
                let _ = tx.send(i);
                if submitted < queued.len() {
                    if let Err(err) =
                        advise_readahead(&fd, queued[submitted].0, queued[submitted].1)
                    {
                        let _ = err_tx_worker.send(err);
                        return;
                    }
                    submitted += 1;
                }
            }
        });
        let slots = std::sync::Arc::new(std::sync::Mutex::new(vec![0.0f64; pieces.len()]));
        let mut handles = Vec::with_capacity(threads);
        for _ in 0..threads {
            let rx = std::sync::Arc::clone(&rx);
            let slots = std::sync::Arc::clone(&slots);
            let pieces = pieces.clone();
            handles.push(scope.spawn(move || {
                loop {
                    let msg = rx.lock().expect("piece queue").recv();
                    let Ok(index) = msg else { break };
                    let (off, n) = pieces[index];
                    let part = sum_range(base, off, off + n, payload.offset, payload.end);
                    slots.lock().expect("piece sums")[index] = part;
                }
            }));
        }
        for handle in handles {
            if handle.join().is_err() {
                let _ = err_tx.send("column sum worker panicked".to_string());
            }
        }
        let slots = slots.lock().expect("piece sums");
        slots.iter().copied().sum::<f64>()
    });
    if let Ok(err) = err_rx.try_recv() {
        return Err(ParseError::ValidationError(err));
    }
    Ok(found)
}

/// One `readahead` stream per worker, each over a disjoint span.
///
/// The piece is `max_sectors_kb` times 1024. `nr_requests` is the depth
/// of each hardware queue, so each worker keeps
/// `max(1, nr_requests / workers)` pieces in flight.
/// Two sessions on one boot compared one stream with a stream per span.
/// Neither plan won at every thread count in both.
/// The host cache was not controlled.
/// The default stays one stream, the simpler plan.
#[cfg(target_os = "linux")]
pub fn sum_split_streams(path: &Path, threads: usize) -> Result<f64, ParseError> {
    let limits = QueueLimits::of(path)?;
    split_stream_sum(path, threads.max(1), &limits)
}

#[cfg(not(target_os = "linux"))]
pub fn sum_split_streams(path: &Path, threads: usize) -> Result<f64, ParseError> {
    let _ = threads;
    plain_sum(path)
}

#[cfg(target_os = "linux")]
fn split_stream_sum(path: &Path, threads: usize, limits: &QueueLimits) -> Result<f64, ParseError> {
    let owned = open_map(path)?;
    let page = limits.page;
    let len = owned.len;
    let base = owned.ptr as usize;
    let header =
        unsafe { std::slice::from_raw_parts(owned.ptr.cast::<u8>(), FIELD_BYTES.min(len)) };
    let payload = f64_payload(header, len)?;
    let request = (limits.max_request & !(page - 1)).max(page);
    let ranges = worker_ranges(len, threads, page);
    if pages_all_resident(owned.ptr, len, page)? {
        let pieces = piece_list(0, len, request);
        return Ok(sum_pieces_in_order(
            base,
            payload.offset,
            payload.end,
            threads,
            &pieces,
        ));
    }
    let depth = (limits.nr_requests / ranges.len()).max(1);
    let (err_tx, err_rx) = std::sync::mpsc::channel::<String>();
    let sum = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(ranges.len());
        for (span, end) in ranges {
            let err_tx = err_tx.clone();
            handles.push(scope.spawn(move || {
                match stream_one_span(
                    path,
                    SpanGeom {
                        base,
                        span,
                        end,
                        page,
                        payload_off: payload.offset,
                        payload_end: payload.end,
                        request,
                        depth,
                    },
                ) {
                    Ok(part) => part,
                    Err(err) => {
                        let _ = err_tx.send(err);
                        0.0
                    }
                }
            }));
        }
        let parts: Vec<f64> = handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok(part) => part,
                Err(_) => {
                    let _ = err_tx.send("split stream worker panicked".to_string());
                    0.0
                }
            })
            .collect();
        drop(err_tx);
        parts.into_iter().sum::<f64>()
    });
    if let Ok(err) = err_rx.try_recv() {
        return Err(ParseError::ValidationError(err));
    }
    Ok(sum)
}

#[cfg(target_os = "linux")]
struct SpanGeom {
    base: usize,
    span: usize,
    end: usize,
    page: usize,
    payload_off: usize,
    payload_end: usize,
    request: usize,
    depth: usize,
}

#[cfg(target_os = "linux")]
fn stream_one_span(path: &Path, geom: SpanGeom) -> Result<f64, String> {
    let SpanGeom {
        base,
        span,
        end,
        page,
        payload_off,
        payload_end,
        request,
        depth,
    } = geom;
    let fd = File::open(path).map_err(|err| err.to_string())?;
    let pieces = piece_list(span, end, request);
    let depth = depth.max(1);
    let mut next_submit = 0usize;
    let mut next_sum = 0usize;
    let mut acc = 0.0f64;
    while next_sum < pieces.len() {
        while next_submit < pieces.len() && next_submit - next_sum < depth {
            let (off, n) = pieces[next_submit];
            advise_readahead(&fd, off, n)?;
            next_submit += 1;
        }
        let (off, n) = pieces[next_sum];
        wait_resident(base, off, n, page)?;
        acc += sum_range(base, off, off + n, payload_off, payload_end);
        next_sum += 1;
    }
    Ok(acc)
}

#[cfg(target_os = "linux")]
fn sum_pieces_in_order(
    base: usize,
    payload_off: usize,
    payload_end: usize,
    threads: usize,
    pieces: &[(usize, usize)],
) -> f64 {
    let mut slots = vec![0.0f64; pieces.len()];
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(threads);
        for _ in 0..threads {
            handles.push(scope.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if index >= pieces.len() {
                        break;
                    }
                    let (off, n) = pieces[index];
                    local.push((
                        index,
                        sum_range(base, off, off + n, payload_off, payload_end),
                    ));
                }
                local
            }));
        }
        for handle in handles {
            for (index, part) in handle.join().expect("column sum") {
                slots[index] = part;
            }
        }
    });
    slots.iter().copied().sum()
}

#[cfg(target_os = "linux")]
fn pages_all_resident(ptr: *mut libc::c_void, len: usize, page: usize) -> Result<bool, ParseError> {
    if len == 0 {
        return Ok(true);
    }
    let pages = len.div_ceil(page);
    let mut vec = vec![0u8; pages];
    let rc = unsafe { libc::mincore(ptr, len, vec.as_mut_ptr()) };
    if rc != 0 {
        return Err(ParseError::ValidationError(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    Ok(vec.iter().all(|byte| byte & 1 == 1))
}

/// `Ok` means the range is resident. `Err` is the errno from `madvise`.
/// Only `EINVAL` means this kernel has no `MADV_POPULATE_READ`, so the
/// caller may fault the pages itself. `EFAULT`, `EIO`, `EHWPOISON` and
/// `ENOMEM` are failures (man 2 madvise).
#[cfg(target_os = "linux")]
fn populate_followup(code: i32) -> Result<(), i32> {
    if code == libc::EINVAL {
        Ok(())
    } else {
        Err(code)
    }
}

#[cfg(target_os = "linux")]
fn touch_pages(addr: usize, off: usize, len: usize, page: usize) {
    // `index < div_ceil(len, page)` and `page > 0` imply `index * page < len`,
    // so a guard on `at >= len` cannot run. `page == 0` panics in `div_ceil`.
    let pages = len.div_ceil(page);
    for index in 0..pages {
        let at = index * page;
        let byte = (addr + off + at) as *const u8;
        std::hint::black_box(unsafe { std::ptr::read_volatile(byte) });
    }
}

#[cfg(target_os = "linux")]
fn wait_resident(addr: usize, off: usize, len: usize, page: usize) -> Result<(), String> {
    // One readahead has already been submitted. MADV_POPULATE_READ (Linux
    // 5.14) faults this range and waits. The address is checked before the
    // call, so EINVAL is the old-kernel miss and one byte of each page is
    // read. EAGAIN is a transient kernel resource (man 2 madvise) and is
    // retried once per page of the range. Any other errno is returned.
    if page == 0 || (addr + off) % page != 0 {
        return Err("madvise address is not page-aligned".into());
    }
    let start = (addr + off) as *mut libc::c_void;
    let pages = len.div_ceil(page).max(1);
    let mut tried = 0usize;
    loop {
        let rc = unsafe { libc::madvise(start, len, libc::MADV_POPULATE_READ) };
        if rc == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        let code = err.raw_os_error().unwrap_or(0);
        if code == libc::EAGAIN && tried < pages {
            tried += 1;
            continue;
        }
        if populate_followup(code).is_ok() {
            touch_pages(addr, off, len, page);
            return Ok(());
        }
        return Err(err.to_string());
    }
}

#[cfg(target_os = "linux")]
fn advise_readahead(file: &File, off: usize, len: usize) -> Result<(), String> {
    let rc = unsafe { libc::readahead(file.as_raw_fd(), off as i64, len) };
    if rc != 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
struct OwnedMap {
    ptr: *mut libc::c_void,
    len: usize,
}

#[cfg(target_os = "linux")]
impl Drop for OwnedMap {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr, self.len) };
    }
}

#[cfg(target_os = "linux")]
fn open_map(path: &Path) -> Result<OwnedMap, ParseError> {
    let file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let len = file
        .metadata()
        .map_err(|err| ParseError::ValidationError(err.to_string()))?
        .len() as usize;
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            file.as_raw_fd(),
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return Err(ParseError::ValidationError(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    unsafe {
        libc::madvise(ptr, len, libc::MADV_SEQUENTIAL);
        libc::madvise(ptr, len, libc::MADV_HUGEPAGE);
    }
    Ok(OwnedMap { ptr, len })
}

#[cfg(target_os = "linux")]
fn sum_range(base: usize, start: usize, end: usize, payload_off: usize, payload_end: usize) -> f64 {
    let from = start.max(payload_off);
    let to = end.min(payload_end);
    if to <= from {
        return 0.0;
    }
    let data = unsafe { std::slice::from_raw_parts((base + from) as *const f64, (to - from) / 8) };
    let mut acc = 0.0f64;
    for &value in data {
        acc += value;
    }
    acc
}

/// Map `path` and fault the pages in before returning.
///
/// The map is `MAP_PRIVATE` and writable. A store to a page copies that
/// page. It does not modify the file. `mmap(2)` leaves it unspecified
/// whether a later write to the file by someone else appears in a page
/// the caller has not stored to. Truncation can raise `SIGBUS`.
///
/// `MADV_SEQUENTIAL` and `MADV_WILLNEED` describe one forward pass.
/// `MADV_HUGEPAGE` is the request used when transparent huge pages are in
/// `madvise` mode (`CONFIG_TRANSPARENT_HUGEPAGE_MADVISE`). File-backed
/// huge pages also need `CONFIG_READ_ONLY_THP_FOR_FS`
/// (Documentation/admin-guide/mm/transhuge.rst). This kernel's
/// `/proc/config.gz` has that option unset, and a mapped read showed
/// `FilePmdMapped` 0. The advice stays, because it is the switch the
/// kernel documents, and it is a no-op without that option.
/// `MADV_POPULATE_READ` (Linux 5.14) waits until the pages are resident
/// and reports an error. `MAP_POPULATE` hides errors and cannot be split
/// across the caller's threads after `mmap` returns.
/// `threads` workers fault page-aligned spans. A file shorter than one
/// page per worker stays on one worker. The page is `sysconf(SC_PAGESIZE)`.
pub fn map_file(path: &Path, threads: usize) -> Result<ConBinMap, ParseError> {
    map_file_with(path, threads, Prefetch::Span)
}

/// [`map_file`] with an explicit prefetch. The benchmark pairs these.
pub fn map_file_with(
    path: &Path,
    threads: usize,
    prefetch: Prefetch,
) -> Result<ConBinMap, ParseError> {
    let file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    // MAP_PRIVATE, PROT_READ|PROT_WRITE. Writes copy the page.
    let map = unsafe { memmap2::MmapOptions::new().map_copy(&file) }
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    if map.len() < FIELD_BYTES {
        return Err(ParseError::ValidationError(
            "CON-bin header is not CONBIN01".into(),
        ));
    }
    let (header, offset) = parse_fields(&map[..FIELD_BYTES])?;
    let page = page_size()?;
    let payload_off = checked_payload_offset(offset, map.len())?;
    fault_in(path, &map, threads, page, prefetch)?;
    Ok(ConBinMap {
        map,
        header,
        payload_off,
    })
}

fn fault_in(
    path: &Path,
    map: &memmap2::MmapMut,
    threads: usize,
    page: usize,
    prefetch: Prefetch,
) -> Result<(), ParseError> {
    #[cfg(unix)]
    let _ = map.advise(memmap2::Advice::Sequential);
    #[cfg(target_os = "linux")]
    let _ = map.advise(memmap2::Advice::HugePage);
    #[cfg(target_os = "linux")]
    {
        match prefetch {
            Prefetch::Span => {
                let _ = map.advise(memmap2::Advice::WillNeed);
                advise_spans(map, memmap2::Advice::PopulateRead, threads, page)?;
            }
            Prefetch::ChunkPopulate => {
                let window = QueueLimits::of(path)?.worker_span(threads);
                chunk_each(map, threads, page, window, ChunkOp::Populate)?;
            }
            Prefetch::ChunkWillneed => {
                let window = QueueLimits::of(path)?.worker_span(threads);
                chunk_each(map, threads, page, window, ChunkOp::Willneed { path })?;
            }
            Prefetch::Hybrid => {
                let window = QueueLimits::of(path)?.worker_span(threads);
                chunk_each(map, threads, page, window, ChunkOp::Hybrid { path })?;
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (path, threads, prefetch);
        for chunk in map.chunks(page) {
            std::hint::black_box(chunk.first().copied());
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
enum ChunkOp<'a> {
    Populate,
    Willneed { path: &'a Path },
    Hybrid { path: &'a Path },
}

#[cfg(target_os = "linux")]
fn chunk_each(
    map: &memmap2::MmapMut,
    threads: usize,
    page: usize,
    window: usize,
    op: ChunkOp<'_>,
) -> Result<(), ParseError> {
    let ranges = worker_ranges(map.len(), threads, page);
    let shared = match op {
        ChunkOp::Populate => ChunkOp::Populate,
        ChunkOp::Willneed { path } => ChunkOp::Willneed { path },
        ChunkOp::Hybrid { path } => ChunkOp::Hybrid { path },
    };
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(ranges.len());
        for (start, end) in ranges {
            let op = match shared {
                ChunkOp::Populate => ChunkOp::Populate,
                ChunkOp::Willneed { path } => ChunkOp::Willneed { path },
                ChunkOp::Hybrid { path } => ChunkOp::Hybrid { path },
            };
            handles.push(scope.spawn(move || chunk_worker(map, start, end, window, page, op)));
        }
        handles.into_iter().try_for_each(|handle| {
            handle
                .join()
                .expect("CON-bin prefetch thread panicked")
        })
    })
}

#[cfg(target_os = "linux")]
fn chunk_worker(
    map: &memmap2::MmapMut,
    start: usize,
    end: usize,
    window: usize,
    page: usize,
    op: ChunkOp<'_>,
) -> Result<(), ParseError> {
    let window = window.max(page);
    let pieces = piece_list(start, end, window);
    let fd = match op {
        ChunkOp::Willneed { path } | ChunkOp::Hybrid { path } => {
            Some(File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?)
        }
        ChunkOp::Populate => None,
    };
    match op {
        ChunkOp::Populate => {
            for (off, len) in pieces {
                populate_range(map, off, len)?;
            }
        }
        ChunkOp::Willneed { .. } => {
            let fd = fd.as_ref().expect("willneed opens a descriptor");
            for (off, len) in pieces {
                willneed(fd, off, len)?;
            }
        }
        ChunkOp::Hybrid { .. } => {
            let fd = fd.as_ref().expect("hybrid opens a descriptor");
            if let Some((off, len)) = pieces.first().copied() {
                willneed(fd, off, len)?;
            }
            for i in 0..pieces.len() {
                if let Some((off, len)) = pieces.get(i + 1).copied() {
                    willneed(fd, off, len)?;
                }
                let (off, len) = pieces[i];
                populate_range(map, off, len)?;
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn piece_list(start: usize, end: usize, window: usize) -> Vec<(usize, usize)> {
    let mut pieces = Vec::new();
    let mut off = start;
    while off < end {
        let len = window.min(end - off);
        pieces.push((off, len));
        off += len;
    }
    pieces
}

#[cfg(target_os = "linux")]
fn populate_range(map: &memmap2::MmapMut, off: usize, len: usize) -> Result<(), ParseError> {
    let page = page_size()?;
    wait_resident(map.as_ptr() as usize, off, len, page).map_err(ParseError::ValidationError)
}

#[cfg(target_os = "linux")]
fn willneed(file: &File, off: usize, len: usize) -> Result<(), ParseError> {
    let rc = unsafe {
        libc::posix_fadvise(
            file.as_raw_fd(),
            off as libc::off_t,
            len as libc::off_t,
            libc::POSIX_FADV_WILLNEED,
        )
    };
    if rc != 0 {
        return Err(ParseError::ValidationError(format!(
            "posix_fadvise WILLNEED: {rc}"
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn worker_ranges(len: usize, threads: usize, page: usize) -> Vec<(usize, usize)> {
    if len == 0 {
        return Vec::new();
    }
    let threads = threads.max(1);
    let workers = if len < page.saturating_mul(threads) {
        1
    } else {
        threads
    };
    if workers == 1 {
        return vec![(0, len)];
    }
    let chunk = (len / workers) & !(page - 1);
    let mut ranges = Vec::with_capacity(workers);
    let mut start = 0usize;
    for worker in 0..workers {
        let take = if worker + 1 == workers {
            len - start
        } else {
            chunk
        };
        ranges.push((start, start + take));
        start += take;
    }
    ranges
}

#[cfg(target_os = "linux")]
fn advise_spans(
    map: &memmap2::MmapMut,
    advice: memmap2::Advice,
    threads: usize,
    page: usize,
) -> Result<(), ParseError> {
    let len = map.len();
    if len == 0 {
        return Ok(());
    }
    let threads = threads.max(1);
    let workers = if len < page.saturating_mul(threads) {
        1
    } else {
        threads
    };
    if workers == 1 {
        return map
            .advise(advice)
            .map_err(|err| ParseError::ValidationError(err.to_string()));
    }
    let chunk = (len / workers) & !(page - 1);
    std::thread::scope(|scope| {
        let mut start = 0usize;
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let take = if worker + 1 == workers {
                len - start
            } else {
                chunk
            };
            let off = start;
            start += take;
            handles.push(scope.spawn(move || map.advise_range(advice, off, take)));
        }
        handles.into_iter().try_for_each(|handle| {
            handle
                .join()
                .expect("CON-bin populate thread panicked")
                .map_err(|err| ParseError::ValidationError(err.to_string()))
        })
    })
}

pub fn read_header(path: &Path) -> Result<ConBinHeader, ParseError> {
    let mut file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let mut bytes = [0u8; FIELD_BYTES];
    file.read_exact(&mut bytes)
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    parse_fields(&bytes).map(|(header, _)| header)
}

fn bytes_of<T>(values: &mut [T]) -> &mut [u8] {
    let len = std::mem::size_of_val(values);
    // SAFETY: `values` is a live slice. The byte view covers exactly those
    // elements and is not used after `values` is borrowed again.
    unsafe { std::slice::from_raw_parts_mut(values.as_mut_ptr().cast::<u8>(), len) }
}

/// Copy a binary64 CON-bin into caller-owned columns. One thread issues the reads.
pub fn read_into(
    path: &Path,
    positions: &mut [f64],
    velocities: Option<&mut [f64]>,
) -> Result<ConBinHeader, ParseError> {
    read_into_threads(path, positions, velocities, 1)
}

/// [`read_into`] with `threads` workers. Each worker reads a page-aligned span.
/// A span shorter than one page stays on one worker.
pub fn read_into_threads(
    path: &Path,
    positions: &mut [f64],
    velocities: Option<&mut [f64]>,
    threads: usize,
) -> Result<ConBinHeader, ParseError> {
    read_columns_into(path, positions, velocities, threads, false)
}

/// Copy both binary64 columns into owned vectors.
///
/// Each vector is `with_capacity`, then `pread` fills `spare_capacity_mut`
/// and `set_len` publishes those slots. The byte view is not a `&mut [f64]`
/// over uninitialised memory. A missing velocity run is an error.
pub fn read_owned_f64(
    path: &Path,
    threads: usize,
) -> Result<(ConBinHeader, Vec<f64>, Vec<f64>), ParseError> {
    let header = read_header(path)?;
    if header.binary32 {
        return Err(ParseError::ValidationError(
            "CON-bin file is binary32; use read_into_f32".into(),
        ));
    }
    if !header.velocities {
        return Err(ParseError::ValidationError(
            "CON-bin file has no velocity run".into(),
        ));
    }
    let n = header.position_len()?;
    let mut positions = Vec::<f64>::with_capacity(n);
    let mut velocities = Vec::<f64>::with_capacity(n);
    fill_owned_f64(path, &mut positions, &mut velocities, n, threads)?;
    Ok((header, positions, velocities))
}

#[cfg(unix)]
fn spare_prefix_uninit(buf: &mut Vec<f64>, n: usize) -> &mut [std::mem::MaybeUninit<u8>] {
    let spare = buf.spare_capacity_mut();
    assert!(spare.len() >= n);
    let nbytes = n * std::mem::size_of::<f64>();
    let ptr = spare.as_mut_ptr().cast::<std::mem::MaybeUninit<u8>>();
    // SAFETY: `n` uninitialised f64 slots cover `nbytes` bytes. A
    // `MaybeUninit<u8>` slot may be uninitialised, so this slice does not
    // claim the bytes are initialised. `pread` writes through the raw
    // pointer, then `set_len` publishes the f64 slots.
    unsafe { std::slice::from_raw_parts_mut(ptr, nbytes) }
}

fn fill_owned_f64(
    path: &Path,
    positions: &mut Vec<f64>,
    velocities: &mut Vec<f64>,
    n: usize,
    threads: usize,
) -> Result<(), ParseError> {
    #[cfg(not(unix))]
    {
        let _ = (path, positions, velocities, n, threads);
        return Err(ParseError::ValidationError(
            "CON-bin pread fill requires Unix".into(),
        ));
    }
    #[cfg(unix)]
    {
        let file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
        let mut raw = [0u8; FIELD_BYTES];
        (&file)
            .read_exact(&mut raw)
            .map_err(|err| ParseError::ValidationError(err.to_string()))?;
        let (_header, offset) = parse_fields(&raw)?;
        let width = std::mem::size_of::<f64>();
        let expect = n.checked_mul(width).ok_or_else(|| {
            ParseError::InvalidCount("CON-bin column byte length overflows".into())
        })?;
        let page = page_size()?;
        let file_len = file
            .metadata()
            .map_err(|err| ParseError::ValidationError(err.to_string()))?
            .len() as usize;
        let pos_off = checked_payload_offset(offset, file_len)? as u64;
        let fd = file.as_raw_fd();
        {
            let bytes = spare_prefix_uninit(positions, n);
            pread_parallel_uninit(fd, bytes, pos_off, threads, page)?;
        }
        // SAFETY: `pread_parallel_uninit` wrote `n` little-endian f64 values or returned.
        unsafe { positions.set_len(n) };
        {
            let bytes = spare_prefix_uninit(velocities, n);
            pread_parallel_uninit(fd, bytes, pos_off + expect as u64, threads, page)?;
        }
        unsafe { velocities.set_len(n) };
        Ok(())
    }
}

/// Copy a binary32 CON-bin into caller-owned columns. One thread issues the reads.
pub fn read_into_f32(
    path: &Path,
    positions: &mut [f32],
    velocities: Option<&mut [f32]>,
) -> Result<ConBinHeader, ParseError> {
    read_into_f32_threads(path, positions, velocities, 1)
}

/// [`read_into_f32`] with `threads` workers. The split is the same page rule
/// as [`read_into_threads`].
pub fn read_into_f32_threads(
    path: &Path,
    positions: &mut [f32],
    velocities: Option<&mut [f32]>,
    threads: usize,
) -> Result<ConBinHeader, ParseError> {
    read_columns_into(path, positions, velocities, threads, true)
}

fn read_columns_into<T>(
    path: &Path,
    positions: &mut [T],
    velocities: Option<&mut [T]>,
    threads: usize,
    binary32: bool,
) -> Result<ConBinHeader, ParseError> {
    let file = File::open(path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let mut raw = [0u8; FIELD_BYTES];
    (&file)
        .read_exact(&mut raw)
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let (header, offset) = parse_fields(&raw)?;
    if header.binary32 != binary32 {
        return Err(ParseError::ValidationError(if binary32 {
            "CON-bin file is binary64; use read_into".into()
        } else {
            "CON-bin file is binary32; use read_into_f32".into()
        }));
    }
    let n = header.position_len()?;
    let width = std::mem::size_of::<T>();
    let expect = n
        .checked_mul(width)
        .ok_or_else(|| ParseError::InvalidCount("CON-bin column byte length overflows".into()))?;
    if positions.len() != n {
        return Err(ParseError::InvalidCount(format!(
            "position slice has {} values, the file has {n}",
            positions.len()
        )));
    }
    let page = page_size()?;
    let file_len = file
        .metadata()
        .map_err(|err| ParseError::ValidationError(err.to_string()))?
        .len() as usize;
    let pos_off = checked_payload_offset(offset, file_len)? as u64;
    // `bytes_of` is `positions.len() * size_of::<T>()`. After `positions.len() == n`
    // and `expect = n * size_of::<T>()` (the multiply already checked), the two
    // lengths are the same value. A second comparison cannot fail.
    let pos_bytes = bytes_of(positions);
    #[cfg(not(unix))]
    {
        let _ = (pos_bytes, pos_off, threads, page, velocities, header);
        return Err(ParseError::ValidationError(
            "CON-bin pread fill requires Unix".into(),
        ));
    }
    #[cfg(unix)]
    {
        let fd = file.as_raw_fd();
        // Sequential advice matches one pass over the payload. WILLNEED is not
        // used here: it starts a second readahead beside the reads themselves.
        // libc exposes posix_fadvise on Linux. macOS has posix_madvise instead.
        #[cfg(target_os = "linux")]
        unsafe {
            libc::posix_fadvise(fd, 0, 0, libc::POSIX_FADV_SEQUENTIAL);
        }
        // Linux read returns at most MAX_RW_COUNT bytes (INT_MAX aligned down to
        // the page, include/linux/fs.h). `pread_all` continues until the span is full.
        pread_parallel(fd, pos_bytes, pos_off, threads, page)?;
        if header.velocities {
            let Some(vel) = velocities else {
                return Err(ParseError::InvalidCount(
                    "CON-bin has velocities and the caller passed no velocity slice".into(),
                ));
            };
            if vel.len() != n {
                return Err(ParseError::InvalidCount(format!(
                    "velocity slice has {} values, the file has {n}",
                    vel.len()
                )));
            }
            let vel_bytes = bytes_of(vel);
            pread_parallel(fd, vel_bytes, pos_off + expect as u64, threads, page)?;
        }
        Ok(header)
    }
}

#[cfg(unix)]
fn pread_parallel(
    fd: i32,
    dest: &mut [u8],
    offset: u64,
    threads: usize,
    page: usize,
) -> Result<(), ParseError> {
    let threads = threads.max(1);
    let workers = if dest.len() < page.saturating_mul(threads) {
        1
    } else {
        threads
    };
    if workers == 1 {
        return pread_all(fd, dest, offset);
    }
    let chunk = (dest.len() / workers) & !(page - 1);
    std::thread::scope(|scope| {
        let mut rest = dest;
        let mut off = offset;
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let take = if worker + 1 == workers {
                rest.len()
            } else {
                chunk
            };
            let (head, tail) = rest.split_at_mut(take);
            rest = tail;
            let this = off;
            off += take as u64;
            handles.push(scope.spawn(move || pread_all(fd, head, this)));
        }
        handles
            .into_iter()
            .try_for_each(|handle| handle.join().unwrap_or(Err(ParseError::IncompleteFrame)))
    })
}

#[cfg(unix)]
fn pread_raw(fd: i32, ptr: *mut u8, len: usize, mut offset: u64) -> Result<(), ParseError> {
    let mut done = 0;
    while done < len {
        let n = unsafe { libc::pread(fd, ptr.add(done).cast(), len - done, offset as libc::off_t) };
        if n < 0 {
            let err = std::io::Error::last_os_error();
            // POSIX: EINTR means the call was interrupted before a transfer.
            // Restart the same offset. Rust's `read_at` does the same for
            // `ErrorKind::Interrupted`.
            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(ParseError::ValidationError(err.to_string()));
        }
        if n == 0 {
            return Err(ParseError::IncompleteFrame);
        }
        done += n as usize;
        offset += n as u64;
    }
    Ok(())
}

#[cfg(unix)]
fn pread_all(fd: i32, dest: &mut [u8], offset: u64) -> Result<(), ParseError> {
    pread_raw(fd, dest.as_mut_ptr(), dest.len(), offset)
}

#[cfg(unix)]
fn pread_uninit(
    fd: i32,
    dest: &mut [std::mem::MaybeUninit<u8>],
    offset: u64,
) -> Result<(), ParseError> {
    // `as_mut_ptr` of `MaybeUninit<u8>` is the address `pread` may write.
    // The slice type does not require the bytes to be initialised.
    pread_raw(fd, dest.as_mut_ptr().cast::<u8>(), dest.len(), offset)
}

#[cfg(unix)]
fn pread_parallel_uninit(
    fd: i32,
    dest: &mut [std::mem::MaybeUninit<u8>],
    offset: u64,
    threads: usize,
    page: usize,
) -> Result<(), ParseError> {
    let threads = threads.max(1);
    let workers = if dest.len() < page.saturating_mul(threads) {
        1
    } else {
        threads
    };
    if workers == 1 {
        return pread_uninit(fd, dest, offset);
    }
    let chunk = (dest.len() / workers) & !(page - 1);
    std::thread::scope(|scope| {
        let mut rest = dest;
        let mut off = offset;
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let take = if worker + 1 == workers {
                rest.len()
            } else {
                chunk
            };
            let (head, tail) = rest.split_at_mut(take);
            rest = tail;
            let this = off;
            off += take as u64;
            handles.push(scope.spawn(move || pread_uninit(fd, head, this)));
        }
        handles
            .into_iter()
            .try_for_each(|handle| handle.join().unwrap_or(Err(ParseError::IncompleteFrame)))
    })
}

fn write_values<T>(file: &mut File, values: &[T]) -> Result<(), ParseError> {
    let bytes = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    };
    file.write_all(bytes)
        .map_err(|err| ParseError::ValidationError(err.to_string()))
}

fn write_header_page(
    file: &mut File,
    header: &ConBinHeader,
    page: usize,
) -> Result<(), ParseError> {
    let mut header_page = vec![0u8; page];
    let prefix = header_fields(header, page as u64);
    header_page[..FIELD_BYTES].copy_from_slice(&prefix);
    file.write_all(&header_page)
        .map_err(|err| ParseError::ValidationError(err.to_string()))
}

/// Stream a CON text file into a CON-bin of the same frames.
///
/// The header occupies one host page, so the payload starts on a page
/// boundary. Positions of every frame are written as one run, then velocities.
pub fn write_from_con(
    con_path: &Path,
    out_path: &Path,
    binary32: bool,
) -> Result<ConBinHeader, ParseError> {
    let text = std::fs::read_to_string(con_path)
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let mut file =
        File::create(out_path).map_err(|err| ParseError::ValidationError(err.to_string()))?;
    let page = page_size()?;
    write_header_page(
        &mut file,
        &ConBinHeader {
            n_frames: 0,
            n_atoms: 0,
            binary32,
            velocities: true,
        },
        page,
    )?;

    let mut n_frames = 0u64;
    let mut n_atoms = 0u64;
    let mut positions: Vec<f64> = Vec::new();
    let mut velocities: Vec<f64> = Vec::new();
    for frame in ConFrameIterator::new(&text) {
        let frame = frame?;
        let n = frame.atom_data.len();
        if n_frames == 0 {
            n_atoms = n as u64;
        } else if n as u64 != n_atoms {
            return Err(ParseError::InvalidCount(
                "CON-bin requires the same atom count in every frame".into(),
            ));
        }
        for atom in &frame.atom_data {
            positions.extend_from_slice(&[atom.x, atom.y, atom.z]);
            velocities.extend_from_slice(&atom.velocity().unwrap_or([0.0; 3]));
        }
        n_frames += 1;
    }
    if binary32 {
        let pos32: Vec<f32> = positions.iter().map(|v| *v as f32).collect();
        let vel32: Vec<f32> = velocities.iter().map(|v| *v as f32).collect();
        write_values(&mut file, &pos32)?;
        write_values(&mut file, &vel32)?;
    } else {
        write_values(&mut file, &positions)?;
        write_values(&mut file, &velocities)?;
    }
    let header = ConBinHeader {
        n_frames,
        n_atoms,
        binary32,
        velocities: true,
    };
    file.seek(SeekFrom::Start(0))
        .map_err(|err| ParseError::ValidationError(err.to_string()))?;
    write_header_page(&mut file, &header, page)?;
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::fd::AsRawFd;

    fn write_known(path: &Path, frames: usize, atoms: usize) {
        let page = page_size().unwrap();
        let mut file = File::create(path).unwrap();
        let header = ConBinHeader {
            n_frames: frames as u64,
            n_atoms: atoms as u64,
            binary32: false,
            velocities: true,
        };
        write_header_page(&mut file, &header, page).unwrap();
        let n = frames * atoms * 3;
        let pos: Vec<f64> = (0..n).map(|i| i as f64 * 0.25).collect();
        let vel: Vec<f64> = (0..n).map(|i| i as f64 * -0.5).collect();
        write_values(&mut file, &pos).unwrap();
        write_values(&mut file, &vel).unwrap();
    }

    #[test]
    fn a_missing_file_is_an_os_error() {
        let path = std::env::temp_dir().join(format!("conbin-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let err = QueueLimits::of(&path).unwrap_err();
        assert!(
            err.to_string().contains("os error 2"),
            "a missing file must be an os error: {err}"
        );
        match map_file(&path, 1) {
            Err(err) => assert!(err.to_string().contains("os error 2"), "{err}"),
            Ok(_) => panic!("a missing file mapped"),
        }
        match sum_split_streams(&path, 2) {
            Err(err) => assert!(err.to_string().contains("os error 2"), "{err}"),
            Ok(_) => panic!("a missing file summed"),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_cached_file_sums_from_resident_pages() {
        let dir = std::path::PathBuf::from(format!("/dev/shm/conbin-resident-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        write_known(&path, 2, 16);
        let cached = std::fs::read(&path).unwrap();
        assert!(cached.len() > 8);
        let file = File::open(&path).unwrap();
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                cached.len(),
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        let page = page_size().unwrap();
        let pages = cached.len().div_ceil(page);
        let mut resident = vec![0u8; pages];
        assert_eq!(unsafe { libc::mincore(ptr, cached.len(), resident.as_mut_ptr()) }, 0);
        assert!(
            resident.iter().all(|byte| byte & 1 == 1),
            "the page cache dropped the file before the sum"
        );
        unsafe { libc::munmap(ptr, cached.len()) };
        let mapped = map_file(&path, 1).unwrap();
        let expect = mapped.positions_f64().unwrap().iter().sum::<f64>()
            + mapped.velocities_f64().unwrap().iter().sum::<f64>();
        let got = sum_split_streams(&path, 2).unwrap();
        assert_eq!(got, expect, "resident sum {got} mapped sum {expect}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn worker_span_shares_the_queue() {
        let limits = QueueLimits {
            page: 4096,
            logical_block: 512,
            max_request: 1280 * 1024,
            nr_requests: 256,
            read_ahead: 128 * 1024,
            nr_hw_queues: 1,
            rotational: true,
        };
        assert_eq!(limits.worker_span(8), 32 * 1280 * 1024);
        assert_eq!(limits.worker_span(1), 256 * 1280 * 1024);
        // /dev/shm is tmpfs, so it has no block queue. The sum must still run.
        let dir = std::path::PathBuf::from(format!("/dev/shm/conbin-sum-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        write_known(&path, 4, 8);
        let mapped = map_file(&path, 1).unwrap();
        let expect = mapped.positions_f64().unwrap().iter().sum::<f64>()
            + mapped.velocities_f64().unwrap().iter().sum::<f64>();
        let got = sum_mapped(&path, 2).unwrap();
        let got_split = sum_split_streams(&path, 2).unwrap();
        assert_eq!(got, expect, "parallel sum {got} serial sum {expect}");
        assert_eq!(
            got_split, expect,
            "split sum {got_split} serial sum {expect}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let short = QueueLimits {
            nr_requests: 2,
            read_ahead: 4096,
            ..limits
        };
        assert_eq!(short.worker_span(8), 1280 * 1024);
        let shm = std::path::PathBuf::from(format!("/dev/shm/conbin-queue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&shm);
        std::fs::create_dir_all(&shm).unwrap();
        let qpath = shm.join("known.conbin");
        write_known(&qpath, 1, 1);
        let got = QueueLimits::of(&qpath).unwrap();
        assert!(got.page.is_power_of_two());
        assert_eq!(got.max_request, got.page);
        assert_eq!(got.nr_requests, 1);
        assert!(!got.rotational);
        let _ = std::fs::remove_dir_all(&shm);
    }

    #[test]
    fn mapped_runs_match_the_pread_fill() {
        let dir = std::env::temp_dir().join(format!("conbin-map-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        write_known(&path, 4, 3);
        let mapped = map_file(&path, 2).unwrap();
        let pos = mapped.positions_f64().unwrap();
        let vel = mapped.velocities_f64().unwrap();
        assert_eq!(pos.len(), 4 * 3 * 3);
        assert_eq!(pos[0], 0.0);
        assert_eq!(pos[1], 0.25);
        assert_eq!(vel[1], -0.5);
        let mut got_pos = vec![0.0; pos.len()];
        let mut got_vel = vec![0.0; vel.len()];
        read_into_threads(&path, &mut got_pos, Some(&mut got_vel), 2).unwrap();
        assert_eq!(got_pos, pos);
        assert_eq!(got_vel, vel);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_from_con_round_trips_text() {
        let dir = std::env::temp_dir().join(format!("conbin-text-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let con = dir.join("two.con");
        std::fs::write(
            &con,
            include_str!("../resources/test/tiny_cuh2.con").repeat(2),
        )
        .unwrap();
        let bin = dir.join("two.conbin");
        let header = write_from_con(&con, &bin, false).unwrap();
        assert_eq!(header.n_frames, 2);
        assert_eq!(header.n_atoms, 4);
        let mapped = map_file(&bin, 1).unwrap();
        let pos = mapped.positions_f64().unwrap();
        assert_eq!(pos.len(), 2 * 4 * 3);
        assert!(pos[0].is_finite());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_store_copies_the_page_and_leaves_the_file() {
        let dir = std::env::temp_dir().join(format!("conbin-cow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        write_known(&path, 2, 4);
        let before = std::fs::read(&path).unwrap();
        let mut mapped = map_file(&path, 1).unwrap();
        {
            let pos = mapped.positions_f64_mut().unwrap();
            let original = pos[0];
            pos[0] = original + 1.0;
            assert_eq!(pos[0], original + 1.0);
        }
        let after = std::fs::read(&path).unwrap();
        assert_eq!(before, after);
        let reread = map_file(&path, 1).unwrap();
        assert_eq!(reread.positions_f64().unwrap()[0], 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_4k_offset_opens_on_a_16k_or_64k_host() {
        // The stored offset is 4096, and the file is shorter than a 16 KiB
        // or 64 KiB page. The reader keeps 4096. It does not replace that
        // offset with the host page.
        let header = ConBinHeader {
            n_frames: 1,
            n_atoms: 1,
            binary32: false,
            velocities: true,
        };
        let mut bytes = header_fields(&header, 4096).to_vec();
        bytes.resize(4096 + 48, 0);
        bytes[4096..4096 + 8].copy_from_slice(&1.25f64.to_le_bytes());
        bytes[4096 + 24..4096 + 32].copy_from_slice(&(-2.5f64).to_le_bytes());
        assert!(bytes.len() < 16 * 1024);
        assert!(bytes.len() < 64 * 1024);
        let off = checked_payload_offset(4096, bytes.len()).unwrap();
        assert_eq!(off, 4096);
        assert_ne!(off, 16 * 1024);
        assert_ne!(off, 64 * 1024);
        let dir = std::env::temp_dir().join(format!("conbin-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        std::fs::write(&path, &bytes).unwrap();
        let mapped = map_file(&path, 1).unwrap();
        assert_eq!(mapped.payload_off, 4096);
        assert_eq!(mapped.positions_f64().unwrap()[0], 1.25);
        assert_eq!(mapped.velocities_f64().unwrap()[0], -2.5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write_typed(path: &Path, binary32: bool, velocities: bool) {
        let header = ConBinHeader {
            n_frames: 1,
            n_atoms: 1,
            binary32,
            velocities,
        };
        let mut bytes = header_fields(&header, 64).to_vec();
        bytes.resize(64 + 48, 0);
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn sum_mapped_rejects_binary32_and_a_missing_velocity_run() {
        let dir = std::env::temp_dir().join(format!("conbin-kind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary32 = dir.join("f32.conbin");
        write_typed(&binary32, true, true);
        let err = sum_mapped(&binary32, 1).err().unwrap();
        assert!(err.to_string().contains("binary32"), "{err}");
        let bare = dir.join("nov.conbin");
        write_typed(&bare, false, false);
        let err = sum_mapped(&bare, 1).err().unwrap();
        assert!(err.to_string().contains("no velocity"), "{err}");
        let err = read_owned_f64(&bare, 1).err().unwrap();
        assert!(err.to_string().contains("no velocity"), "{err}");
        let known = dir.join("known.conbin");
        write_known(&known, 1, 2);
        let (_header, pos, vel) = read_owned_f64(&known, 1).unwrap();
        let mapped = map_file(&known, 1).unwrap();
        assert_eq!(pos.as_slice(), mapped.positions_f64().unwrap());
        assert_eq!(vel.as_slice(), mapped.velocities_f64().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    fn dir_is_tmpfs(path: &Path) -> bool {
        let text = path.to_string_lossy();
        let Ok(c) = std::ffi::CString::new(text.as_bytes()) else {
            return false;
        };
        let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::statfs(c.as_ptr(), &mut stat) };
        // include/uapi/linux/magic.h TMPFS_MAGIC
        rc == 0 && stat.f_type == 0x01021994
    }

    #[cfg(target_os = "linux")]
    fn streaming_dir() -> Option<std::path::PathBuf> {
        let mut candidates = vec![std::env::temp_dir()];
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd);
        }
        candidates.push(std::path::PathBuf::from("/var/tmp"));
        for base in candidates {
            if base.is_dir() && !dir_is_tmpfs(&base) {
                let dir = base.join(format!("conbin-stream-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&dir);
                if std::fs::create_dir_all(&dir).is_ok() {
                    return Some(dir);
                }
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sum_mapped_streams_when_pages_are_not_resident() {
        let Some(dir) = streaming_dir() else {
            panic!(
                "sum_mapped_streams_when_pages_are_not_resident: temp, the working directory and /var/tmp are tmpfs, and POSIX_FADV_DONTNEED leaves tmpfs pages resident"
            );
        };
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known.conbin");
        write_known(&path, 2, 32);
        let file = File::open(&path).unwrap();
        file.sync_all().unwrap();
        let rc = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
        assert_eq!(rc, 0);
        let len = file.metadata().unwrap().len() as usize;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        let page = page_size().unwrap();
        let pages = len.div_ceil(page);
        let mut vec = vec![0u8; pages];
        let rc = unsafe { libc::mincore(ptr, len, vec.as_mut_ptr()) };
        assert_eq!(rc, 0);
        assert!(
            vec.iter().any(|byte| byte & 1 == 0),
            "every page stayed resident"
        );
        unsafe { libc::munmap(ptr, len) };
        let wide = dir.join("wide.conbin");
        write_known(&wide, 1, 8000);
        let wide_file = File::open(&wide).unwrap();
        wide_file.sync_all().unwrap();
        assert_eq!(
            unsafe { libc::posix_fadvise(wide_file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) },
            0
        );
        let wide_len = wide_file.metadata().unwrap().len() as usize;
        let wide_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                wide_len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                wide_file.as_raw_fd(),
                0,
            )
        };
        assert_ne!(wide_ptr, libc::MAP_FAILED);
        let wide_pages = wide_len.div_ceil(page);
        let mut wide_vec = vec![0u8; wide_pages];
        assert_eq!(unsafe { libc::mincore(wide_ptr, wide_len, wide_vec.as_mut_ptr()) }, 0);
        assert!(
            wide_vec.iter().any(|byte| byte & 1 == 0),
            "wide file stayed resident"
        );
        unsafe { libc::munmap(wide_ptr, wide_len) };
        let limits = QueueLimits::page_default(page);
        let streamed = sequential_resident_sum(&wide, 1, &limits).unwrap();
        assert!(streamed.is_finite(), "{streamed}");
        let split = split_stream_sum(&wide, 2, &limits).unwrap();
        assert!(split.is_finite(), "{split}");
        let got = sum_mapped(&path, 2).unwrap();
        let mapped = map_file(&path, 1).unwrap();
        let expect = mapped.positions_f64().unwrap().iter().sum::<f64>()
            + mapped.velocities_f64().unwrap().iter().sum::<f64>();
        assert_eq!(got, expect, "{got} {expect}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Fills both columns with `pread` into spare capacity.
    /// Miri runs this with `-Zmiri-disable-isolation`.
    /// Miri does not model file-backed mmap, `mincore`, `readahead`
    /// or `MADV_POPULATE_READ`.
    #[cfg(unix)]
    #[test]
    fn read_owned_f64_fills_spare_capacity_by_pread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("known.conbin");
        write_known(&path, 2, 8);
        for threads in [1usize, 2] {
            let (header, pos, vel) = read_owned_f64(&path, threads).unwrap();
            let n = header.position_len().unwrap();
            assert_eq!(pos.len(), n);
            assert_eq!(vel.len(), n);
            let mut raw = Vec::new();
            std::io::Read::read_to_end(&mut File::open(&path).unwrap(), &mut raw).unwrap();
            let off = raw.len() - 2 * n * 8;
            let want: Vec<f64> = raw[off..]
                .chunks_exact(8)
                .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
                .collect();
            assert_eq!(pos.as_slice(), &want[..n]);
            assert_eq!(vel.as_slice(), &want[n..]);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn populate_followup_touches_only_after_einval() {
        assert!(populate_followup(libc::EINVAL).is_ok());
        for code in [libc::EFAULT, libc::EIO, libc::EHWPOISON, libc::ENOMEM] {
            assert_eq!(populate_followup(code).err().unwrap(), code);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wait_resident_rejects_an_unaligned_address() {
        let page = page_size().unwrap();
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        let err = wait_resident(ptr as usize + 1, 0, 1, page).err().unwrap();
        assert!(err.contains("page-aligned"), "{err}");
        unsafe { libc::munmap(ptr, page) };
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wait_resident_returns_an_unmapped_populate_error() {
        let page = page_size().unwrap();
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        unsafe { libc::munmap(ptr, page) };
        let err = wait_resident(ptr as usize, 0, page, page).err().unwrap();
        assert!(err.contains("os error"), "{err}");
    }

    #[test]
    fn an_f64_read_rejects_a_binary32_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f32.conbin");
        let page = page_size().unwrap();
        let mut file = File::create(&path).unwrap();
        let header = ConBinHeader {
            n_frames: 1,
            n_atoms: 1,
            binary32: true,
            velocities: false,
        };
        write_header_page(&mut file, &header, page).unwrap();
        drop(file);
        let mut pos = [0.0f64; 3];
        let err = read_into(&path, &mut pos, None).unwrap_err();
        assert_has(&err, "binary32");
        let wide = dir.path().join("f64.conbin");
        write_known(&wide, 1, 1);
        let err = read_columns_into::<f64>(&wide, &mut pos, None, 1, true).unwrap_err();
        assert_has(&err, "binary64");
    }

    fn assert_has(err: &ParseError, needle: &str) {
        let text = err.to_string();
        assert!(text.contains(needle), "needle {needle} not in {text}");
    }

    #[test]
    fn header_edges_reject_bad_counts_magic_and_offsets() {
        let overflow = ConBinHeader {
            n_frames: u64::MAX,
            n_atoms: 2,
            binary32: false,
            velocities: true,
        };
        assert_has(&overflow.position_len().err().unwrap(), "overflows");
        let times_three = ConBinHeader {
            n_frames: (usize::MAX / 2) as u64,
            n_atoms: 1,
            binary32: true,
            velocities: false,
        };
        assert_has(&times_three.position_len().err().unwrap(), "overflows");

        let header = ConBinHeader {
            n_frames: 2,
            n_atoms: 3,
            binary32: true,
            velocities: false,
        };
        let bytes = header_fields(&header, 128);
        let (parsed, offset) = parse_fields(&bytes).unwrap();
        assert_eq!(parsed, header);
        assert_eq!(offset, 128);
        let other = header_fields(
            &ConBinHeader {
                n_frames: 1,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            64,
        );
        assert!(parse_fields(&other).unwrap().0.velocities);

        assert_has(&parse_fields(&[0u8; 8]).err().unwrap(), "CONBIN01");
        assert_has(&parse_fields(&[0u8; FIELD_BYTES]).err().unwrap(), "CONBIN01");
        let mut bad_version = bytes;
        bad_version[8..12].copy_from_slice(&99u32.to_le_bytes());
        assert_has(&parse_fields(&bad_version).err().unwrap(), "version 99");

        assert_has(&checked_payload_offset(16, 100).err().unwrap(), "8-byte");
        assert_has(&checked_payload_offset(104, 100).err().unwrap(), "8-byte");
        assert_has(&checked_payload_offset(41, 100).err().unwrap(), "8-byte");
        assert_eq!(checked_payload_offset(48, 100).unwrap(), 48);

        let stand = QueueLimits::page_default(4096);
        assert_eq!(stand.page, 4096);
        assert_eq!(stand.logical_block, 4096);
        assert_eq!(stand.max_request, 4096);
        assert_eq!(stand.nr_requests, 1);
        assert_eq!(stand.read_ahead, 4096);
        assert_eq!(stand.nr_hw_queues, 1);
        assert!(!stand.rotational);
        assert_eq!(stand.worker_span(0), 4096);

        let wide = header_fields(
            &ConBinHeader {
                n_frames: (usize::MAX / 3) as u64,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            64,
        );
        assert_has(
            &f64_payload(&wide, usize::MAX).err().unwrap(),
            "column length overflows",
        );
        let bytes_overflow = header_fields(
            &ConBinHeader {
                n_frames: 1 << 60,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            64,
        );
        assert_has(
            &f64_payload(&bytes_overflow, usize::MAX).err().unwrap(),
            "byte length overflows",
        );
        let off = (usize::MAX & !7) as u64;
        let add_overflow = header_fields(
            &ConBinHeader {
                n_frames: 1,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            off,
        );
        assert_has(
            &f64_payload(&add_overflow, usize::MAX).err().unwrap(),
            "payload offset overflows",
        );
        let short = header_fields(
            &ConBinHeader {
                n_frames: 1,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            64,
        );
        let incomplete = f64_payload(&short, 64).err().unwrap();
        assert!(
            format!("{incomplete:?}").contains("IncompleteFrame"),
            "{incomplete}"
        );
    }

    #[cfg(target_os = "linux")]
    fn assert_incomplete(err: &ParseError) {
        assert!(
            format!("{err:?}").contains("IncompleteFrame"),
            "{err}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mapped_reads_cover_width_mismatch_and_short_files() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.conbin");
        assert_has(
            &read_header(&missing).err().unwrap(),
            "os error 2",
        );
        assert_has(&map_file(&missing, 1).err().unwrap(), "os error 2");

        let short = dir.path().join("short.conbin");
        std::fs::write(&short, b"CONBIN").unwrap();
        assert_has(&read_header(&short).err().unwrap(), "fill");
        assert_has(&map_file(&short, 1).err().unwrap(), "CONBIN01");
        let mut one = [0.0];
        assert_has(&read_into(&short, &mut one, None).err().unwrap(), "fill");
        let mut pos_v = Vec::new();
        let mut vel_v = Vec::new();
        assert_has(
            &fill_owned_f64(&short, &mut pos_v, &mut vel_v, 1, 1).err().unwrap(),
            "fill",
        );

        let zeros = dir.path().join("zeros.conbin");
        std::fs::write(&zeros, vec![0u8; FIELD_BYTES]).unwrap();
        assert_has(&read_header(&zeros).err().unwrap(), "CONBIN01");

        let bad_ver = dir.path().join("ver.conbin");
        let mut raw = header_fields(
            &ConBinHeader {
                n_frames: 1,
                n_atoms: 1,
                binary32: false,
                velocities: true,
            },
            64,
        )
        .to_vec();
        raw[8..12].copy_from_slice(&7u32.to_le_bytes());
        raw.resize(64 + 48, 0);
        std::fs::write(&bad_ver, &raw).unwrap();
        assert_has(&read_header(&bad_ver).err().unwrap(), "version 7");
        let huge = dir.path().join("huge.conbin");
        std::fs::write(
            &huge,
            header_fields(
                &ConBinHeader {
                    n_frames: 1 << 60,
                    n_atoms: 1,
                    binary32: false,
                    velocities: false,
                },
                64,
            ),
        )
        .unwrap();
        assert_has(
            &read_into(&huge, &mut [0.0; 0], None).err().unwrap(),
            "byte length overflows",
        );

        let known = dir.path().join("known.conbin");
        write_known(&known, 1, 2);
        let overflow = fill_owned_f64(&known, &mut pos_v, &mut vel_v, usize::MAX / 2, 1).err().unwrap();
        assert_has(&overflow, "byte length overflows");

        let mut pos = vec![0.0; 6];
        let header = read_into(&known, &mut pos, Some(&mut vec![0.0; 6])).unwrap();
        assert_eq!(header.n_atoms, 2);
        assert_eq!(pos[1], 0.25);
        assert_has(
            &read_into(&known, &mut [0.0; 1], None).err().unwrap(),
            "position slice",
        );
        assert_has(
            &read_into(&known, &mut pos, None).err().unwrap(),
            "no velocity slice",
        );
        assert_has(
            &read_into_threads(&known, &mut pos, Some(&mut [0.0; 1]), 1).err().unwrap(),
            "velocity slice",
        );

        let bare = dir.path().join("nov.conbin");
        write_typed(&bare, false, false);
        let mut bare_pos = vec![0.0; 3];
        let bare_header = read_into(&bare, &mut bare_pos, None).unwrap();
        assert!(!bare_header.velocities);
        assert_eq!(bare_pos, vec![0.0; 3]);

        let f32_path = dir.path().join("f32.conbin");
        write_f32_file(&f32_path, 1, 1);
        assert_has(&read_owned_f64(&f32_path, 1).err().unwrap(), "binary32");
        assert_has(&read_into(&f32_path, &mut bare_pos, None).err().unwrap(), "read_into_f32");
        let mut p32 = vec![0.0f32; 3];
        let mut v32 = vec![0.0f32; 3];
        let h32 = read_into_f32(&f32_path, &mut p32, Some(&mut v32)).unwrap();
        assert!(h32.binary32);
        assert_eq!(p32, vec![0.0, 0.5, 1.0]);
        assert_eq!(v32, vec![0.0, -1.0, -2.0]);
        let mapped32 = map_file(&f32_path, 1).unwrap();
        assert_eq!(mapped32.positions_f32().unwrap(), p32.as_slice());
        assert_eq!(mapped32.velocities_f32().unwrap(), v32.as_slice());
        assert_has(
            &read_into_f32_threads(&known, &mut p32, Some(&mut v32), 2).err().unwrap(),
            "read_into",
        );

        let mut mapped = map_file(&known, 0).unwrap();
        assert_eq!(mapped.positions_f64().unwrap()[0], 0.0);
        mapped.payload_off = 1;
        assert_has(&mapped.positions_f64().err().unwrap(), "aligned");
        assert_has(&mapped.positions_f64_mut().err().unwrap(), "aligned");

        mapped.payload_off = usize::MAX - 10;
        mapped.header.n_frames = 1;
        mapped.header.n_atoms = 1;
        assert_has(&mapped.positions_f64().err().unwrap(), "extends past");
        assert_has(&mapped.positions_f64_mut().err().unwrap(), "extends past");
        assert_has(&mapped.velocities_f64().err().unwrap(), "payload offset overflows");
        assert_has(
            &mapped.run_mut::<f64>(8, 2).err().unwrap(),
            "payload offset overflows",
        );

        mapped.payload_off = 64;
        mapped.header.n_frames = 1 << 62;
        mapped.header.n_atoms = 1;
        assert_has(&mapped.positions_f64().err().unwrap(), "byte length overflows");
        assert_has(
            &mapped.positions_f64_mut().err().unwrap(),
            "byte length overflows",
        );
        assert_has(&mapped.velocities_f64().err().unwrap(), "byte length overflows");
        assert_has(
            &mapped.run::<f64>(8, usize::MAX / 2).err().unwrap(),
            "byte length overflows",
        );
        assert_has(
            &mapped.run_mut::<f64>(8, usize::MAX / 2).err().unwrap(),
            "byte length overflows",
        );

        mapped.header.n_frames = 1;
        mapped.header.n_atoms = 10_000;
        mapped.payload_off = 64;
        assert_incomplete(&mapped.positions_f64().err().unwrap());
        assert_incomplete(&mapped.positions_f64_mut().err().unwrap());

        let empty = dir.path().join("empty.conbin");
        File::create(&empty).unwrap();
        assert_has(&open_map(&empty).err().unwrap(), "os error");
        assert_has(&map_file(dir.path(), 1).err().unwrap(), "os error");

        let mut full = File::create("/dev/full").unwrap();
        assert_has(&write_values(&mut full, &[1.0f64]).err().unwrap(), "os error 28");
        assert_has(
            &write_header_page(
                &mut full,
                &ConBinHeader {
                    n_frames: 1,
                    n_atoms: 1,
                    binary32: false,
                    velocities: true,
                },
                4096,
            )
            .err().unwrap(),
            "os error 28",
        );

        let con = dir.path().join("tiny.con");
        std::fs::write(&con, include_str!("../resources/test/tiny_cuh2.con")).unwrap();
        assert_has(
            &write_from_con(Path::new("/no/such-conbin.con"), &dir.path().join("out.conbin"), false)
                .err().unwrap(),
            "os error 2",
        );
        let mismatch = dir.path().join("mismatch.con");
        std::fs::write(
            &mismatch,
            format!(
                "{}{}",
                include_str!("../resources/test/tiny_cuh2.con"),
                ONE_ATOM_CON
            ),
        )
        .unwrap();
        assert_has(
            &write_from_con(&mismatch, &dir.path().join("bad.conbin"), false).err().unwrap(),
            "same atom count",
        );
        let bin32 = dir.path().join("from.conbin");
        let written = write_from_con(&con, &bin32, true).unwrap();
        assert!(written.binary32);
        assert_eq!(written.n_frames, 1);
        assert_eq!(written.n_atoms, 4);
        let n = written.position_len().unwrap();
        let mut got_p = vec![0.0f32; n];
        let mut got_v = vec![0.0f32; n];
        read_into_f32_threads(&bin32, &mut got_p, Some(&mut got_v), 2).unwrap();
        assert!(got_p.iter().any(|value| *value != 0.0));
        assert_eq!(read_header(&bin32).unwrap().n_atoms, 4);
        let again = map_file_with(&bin32, 1, Prefetch::Span).unwrap();
        assert_eq!(again.positions_f32().unwrap()[0], got_p[0]);
    }

    const ONE_ATOM_CON: &str = r#"Random Number Seed
{"con_spec_version":2}
15.345600 21.702000 100.000000
90.000000 90.000000 90.000000
0 0
218 0 1
1
1
63.546000
Cu
Coordinates of Component 1
   0.639400 0.904500 6.975300 7 0
"#;

    fn write_f32_file(path: &Path, frames: u64, atoms: u64) {
        let page = page_size().unwrap();
        let header = ConBinHeader {
            n_frames: frames,
            n_atoms: atoms,
            binary32: true,
            velocities: true,
        };
        let mut bytes = vec![0u8; page];
        bytes[..FIELD_BYTES].copy_from_slice(&header_fields(&header, page as u64));
        let n = (frames as usize) * (atoms as usize) * 3;
        for i in 0..n {
            bytes.extend_from_slice(&((i as f32) * 0.5).to_le_bytes());
        }
        for i in 0..n {
            bytes.extend_from_slice(&((i as f32) * -1.0).to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[cfg(target_os = "linux")]
    fn drop_pages(path: &Path) {
        let file = File::open(path).unwrap();
        file.sync_all().unwrap();
        let rc = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
        assert_eq!(rc, 0);
    }

    #[cfg(target_os = "linux")]
    fn nonresident_pages(path: &Path) -> usize {
        let file = File::open(path).unwrap();
        let len = file.metadata().unwrap().len() as usize;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        let page = page_size().unwrap();
        let pages = len.div_ceil(page);
        let mut vec = vec![0u8; pages];
        let rc = unsafe { libc::mincore(ptr, len, vec.as_mut_ptr()) };
        assert_eq!(rc, 0, "{}", std::io::Error::last_os_error());
        unsafe { libc::munmap(ptr, len) };
        vec.iter().filter(|byte| *byte & 1 == 0).count()
    }

    #[cfg(target_os = "linux")]
    fn series_sum(n: usize) -> f64 {
        let n = n as f64;
        let sum_i = n * (n - 1.0) / 2.0;
        sum_i * 0.25 + sum_i * -0.5
    }

    #[cfg(target_os = "linux")]
    fn small_limits() -> QueueLimits {
        let page = page_size().unwrap();
        QueueLimits {
            page,
            logical_block: 512,
            max_request: page,
            nr_requests: 1,
            read_ahead: page,
            nr_hw_queues: 1,
            rotational: false,
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parallel_prefetch_and_cold_sums_match_the_series() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.conbin");
        write_known(&path, 1, 700);
        let n = 700 * 3;
        let mut pos = vec![0.0; n];
        let mut vel = vec![0.0; n];
        read_into_threads(&path, &mut pos, Some(&mut vel), 2).unwrap();
        assert_eq!(pos.len(), n);
        assert_eq!(pos[1], 0.25);
        assert_eq!(vel[n - 1], (n - 1) as f64 * -0.5);
        let mut pos0 = vec![0.0; n];
        let mut vel0 = vec![0.0; n];
        read_into_threads(&path, &mut pos0, Some(&mut vel0), 0).unwrap();
        assert_eq!(pos0, pos);
        let (header, owned_p, owned_v) = read_owned_f64(&path, 4).unwrap();
        assert_eq!(header.position_len().unwrap(), n);
        assert_eq!(owned_p, pos);
        assert_eq!(owned_v, vel);

        let truncated = dir.path().join("trunc.conbin");
        let page = page_size().unwrap();
        let mut bytes = vec![0u8; page];
        bytes[..FIELD_BYTES].copy_from_slice(&header_fields(
            &ConBinHeader {
                n_frames: 1,
                n_atoms: 700,
                binary32: false,
                velocities: true,
            },
            page as u64,
        ));
        std::fs::write(&truncated, &bytes).unwrap();
        assert_incomplete(&read_into_threads(&truncated, &mut pos, Some(&mut vel), 2).err().unwrap());
        assert_incomplete(&read_owned_f64(&truncated, 4).err().unwrap());

        let file = File::open(&path).unwrap();
        let mut buf = [0u8; 8];
        assert_incomplete(&pread_raw(file.as_raw_fd(), buf.as_mut_ptr(), 8, 1_000_000_000).err().unwrap());
        let directory = File::open(dir.path()).unwrap();
        assert_has(
            &pread_raw(directory.as_raw_fd(), buf.as_mut_ptr(), 4, 0).err().unwrap(),
            "os error 21",
        );

        for prefetch in [
            Prefetch::Span,
            Prefetch::ChunkPopulate,
            Prefetch::ChunkWillneed,
            Prefetch::Hybrid,
        ] {
            let mapped = map_file_with(&path, 2, prefetch).unwrap();
            assert_eq!(mapped.positions_f64().unwrap()[1], 0.25);
            assert_eq!(mapped.velocities_f64().unwrap()[0], 0.0);
        }

        let cold = dir.path().join("cold.conbin");
        write_known(&cold, 8, 8_000);
        let cold_n = 8 * 8_000 * 3;
        let cold_expect = series_sum(cold_n);
        let limits = small_limits();
        drop_pages(&cold);
        assert!(
            nonresident_pages(&cold) > 0,
            "pages stayed resident after dropping them"
        );
        assert_eq!(
            sequential_resident_sum(&cold, 2, &limits).unwrap(),
            cold_expect
        );
        let wide = dir.path().join("wide-cold.conbin");
        write_known(&wide, 4, 400_000);
        let wide_n = 4 * 400_000 * 3;
        let wide_expect = series_sum(wide_n);
        let deep = QueueLimits {
            page: limits.page,
            logical_block: limits.logical_block,
            max_request: limits.max_request,
            nr_requests: 100_000,
            read_ahead: limits.read_ahead,
            nr_hw_queues: limits.nr_hw_queues,
            rotational: limits.rotational,
        };
        drop_pages(&wide);
        assert_eq!(
            sequential_resident_sum(&wide, 1, &deep).unwrap(),
            wide_expect
        );
        drop_pages(&cold);
        assert_eq!(split_stream_sum(&cold, 2, &limits).unwrap(), cold_expect);
        drop_pages(&cold);
        assert_eq!(sum_mapped(&cold, 2).unwrap(), cold_expect);
        drop_pages(&cold);
        assert_eq!(sum_split_streams(&cold, 2).unwrap(), cold_expect);
        let owned = open_map(&cold).unwrap();
        let spanned = stream_one_span(
            &cold,
            SpanGeom {
                base: owned.ptr as usize,
                span: 0,
                end: owned.len,
                page,
                payload_off: page,
                payload_end: page + cold_n * 16,
                request: page,
                depth: 1,
            },
        )
        .unwrap();
        assert_eq!(spanned, cold_expect);
        drop(owned);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_prefetch_helpers_cover_empty_and_split_ranges() {
        let page = page_size().unwrap();
        assert!(worker_ranges(0, 4, page).is_empty());
        assert_eq!(worker_ranges(page, 4, page), vec![(0, page)]);
        assert_eq!(worker_ranges(10, 0, page), vec![(0, 10)]);
        let split = worker_ranges(page * 3, 2, page);
        assert_eq!(split.len(), 2);
        assert_eq!(split[0], (0, page));
        assert_eq!(split[1], (page, page * 3));

        let empty = memmap2::MmapOptions::new().len(0).map_anon().unwrap();
        assert_eq!(empty.len(), 0);
        advise_spans(&empty, memmap2::Advice::Sequential, 4, page).unwrap();
        assert!(pages_all_resident(std::ptr::null_mut(), 0, page).unwrap());
        assert_has(
            &pages_all_resident(0x1000 as *mut libc::c_void, page, page).err().unwrap(),
            "os error",
        );

        let mut big = memmap2::MmapOptions::new().len(page * 3).map_anon().unwrap();
        advise_spans(&big, memmap2::Advice::PopulateRead, 2, page).unwrap();
        let mut resident = vec![0u8; 3];
        let rc = unsafe { libc::mincore(big.as_ptr() as *mut libc::c_void, big.len(), resident.as_mut_ptr()) };
        assert_eq!(rc, 0);
        assert!(resident.iter().all(|byte| byte & 1 == 1), "{resident:?}");

        unsafe { std::ptr::write(big.as_mut_ptr(), 0x11) };
        unsafe { std::ptr::write(big.as_mut_ptr().add(page), 0x22) };
        touch_pages(big.as_ptr() as usize, 0, page + 16, page);
        assert_eq!(unsafe { std::ptr::read(big.as_ptr()) }, 0x11);
        assert_eq!(unsafe { std::ptr::read(big.as_ptr().add(page)) }, 0x22);
        let err = wait_resident(big.as_ptr() as usize, 0, page, 0).err().unwrap();
        assert!(err.contains("page-aligned"), "{err}");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("span.conbin");
        write_known(&path, 1, 8);
        let file = File::open(&path).unwrap();
        advise_readahead(&file, 0, page).unwrap();
        let directory = File::open(dir.path()).unwrap();
        let err = advise_readahead(&directory, 0, page).err().unwrap();
        assert!(err.contains("Invalid"), "{err}");
        willneed(&file, 0, page).unwrap();

        let fifo = dir.path().join("pipe");
        let c_fifo = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_fifo.as_ptr(), 0o644) }, 0);
        let fifo_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&fifo)
            .unwrap();
        assert_has(&willneed(&fifo_file, 0, page).err().unwrap(), "posix_fadvise");

        let mapped = map_file(&path, 1).unwrap();
        assert_has(&populate_range(&mapped.map, 1, page).err().unwrap(), "page-aligned");
        populate_range(&mapped.map, 0, page).unwrap();
        chunk_worker(&mapped.map, 0, 0, page, page, ChunkOp::Populate).unwrap();
        chunk_worker(&mapped.map, 0, 0, page, page, ChunkOp::Hybrid { path: &path }).unwrap();
        let end = page.min(mapped.map.len());
        chunk_worker(
            &mapped.map,
            0,
            mapped.map.len().min(page * 3),
            end.max(1),
            page,
            ChunkOp::Hybrid { path: &path },
        )
        .unwrap();
        chunk_worker(&mapped.map, 0, end, page, page, ChunkOp::Willneed { path: &path }).unwrap();
        chunk_worker(&mapped.map, 0, end, page, page, ChunkOp::Populate).unwrap();
        assert_has(
            &chunk_worker(
                &mapped.map,
                0,
                end,
                page,
                page,
                ChunkOp::Willneed { path: &fifo },
            )
            .err().unwrap(),
            "posix_fadvise",
        );

        let missing = stream_one_span(
            Path::new("/no/such-conbin-span"),
            SpanGeom {
                base: 0,
                span: 0,
                end: page,
                page,
                payload_off: 0,
                payload_end: 0,
                request: page,
                depth: 1,
            },
        )
        .err().unwrap();
        assert!(missing.contains("os error 2"), "{missing}");
        let denied = stream_one_span(
            dir.path(),
            SpanGeom {
                base: 0,
                span: 0,
                end: page,
                page,
                payload_off: 0,
                payload_end: 0,
                request: page,
                depth: 1,
            },
        )
        .err().unwrap();
        assert!(denied.contains("Invalid"), "{denied}");

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page,
                libc::PROT_READ,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        let mut message = None;
        let mut held = Some(ptr);
        for round in 0..8 {
            let again = unsafe {
                libc::mmap(
                    ptr,
                    page,
                    libc::PROT_READ,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED_NOREPLACE,
                    -1,
                    0,
                )
            };
            if again != libc::MAP_FAILED {
                unsafe { libc::munmap(again, page) };
            }
            if let Some(page_ptr) = held.take() {
                assert_eq!(again, libc::MAP_FAILED);
                unsafe { libc::munmap(page_ptr, page) };
            }
            let unmapped = stream_one_span(
                &path,
                SpanGeom {
                    base: ptr as usize,
                    span: 0,
                    end: page,
                    page,
                    payload_off: 0,
                    payload_end: page,
                    request: page,
                    depth: 1,
                },
            );
            if round == 0 {
                continue;
            }
            if let Err(err) = unmapped {
                message = Some(err);
                break;
            }
        }
        let message = message.expect("unmapped populate");
        assert!(message.contains("os error"), "{message}");
    }

    #[cfg(target_os = "linux")]
    fn ensure_scratch() {
        if std::env::var_os("TMPDIR").is_none() {
            unsafe { std::env::set_var("TMPDIR", std::env::temp_dir()) };
        }
    }

    #[cfg(target_os = "linux")]
    fn set_case(name: &str) {
        ensure_scratch();
        let path = std::env::temp_dir().join(format!("conbin-active-{}", std::process::id()));
        std::fs::write(path, name).unwrap();
    }

    #[cfg(target_os = "linux")]
    fn injected(name: &str) -> bool {
        ensure_scratch();
        std::env::temp_dir()
            .join(format!("conbin-mark-{}-{name}", std::process::id()))
            .is_file()
    }

    #[cfg(target_os = "linux")]
    fn compile_preload() -> String {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let source = std::env::temp_dir().join("conbin_cov_preload.c");
            let object = std::env::temp_dir().join("conbin_cov_preload.so");
            std::fs::write(&source, PRELOAD_C).unwrap();
            let out = std::process::Command::new("gcc")
                .args([
                    "-shared",
                    "-fPIC",
                    "-Wall",
                    "-Werror",
                    "-o",
                    object.to_str().unwrap(),
                    source.to_str().unwrap(),
                    "-ldl",
                ])
                .output()
                .expect("gcc");
            assert!(
                out.status.success(),
                "gcc preload: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        });
        std::env::temp_dir()
            .join("conbin_cov_preload.so")
            .to_string_lossy()
            .into_owned()
    }

    #[cfg(target_os = "linux")]
    fn spawn_case(test: &str, case: &str, unshare_mount: bool, preload: bool) {
        ensure_scratch();
        let exe = std::env::current_exe().unwrap();
        let mut cmd = if unshare_mount {
            let mut command = std::process::Command::new("unshare");
            command.args(["--user", "--map-root-user", "--mount", "--"]);
            command.arg(&exe);
            command
        } else {
            std::process::Command::new(&exe)
        };
        if preload {
            cmd.env("LD_PRELOAD", compile_preload());
        }
        let out = cmd
            .arg(test)
            .arg("--exact")
            .arg("--nocapture")
            .env("CONBIN_CASE", case)
            .env("RUST_BACKTRACE", "1")
            .output()
            .expect("spawn");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "case {case} status {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            out.status
        );
        assert!(
            stdout.contains(&format!("CASE_OK {case}")),
            "case {case}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }

    #[cfg(target_os = "linux")]
    fn run_fault_suite() {
        let page = page_size().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let known = dir.path().join("known.conbin");
        write_known(&known, 2, 4);
        let stat_path = dir.path().join("conbin-statfail");
        write_known(&stat_path, 1, 1);
        let reopen = dir.path().join("conbin-reopen-fail");
        write_known(&reopen, 8, 8_000);
        let limits = small_limits();

        set_case("pagesize-fail");
        assert_has(&page_size().unwrap_err(), "SC_PAGESIZE");
        set_case("pagesize-odd");
        assert_has(&page_size().unwrap_err(), "power of two");
        set_case("");
        assert!(page_size().unwrap().is_power_of_two());

        assert_has(&QueueLimits::of(&stat_path).unwrap_err(), "os error 5");
        let err = open_map(&stat_path).err().unwrap();
        assert_has(&err, "os error 5");
        let mut pos = vec![0.0; 3];
        let mut vel = vec![0.0; 3];
        assert_has(
            &read_into(&stat_path, &mut pos, Some(&mut vel)).unwrap_err(),
            "os error 5",
        );
        let mut spare_p = Vec::new();
        let mut spare_v = Vec::new();
        assert_has(
            &fill_owned_f64(&stat_path, &mut spare_p, &mut spare_v, 3, 1).unwrap_err(),
            "os error 5",
        );

        set_case("pread-eintr");
        let n = 2 * 4 * 3;
        let mut got_p = vec![0.0; n];
        let mut got_v = vec![0.0; n];
        read_into(&known, &mut got_p, Some(&mut got_v)).unwrap();
        assert_eq!(got_p[1], 0.25);
        assert_eq!(got_v[1], -0.5);
        assert!(injected("pread-eintr"));
        set_case("");

        set_case("");
        drop_pages(&reopen);
        let mut seq_err = None;
        let mut saw_ok = false;
        for step in 0..8 {
            drop_pages(&reopen);
            if step == 1 {
                set_case("open-fail-seq");
            }
            match sequential_resident_sum(&reopen, 1, &limits) {
                Ok(sum) => {
                    saw_ok = true;
                    assert!(sum.is_finite(), "{sum}");
                }
                Err(err) => {
                    seq_err = Some(err);
                    break;
                }
            }
        }
        assert!(saw_ok, "one resident sum succeeds before the open failure");
        assert_has(
            seq_err.as_ref().expect("open-fail-seq must fail"),
            "os error 13",
        );
        set_case("");
        drop_pages(&reopen);
        set_case("open-fail-split");
        let mut split_err = None;
        for _ in 0..8 {
            drop_pages(&reopen);
            match split_stream_sum(&reopen, 1, &limits) {
                Err(err) => {
                    split_err = Some(err);
                    break;
                }
                Ok(_) => continue,
            }
        }
        assert_has(
            &split_err.expect("open-fail-split did not fail"),
            "os error 13",
        );
        set_case("");
        drop_pages(&reopen);
        set_case("readahead-fail");
        assert_has(
            &sequential_resident_sum(&reopen, 1, &limits).unwrap_err(),
            "Invalid",
        );
        set_case("");
        drop_pages(&reopen);
        set_case("readahead-fail-2");
        let mut readahead_err = None;
        for _ in 0..8 {
            drop_pages(&reopen);
            match sequential_resident_sum(&reopen, 1, &limits) {
                Err(err) => {
                    readahead_err = Some(err);
                    break;
                }
                Ok(_) => continue,
            }
        }
        assert_has(
            &readahead_err.expect("readahead-fail-2 did not fail"),
            "Invalid",
        );
        set_case("");
        drop_pages(&reopen);
        set_case("madvise-eio");
        assert_has(
            &sequential_resident_sum(&reopen, 1, &limits).unwrap_err(),
            "os error 5",
        );
        let advise_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&reopen)
            .unwrap();
        let mut advise_map = unsafe { memmap2::MmapMut::map_mut(&advise_file) }.unwrap();
        let one = advise_spans(&advise_map, memmap2::Advice::WillNeed, 1, page).unwrap_err();
        assert!(one.to_string().contains("os error"), "{one}");
        let many = advise_spans(&advise_map, memmap2::Advice::WillNeed, 8, page).unwrap_err();
        assert!(many.to_string().contains("os error"), "{many}");
        drop(advise_map);
        assert!(injected("madvise-eio"));
        set_case("");

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);
        unsafe { std::ptr::write(ptr as *mut u8, 0x5A) };
        set_case("madvise-einval");
        wait_resident(ptr as usize, 0, page, page).unwrap();
        assert_eq!(unsafe { std::ptr::read(ptr as *const u8) }, 0x5A);
        assert!(injected("madvise-einval"));
        set_case("madvise-eagain-once");
        wait_resident(ptr as usize, 0, page, page).unwrap();
        assert!(injected("madvise-eagain-once"));
        set_case("madvise-eagain-many");
        let again = wait_resident(ptr as usize, 0, page, page).unwrap_err();
        assert!(again.contains("os error 11"), "{again}");
        assert!(injected("madvise-eagain-many"));
        unsafe { libc::munmap(ptr, page) };

        let con = dir.path().join("tiny.con");
        std::fs::write(&con, include_str!("../resources/test/tiny_cuh2.con")).unwrap();
        set_case("lseek-fail");
        assert_has(
            &write_from_con(&con, &dir.path().join("seek.conbin"), false).unwrap_err(),
            "os error 29",
        );
        set_case("");
    }

    #[cfg(target_os = "linux")]
    fn make_rprivate() {
        let root = std::ffi::CString::new("/").unwrap();
        let rc = unsafe {
            libc::mount(
                std::ptr::null(),
                root.as_ptr(),
                std::ptr::null(),
                libc::MS_REC | libc::MS_PRIVATE,
                std::ptr::null(),
            )
        };
        assert_eq!(rc, 0, "{}", std::io::Error::last_os_error());
    }

    #[cfg(target_os = "linux")]
    fn mount_tmpfs(target: &str) {
        let src = std::ffi::CString::new("tmpfs").unwrap();
        let tgt = std::ffi::CString::new(target).unwrap();
        let kind = std::ffi::CString::new("tmpfs").unwrap();
        let rc = unsafe { libc::mount(src.as_ptr(), tgt.as_ptr(), kind.as_ptr(), 0, std::ptr::null()) };
        assert_eq!(rc, 0, "mount {target}: {}", std::io::Error::last_os_error());
    }

    #[cfg(target_os = "linux")]
    fn mount_bind(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).unwrap();
        let from = std::ffi::CString::new(src.to_str().unwrap()).unwrap();
        let to = std::ffi::CString::new(dst.to_str().unwrap()).unwrap();
        let rc = unsafe {
            libc::mount(
                from.as_ptr(),
                to.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND,
                std::ptr::null(),
            )
        };
        assert_eq!(
            rc,
            0,
            "bind {} -> {}: {}",
            src.display(),
            dst.display(),
            std::io::Error::last_os_error()
        );
    }

    /// A tmpfs on the scratch directory hides a coverage profile that lives there.
    /// Bind the profile directory aside first, then put it back at the
    /// same path so the process still writes the profile at exit.
    #[cfg(target_os = "linux")]
    fn hold_coverage_profile() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let raw = std::env::var("LLVM_PROFILE_FILE").ok()?;
        let literal = raw.split('%').next().unwrap_or(raw.as_str());
        let dir = std::path::Path::new(literal).parent()?.to_path_buf();
        // Callers set the profile under target/cov-live or leave it unset.
        // hold_coverage_profile: run_namespace_suite, the outside-profile
        // test, and conbin_namespace_child. None of them build a profile
        // under a temporary directory.
        let live = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/cov-live");
        if !dir.starts_with(&live) {
            return None;
        }
        let hold = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/cov-hold-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&hold);
        mount_bind(&dir, &hold);
        Some((dir, hold))
    }

    #[cfg(target_os = "linux")]
    fn write_sys(path: &str, body: &str) {
        let path = Path::new(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[cfg(target_os = "linux")]
    fn mkdev(major: u64, minor: u64) -> u64 {
        ((major & 0xfff) << 8)
            | (minor & 0xff)
            | ((minor & !0xff) << 12)
            | ((major & !0xfff) << 32)
    }

    #[cfg(target_os = "linux")]
    fn run_namespace_suite() {
        let held = hold_coverage_profile();
        make_rprivate();
        mount_tmpfs("/sys/dev/block");
        assert_eq!(dev_nums(mkdev(9, 1)), (9, 1));
        assert_eq!(dev_nums(mkdev(9, 5)), (9, 5));
        write_sys("/sys/dev/block/9:1/queue/nr_hw_queues", "4\n");
        write_sys("/sys/dev/block/9:2/queue/nr_hw_queues", "0\n");
        write_sys("/sys/dev/block/9:3/queue/nr_hw_queues", "nope\n");
        std::fs::create_dir_all("/sys/dev/block/9:4/mq/0").unwrap();
        std::fs::create_dir_all("/sys/dev/block/9:4/mq/1").unwrap();
        write_sys("/sys/dev/block/9:5/partition", "1\n");
        write_sys("/sys/dev/block/9:6/partition", "1\n");
        write_sys("/sys/dev/block/9:7/partition", "1\n");
        write_sys("/sys/dev/block/queue/logical_block_size", "512\n");
        std::fs::create_dir_all("/sys/dev/block/mq/0").unwrap();
        std::fs::create_dir_all("/sys/dev/block/mq/1").unwrap();
        std::fs::create_dir_all("/sys/dev/block/mq/2").unwrap();
        assert_eq!(queue_text(mkdev(9, 1), "nr_hw_queues").unwrap().trim(), "4");
        assert_eq!(queue_text(mkdev(9, 5), "logical_block_size").unwrap().trim(), "512");
        assert!(queue_text(mkdev(9, 6), "no_such_queue_file").is_none());
        assert_eq!(hw_queue_count(mkdev(9, 1)), 4);
        assert_eq!(hw_queue_count(mkdev(9, 2)), 1);
        assert_eq!(hw_queue_count(mkdev(9, 3)), 1);
        assert_eq!(hw_queue_count(mkdev(9, 4)), 2);
        assert_eq!(hw_queue_count(mkdev(9, 7)), 3);
        let defaults = QueueLimits::from_sysfs(mkdev(9, 8), 4096);
        assert_eq!(defaults.page, 4096);
        assert_eq!(defaults.logical_block, 4096);
        assert_eq!(defaults.max_request, 4096);
        assert_eq!(defaults.nr_requests, 1);
        assert_eq!(defaults.read_ahead, 4096);
        assert_eq!(defaults.nr_hw_queues, 1);
        assert!(!defaults.rotational);

        // Mount the scratch directory, not the repository. A tmpfs on the
        // working directory would hide the coverage profile.
        let scratch = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/conbin-cwd-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&scratch).unwrap();
        std::env::set_current_dir(&scratch).unwrap();
        let tmp = format!("{}{}", "/", "tmp");
        let var_tmp = format!("{}{}{}{}", "/", "var", "/", "tmp");
        mount_tmpfs(&tmp);
        mount_tmpfs(&var_tmp);
        if let Some((dir, hold)) = &held {
            // The held directory is under target/cov-live, so the tmpfs
            // just mounted on the temporary directories does not cover it.
            mount_tmpfs(dir.to_str().unwrap());
            std::fs::create_dir_all(dir).unwrap();
            mount_bind(hold, dir);
            let marker = dir.join("marker");
            if marker.is_file() {
                assert_eq!(std::fs::read(&marker).unwrap(), b"held");
            }
        }
        mount_tmpfs(scratch.to_str().unwrap());
        unsafe { std::env::set_var("TMPDIR", &tmp) };
        let caught = std::panic::catch_unwind(sum_mapped_streams_when_pages_are_not_resident);
        let message = caught.expect_err("streaming dir should be exhausted");
        let text = message
            .downcast_ref::<&str>()
            .copied()
            .unwrap_or("not a string panic");
        assert!(text.contains("tmpfs"), "{text}");
    }

    #[cfg(target_os = "linux")]
    fn run_stream_suite() {
        use std::os::unix::fs::PermissionsExt;
        let dev_null = format!("{}{}{}{}", "/", "dev", "/", "null");
        let dev_shm = format!("{}{}{}{}", "/", "dev", "/", "shm");
        unsafe { std::env::set_var("TMPDIR", &dev_null) };
        let got = streaming_dir().unwrap();
        assert!(got.is_dir(), "{got:?}");
        assert!(!got.starts_with(&dev_null), "{got:?}");

        unsafe { std::env::set_var("TMPDIR", &dev_shm) };
        let got = streaming_dir().unwrap();
        assert!(got.is_dir(), "{got:?}");
        assert!(!got.starts_with(&dev_shm), "{got:?}");

        unsafe { std::env::remove_var("TMPDIR") };
        let ro = std::env::temp_dir().join(format!("conbin-ro-{}", std::process::id()));
        std::fs::create_dir_all(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
        unsafe { std::env::set_var("TMPDIR", &ro) };
        let got = streaming_dir().unwrap();
        assert!(got.is_dir(), "{got:?}");
        assert!(!got.starts_with(&ro), "{got:?}");
        assert_eq!(std::fs::read_dir(&ro).unwrap().count(), 0);

        unsafe { std::env::remove_var("TMPDIR") };
        let gone = std::env::temp_dir().join(format!("conbin-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&gone).unwrap();
        std::env::set_current_dir(&gone).unwrap();
        std::fs::remove_dir(&gone).unwrap();
        let err = std::env::current_dir().unwrap_err();
        assert_eq!(err.raw_os_error(), Some(2), "{err}");
        let got = streaming_dir();
        assert!(got.is_some(), "{got:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dir_is_tmpfs_reports_each_kind() {
        assert!(!dir_is_tmpfs(Path::new("bad\0path")));
        assert!(!dir_is_tmpfs(Path::new("/no/such/conbin/dir-is-tmpfs")));
        let dev_shm = format!("{}{}{}{}", "/", "dev", "/", "shm");
        assert!(dir_is_tmpfs(Path::new(&dev_shm)));
        assert!(!dir_is_tmpfs(Path::new("/")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_profile_outside_temporary_directories_is_not_moved() {
        let previous = std::env::var("LLVM_PROFILE_FILE").ok();
        unsafe { std::env::set_var("LLVM_PROFILE_FILE", "/var/cov/profile.profraw") };
        assert!(hold_coverage_profile().is_none());
        match previous {
            Some(value) => unsafe { std::env::set_var("LLVM_PROFILE_FILE", value) },
            None => unsafe { std::env::remove_var("LLVM_PROFILE_FILE") },
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn conbin_fault_child() {
        match std::env::var("CONBIN_CASE") {
            Ok(case) => {
                assert_eq!(case, "fault");
                run_fault_suite();
                println!("CASE_OK fault");
            }
            Err(err) => assert_eq!(err, std::env::VarError::NotPresent),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn conbin_namespace_child() {
        match std::env::var("CONBIN_CASE") {
            Ok(case) => {
                assert_eq!(case, "namespace");
                let saved = std::env::var("LLVM_PROFILE_FILE").ok();
                let live = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/cov-live");
                std::fs::create_dir_all(&live).unwrap();
                std::fs::write(live.join("marker"), b"held").unwrap();
                let profile = live.join("profile.profraw");
                unsafe { std::env::set_var("LLVM_PROFILE_FILE", &profile) };
                let held = hold_coverage_profile().expect("a repo-local profile is held");
                assert!(held.0.starts_with(&live), "{}", held.0.display());
                assert_eq!(std::fs::read(held.1.join("marker")).unwrap(), b"held");
                let rc = unsafe { libc::umount(std::ffi::CString::new(held.1.to_str().unwrap()).unwrap().as_ptr()) };
                assert_eq!(rc, 0, "{}", std::io::Error::last_os_error());
                let _ = std::fs::remove_dir_all(&held.1);
                unsafe { std::env::remove_var("LLVM_PROFILE_FILE") };
                run_namespace_suite();
                let bare = live.join(format!("case-bare-{}", std::process::id()));
                let marked = live.join(format!("case-marked-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&bare);
                let _ = std::fs::remove_dir_all(&marked);
                std::fs::create_dir_all(&bare).unwrap();
                std::fs::create_dir_all(&marked).unwrap();
                std::fs::write(marked.join("marker"), b"held").unwrap();
                unsafe {
                    std::env::set_var("LLVM_PROFILE_FILE", bare.join("profile.profraw"));
                }
                run_namespace_suite();
                unsafe {
                    std::env::set_var("LLVM_PROFILE_FILE", marked.join("profile.profraw"));
                }
                run_namespace_suite();
                let _ = std::fs::remove_dir_all(&bare);
                let _ = std::fs::remove_dir_all(&marked);
                match saved {
                    Some(value) => unsafe { std::env::set_var("LLVM_PROFILE_FILE", value) },
                    None => unsafe { std::env::remove_var("LLVM_PROFILE_FILE") },
                }
                println!("CASE_OK namespace");
            }
            Err(err) => assert_eq!(err, std::env::VarError::NotPresent),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn conbin_stream_child() {
        match std::env::var("CONBIN_CASE") {
            Ok(case) => {
                assert_eq!(case, "stream");
                run_stream_suite();
                println!("CASE_OK stream");
            }
            Err(err) => assert_eq!(err, std::env::VarError::NotPresent),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn fault_injection_and_namespace_edges_run_in_child_processes() {
        spawn_case("conbin::tests::conbin_fault_child", "fault", false, true);
        spawn_case("conbin::tests::conbin_stream_child", "stream", false, false);
        spawn_case(
            "conbin::tests::conbin_namespace_child",
            "namespace",
            true,
            false,
        );
    }

    #[cfg(target_os = "linux")]
    const PRELOAD_C: &str = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

static int (*real_open64)(const char *, int, ...) = 0;
static ssize_t (*real_pread64)(int, void *, size_t, off_t) = 0;
static ssize_t (*real_pread)(int, void *, size_t, off_t) = 0;
static int (*real_madvise)(void *, size_t, int) = 0;
static ssize_t (*real_readahead)(int, off_t, size_t) = 0;
static off_t (*real_lseek64)(int, off_t, int) = 0;
static off_t (*real_lseek)(int, off_t, int) = 0;
static int (*real_statx)(int, const char *, int, unsigned int, struct statx *) = 0;
static long (*real_sysconf)(int) = 0;

static char active[80];
static int readahead_count;
static int madvise_calls;
static int marker_opens;
static int pread_eintr_done;

static void refresh(void) {
    if (!real_open64) {
        return;
    }
    char path[128];
    const char *dir = getenv("TMPDIR");
    if (!dir || !dir[0]) {
        dir = ".";
    }
    snprintf(path, sizeof path, "%s/conbin-active-%d", dir, (int)getpid());
    int fd = real_open64(path, O_RDONLY, 0);
    char buf[80];
    memset(buf, 0, sizeof buf);
    if (fd >= 0) {
        ssize_t n = read(fd, buf, sizeof buf - 1);
        close(fd);
        if (n > 0 && buf[n - 1] == '\n') {
            buf[n - 1] = 0;
        }
    }
    if (strcmp(buf, active) != 0) {
        memcpy(active, buf, sizeof active);
        readahead_count = 0;
        madvise_calls = 0;
        marker_opens = 0;
        pread_eintr_done = 0;
    }
}

static int marked(int fd, const char *path) {
    if (path && strstr(path, "conbin-statfail") != 0) {
        return 1;
    }
    if (fd < 0) {
        return 0;
    }
    char linkpath[64];
    char buf[4096];
    snprintf(linkpath, sizeof linkpath, "/proc/self/fd/%d", fd);
    ssize_t n = readlink(linkpath, buf, sizeof buf - 1);
    if (n < 0) {
        return 0;
    }
    buf[n] = 0;
    return strstr(buf, "conbin-statfail") != 0;
}

static void mark(const char *name) {
    char path[160];
    const char *dir = getenv("TMPDIR");
    if (!dir || !dir[0]) {
        dir = ".";
    }
    snprintf(path, sizeof path, "%s/conbin-mark-%d-%s", dir, (int)getpid(), name);
    if (!real_open64) {
        return;
    }
    int fd = real_open64(path, O_CREAT | O_WRONLY | O_TRUNC, 0644);
    if (fd >= 0) {
        ssize_t wrote = write(fd, "1", 1);
        if (wrote < 0) {
            wrote = 0;
        }
        close(fd);
    }
}

static int fail_reopen(const char *path) {
    if (!path || strstr(path, "conbin-reopen-fail") == 0) {
        return 0;
    }
    if (strcmp(active, "open-fail-seq") != 0 && strcmp(active, "open-fail-split") != 0) {
        return 0;
    }
    marker_opens++;
    if (marker_opens >= 2) {
        errno = EACCES;
        return 1;
    }
    return 0;
}

static void init(void) __attribute__((constructor));
static void init(void) {
    real_open64 = dlsym(RTLD_NEXT, "open64");
    real_pread64 = dlsym(RTLD_NEXT, "pread64");
    real_pread = dlsym(RTLD_NEXT, "pread");
    real_madvise = dlsym(RTLD_NEXT, "madvise");
    real_readahead = dlsym(RTLD_NEXT, "readahead");
    real_lseek64 = dlsym(RTLD_NEXT, "lseek64");
    real_lseek = dlsym(RTLD_NEXT, "lseek");
    real_statx = dlsym(RTLD_NEXT, "statx");
    real_sysconf = dlsym(RTLD_NEXT, "sysconf");
}

int open64(const char *path, int flags, ...) {
    unsigned int mode = 0;
    if (flags & O_CREAT) {
        va_list ap;
        va_start(ap, flags);
        mode = va_arg(ap, unsigned int);
        va_end(ap);
    }
    refresh();
    if (fail_reopen(path)) {
        return -1;
    }
    return real_open64(path, flags, mode);
}

static ssize_t pread_common(ssize_t (*real)(int, void *, size_t, off_t), int fd, void *buf, size_t count, off_t offset) {
    refresh();
    if (!pread_eintr_done && strcmp(active, "pread-eintr") == 0) {
        pread_eintr_done = 1;
        mark("pread-eintr");
        errno = EINTR;
        return -1;
    }
    return real(fd, buf, count, offset);
}

ssize_t pread64(int fd, void *buf, size_t count, off_t offset) {
    return pread_common(real_pread64, fd, buf, count, offset);
}

ssize_t pread(int fd, void *buf, size_t count, off_t offset) {
    return pread_common(real_pread, fd, buf, count, offset);
}

int madvise(void *addr, size_t len, int advice) {
    refresh();
    if (strcmp(active, "madvise-einval") == 0) {
        mark("madvise-einval");
        errno = EINVAL;
        return -1;
    }
    if (strcmp(active, "madvise-eio") == 0) {
        mark("madvise-eio");
        errno = EIO;
        return -1;
    }
    if (strcmp(active, "madvise-eagain-many") == 0) {
        mark("madvise-eagain-many");
        errno = EAGAIN;
        return -1;
    }
    if (strcmp(active, "madvise-eagain-once") == 0) {
        madvise_calls++;
        if (madvise_calls == 1) {
            mark("madvise-eagain-once");
            errno = EAGAIN;
            return -1;
        }
    }
    return real_madvise(addr, len, advice);
}

ssize_t readahead(int fd, off_t offset, size_t count) {
    refresh();
    readahead_count++;
    if (strcmp(active, "readahead-fail") == 0) {
        errno = EINVAL;
        return -1;
    }
    if (strcmp(active, "readahead-fail-2") == 0 && readahead_count >= 2) {
        errno = EINVAL;
        return -1;
    }
    return real_readahead(fd, offset, count);
}

static off_t fail_seek(int fd, off_t offset, int whence, off_t (*real)(int, off_t, int)) {
    refresh();
    if (strcmp(active, "lseek-fail") == 0) {
        errno = ESPIPE;
        return -1;
    }
    return real(fd, offset, whence);
}

off_t lseek64(int fd, off_t offset, int whence) {
    return fail_seek(fd, offset, whence, real_lseek64);
}

off_t lseek(int fd, off_t offset, int whence) {
    return fail_seek(fd, offset, whence, real_lseek);
}

int statx(int dirfd, const char *pathname, int flags, unsigned int mask, struct statx *buf) {
    refresh();
    if (marked(dirfd, pathname)) {
        errno = EIO;
        return -1;
    }
    return real_statx(dirfd, pathname, flags, mask, buf);
}

long sysconf(int name) {
    refresh();
    if (name == _SC_PAGESIZE && strcmp(active, "pagesize-fail") == 0) {
        return -1;
    }
    if (name == _SC_PAGESIZE && strcmp(active, "pagesize-odd") == 0) {
        return 3;
    }
    return real_sysconf(name);
}
"#;
}
