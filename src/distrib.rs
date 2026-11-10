//! Rank conventions shared by every distributed entry point.
//!
//! Frame ownership is a pure function of `(frame index, n_frames,
//! n_ranks)`: a contiguous block partition with the remainder going to
//! the lowest ranks. This part has no MPI dependency, so serial code and
//! tests can reason about the same partition the MPI paths use.
//!
//! With the `mpi` feature, [`Comm`] wraps a communicator handed in by
//! the caller. The library never calls `MPI_Init` or `MPI_Finalize` and
//! never falls back to the world communicator: every entry point takes
//! a communicator, either as a Rust [`Comm`] or as a Fortran `MPI_Fint`
//! handle over the C ABI (see `include/readcon-core-mpi.h`).

/// Half-open frame range `[start, end)` owned by `rank`.
///
/// Returns an empty range at `n_frames` when `rank >= n_ranks` or
/// `n_ranks == 0`.
pub fn block_range(n_frames: usize, n_ranks: usize, rank: usize) -> std::ops::Range<usize> {
    if n_ranks == 0 || rank >= n_ranks {
        return n_frames..n_frames;
    }
    let base = n_frames / n_ranks;
    let rem = n_frames % n_ranks;
    let start = rank * base + rank.min(rem);
    let len = base + usize::from(rank < rem);
    start..start + len
}

/// Rank that owns frame `i` under [`block_range`].
pub fn block_owner(n_frames: usize, n_ranks: usize, i: usize) -> Option<usize> {
    if n_ranks == 0 || i >= n_frames {
        return None;
    }
    let base = n_frames / n_ranks;
    let rem = n_frames % n_ranks;
    let big = rem * (base + 1);
    Some(if i < big {
        i / (base + 1)
    } else {
        rem + (i - big) / base
    })
}

/// C ABI for [`block_range`]; writes `[*start, *end)`. Returns 0, or -1
/// on a null pointer.
///
/// # Safety
/// `start` and `end` must be null or valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rkr_block_range(
    n_frames: u64,
    n_ranks: u32,
    rank: u32,
    start: *mut u64,
    end: *mut u64,
) -> i32 {
    if start.is_null() || end.is_null() {
        return -1;
    }
    let r = block_range(n_frames as usize, n_ranks as usize, rank as usize);
    unsafe {
        *start = r.start as u64;
        *end = r.end as u64;
    }
    0
}

#[cfg(feature = "mpi")]
pub use with_mpi::*;

#[cfg(feature = "mpi")]
mod with_mpi {
    use ::mpi::ffi;
    use ::mpi::raw::FromRaw;
    use ::mpi::topology::SimpleCommunicator;
    use ::mpi::traits::*;

    /// The caller's communicator, duplicated so library traffic never
    /// matches the caller's own messages. The duplicate is freed on drop.
    pub struct Comm {
        inner: SimpleCommunicator,
    }

    impl Comm {
        /// Duplicate any rsmpi communicator.
        pub fn dup_of<C: Communicator>(c: &C) -> Self {
            Comm {
                inner: c.duplicate(),
            }
        }

        /// Duplicate a raw `MPI_Comm` owned by the caller.
        ///
        /// # Safety
        /// MPI must be initialized and `raw` must be a live intra-communicator.
        pub unsafe fn dup_raw(raw: ffi::MPI_Comm) -> Result<Self, i32> {
            let mut out: ffi::MPI_Comm = unsafe { std::mem::zeroed() };
            let rc = unsafe { ffi::MPI_Comm_dup(raw, &mut out) };
            if rc != 0 {
                return Err(rc);
            }
            Ok(Comm {
                inner: unsafe { SimpleCommunicator::from_raw(out) },
            })
        }

        /// Duplicate a Fortran `MPI_Fint` communicator handle.
        ///
        /// # Safety
        /// MPI must be initialized and `f` must name a live intra-communicator.
        pub unsafe fn dup_fint(f: ffi::RSMPI_Fint) -> Result<Self, i32> {
            unsafe { Self::dup_raw(ffi::RSMPI_Comm_f2c(f)) }
        }

        pub fn rank(&self) -> usize {
            self.inner.rank() as usize
        }

        pub fn size(&self) -> usize {
            self.inner.size() as usize
        }

        /// The wrapped communicator.
        pub fn raw(&self) -> &SimpleCommunicator {
            &self.inner
        }

        /// This rank's frames under [`super::block_range`].
        pub fn my_frames(&self, n_frames: usize) -> std::ops::Range<usize> {
            super::block_range(n_frames, self.size(), self.rank())
        }

        /// Broadcast a byte blob from `root`. `data` is read on `root` and
        /// ignored elsewhere; every rank returns the root's bytes.
        pub fn bcast_bytes(&self, root: usize, data: &[u8]) -> Vec<u8> {
            let r = self.inner.process_at_rank(root as i32);
            let mut n: u64 = if self.rank() == root {
                data.len() as u64
            } else {
                0
            };
            r.broadcast_into(&mut n);
            let mut buf = if self.rank() == root {
                data.to_vec()
            } else {
                vec![0u8; n as usize]
            };
            if n > 0 {
                r.broadcast_into(&mut buf[..]);
            }
            buf
        }
    }

    /// Size and rank of a Fortran communicator; returns 0 or the MPI error.
    ///
    /// # Safety
    /// MPI must be initialized; `size` and `rank` must be valid for writes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn rkr_mpi_comm_info_f(
        comm_f: ffi::RSMPI_Fint,
        size: *mut i32,
        rank: *mut i32,
    ) -> i32 {
        if size.is_null() || rank.is_null() {
            return -1;
        }
        match unsafe { Comm::dup_fint(comm_f) } {
            Ok(c) => {
                unsafe {
                    *size = c.size() as i32;
                    *rank = c.rank() as i32;
                }
                0
            }
            Err(e) => e,
        }
    }
}

/// One `MPI_Init` for the process. Dropping a `Universe` finalizes MPI, so a
/// second test cannot call `initialize` again. The lock lives until exit.
#[cfg(all(test, feature = "mpi"))]
pub(crate) fn test_universe() -> &'static ::mpi::environment::Universe {
    static UNIVERSE: std::sync::OnceLock<::mpi::environment::Universe> = std::sync::OnceLock::new();
    UNIVERSE.get_or_init(|| ::mpi::initialize().expect("MPI_Init"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_partition_covers_and_matches_owner() {
        for n in 0..40 {
            for p in 1..9 {
                let mut next = 0;
                for r in 0..p {
                    let range = block_range(n, p, r);
                    assert_eq!(range.start, next);
                    for i in range.clone() {
                        assert_eq!(block_owner(n, p, i), Some(r));
                    }
                    next = range.end;
                }
                assert_eq!(next, n);
                assert!(block_range(n, p, p).is_empty());
                assert_eq!(block_owner(n, p, n), None);
            }
        }
        assert!(block_range(10, 0, 0).is_empty());
        assert_eq!(block_owner(10, 0, 0), None);
        assert_eq!(block_owner(0, 1, 0), None);
    }

    #[test]
    fn block_range_export_rejects_a_null_and_writes_the_span() {
        let mut start = 9u64;
        let mut end = 9u64;
        let rc = unsafe { rkr_block_range(10, 0, 0, std::ptr::null_mut(), &mut end) };
        assert_eq!(rc, -1);
        assert_eq!(end, 9);
        let rc = unsafe { rkr_block_range(10, 4, 1, &mut start, std::ptr::null_mut()) };
        assert_eq!(rc, -1);
        assert_eq!(start, 9);
        let rc = unsafe { rkr_block_range(10, 4, 1, &mut start, &mut end) };
        assert_eq!(rc, 0);
        assert_eq!(start, 3);
        assert_eq!(end, 6);
        assert_eq!(block_owner(10, 4, 3), Some(1));
        assert_eq!(block_owner(10, 4, 5), Some(1));
    }

    #[cfg(feature = "mpi")]
    #[test]
    fn a_duplicated_world_keeps_its_rank_and_broadcasts() {
        use ::mpi::raw::AsRaw;
        let universe = super::test_universe();
        let world = universe.world();
        let comm = with_mpi::Comm::dup_of(&world);
        assert!(comm.size() >= 1);
        assert!(comm.rank() < comm.size());
        let mine = comm.my_frames(10);
        assert_eq!(mine.start, block_range(10, comm.size(), comm.rank()).start);
        let blob = comm.bcast_bytes(0, b"frame");
        assert_eq!(blob, b"frame");
        let empty = comm.bcast_bytes(0, b"");
        assert!(empty.is_empty());
        unsafe {
            ::mpi::ffi::MPI_Comm_set_errhandler(
                ::mpi::ffi::RSMPI_COMM_WORLD,
                ::mpi::ffi::RSMPI_ERRORS_RETURN,
            );
        }
        let mut size = -1i32;
        let mut rank = -1i32;
        let rc = unsafe {
            with_mpi::rkr_mpi_comm_info_f(
                ::mpi::ffi::RSMPI_Comm_c2f(::mpi::ffi::RSMPI_COMM_NULL),
                &mut size,
                &mut rank,
            )
        };
        unsafe {
            ::mpi::ffi::MPI_Comm_set_errhandler(
                ::mpi::ffi::RSMPI_COMM_WORLD,
                ::mpi::ffi::RSMPI_ERRORS_ARE_FATAL,
            );
        }
        assert_ne!(rc, 0, "a null communicator must not report a rank");
        let mut size = -1i32;
        let mut rank = -1i32;
        let raw = world.as_raw();
        let rc = unsafe {
            with_mpi::rkr_mpi_comm_info_f(
                ::mpi::ffi::RSMPI_Comm_c2f(raw),
                std::ptr::null_mut(),
                &mut rank,
            )
        };
        assert_eq!(rc, -1);
        let rc = unsafe {
            with_mpi::rkr_mpi_comm_info_f(
                ::mpi::ffi::RSMPI_Comm_c2f(raw),
                &mut size,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc, -1);
        unsafe {
            ::mpi::ffi::MPI_Comm_set_errhandler(
                ::mpi::ffi::RSMPI_COMM_WORLD,
                ::mpi::ffi::RSMPI_ERRORS_RETURN,
            );
        }
        let rejected = unsafe { with_mpi::Comm::dup_raw(::mpi::ffi::RSMPI_COMM_NULL) };
        unsafe {
            ::mpi::ffi::MPI_Comm_set_errhandler(
                ::mpi::ffi::RSMPI_COMM_WORLD,
                ::mpi::ffi::RSMPI_ERRORS_ARE_FATAL,
            );
        }
        assert!(rejected.is_err(), "duplicating MPI_COMM_NULL must fail");
        let rc = unsafe {
            with_mpi::rkr_mpi_comm_info_f(
                ::mpi::ffi::RSMPI_Comm_c2f(raw),
                &mut size,
                &mut rank,
            )
        };
        assert_eq!(rc, 0);
        assert_eq!(size as usize, comm.size());
        assert_eq!(rank as usize, comm.rank());
    }
}
