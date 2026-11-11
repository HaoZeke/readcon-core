// Copyright (c) readcon developers
//! Chemfiles → CON conversion.
//!
//! Real implementation requires the `chemfiles` Cargo feature (links libchemfiles).
//! Without it, path/memory helpers are still present and return
//! [`ChemfilesImportError::FeatureDisabled`] so call sites compile uniformly.
//!
//! After a successful read, numbers are in chemfiles internal units
//! (Å, Å/ps, amu, degrees). Import stamps those onto CON line-2 `units`
//! (`time` is `ps`, not the CON-native default `fs`).

use std::path::PathBuf;

/// Options for a chemfiles trajectory read (`read_step`, topology file, stride).
#[derive(Clone, Debug)]
pub struct ChemfilesReadOpts {
    /// First step to keep (inclusive, 0-based). Maps to `Trajectory::read_step`.
    pub start: usize,
    /// Stride between kept steps. Must be `>= 1`.
    pub step: usize,
    /// Exclusive end step. `None` means `nsteps`.
    pub stop: Option<usize>,
    /// Force a chemfiles format name (`"XYZ"`, `"PDB"`, `"GRO"`, …).
    /// Empty / `None` lets chemfiles guess from the path.
    pub format: Option<String>,
    /// Optional topology file (`Trajectory::set_topology_file`).
    pub topology: Option<PathBuf>,
    /// Format for [`Self::topology`] (`set_topology_with_format`). Empty = guess.
    pub topology_format: Option<String>,
    /// Call chemfiles `guess_bonds` when the frame has no topology bonds.
    pub guess_bonds: bool,
}

impl Default for ChemfilesReadOpts {
    fn default() -> Self {
        Self {
            start: 0,
            step: 1,
            stop: None,
            format: None,
            topology: None,
            topology_format: None,
            guess_bonds: false,
        }
    }
}

impl ChemfilesReadOpts {
    /// Effective stride. Rejects `step == 0`.
    pub fn stride(&self) -> Result<usize, String> {
        if self.step == 0 {
            Err("chemfiles read stride must be >= 1".into())
        } else {
            Ok(self.step)
        }
    }
}

/// Chemfiles internal unit system after a successful read
/// (<https://chemfiles.org/chemfiles/latest/overview.html#units>).
///
/// Positions and cell lengths are Ångström, velocities are Å/ps, masses
/// are amu. Energy is the CON v3 required key; chemfiles does not convert
/// energies, so this object uses the CON default `eV`.
pub fn chemfiles_internal_units_json() -> serde_json::Value {
    serde_json::json!({
        "length": "angstrom",
        "energy": "eV",
        "mass": "amu",
        "time": "ps"
    })
}

#[cfg(feature = "chemfiles")]
#[path = "chemfiles_import_imp.rs"]
mod imp;

#[cfg(feature = "chemfiles")]
pub use imp::*;

#[cfg(not(feature = "chemfiles"))]
mod stubs {
    use std::fmt;
    use std::path::Path;

    use crate::types::ConFrame;

    /// Prefix for unmapped chemfiles frame properties in CON metadata.
    pub const CHEMFILES_EXTRA_PREFIX: &str = "chemfiles::";
    /// Per-atom property bag key in frame metadata.
    pub const CHEMFILES_ATOM_PROPS_KEY: &str = "chemfiles_atom_properties";
    /// Display names in chemfiles / `atom_id` order.
    pub const CHEMFILES_ATOM_NAMES_KEY: &str = "chemfiles_atom_names";
    /// Atomic types in chemfiles / `atom_id` order.
    pub const CHEMFILES_ATOM_TYPES_KEY: &str = "chemfiles_atom_types";
    /// Residue list (`name`, optional `id`, remapped `atoms`) from chemfiles topology.
    pub const CHEMFILES_RESIDUES_KEY: &str = "chemfiles_residues";
    /// Provenance object for chemfiles internal units (length/velocity/angle/mass).
    pub const CHEMFILES_UNIT_SYSTEM_KEY: &str = "chemfiles::unit_system";

    /// Errors from chemfiles I/O or conversion (or missing feature).
    #[derive(Debug)]
    pub enum ChemfilesImportError {
        /// Atom / property count mismatch or other structural problem.
        InvalidFrame(String),
        /// I/O while reading a trajectory path.
        Io(std::io::Error),
        /// This build was compiled without the `chemfiles` Cargo feature.
        FeatureDisabled,
    }

    impl fmt::Display for ChemfilesImportError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ChemfilesImportError::InvalidFrame(msg) => {
                    write!(f, "invalid chemfiles frame: {msg}")
                }
                ChemfilesImportError::Io(e) => write!(f, "I/O error: {e}"),
                ChemfilesImportError::FeatureDisabled => write!(
                    f,
                    "chemfiles support is not enabled in this build; rebuild with `--features chemfiles` \
(Python: `maturin develop --features python,chemfiles` or install the `chemfiles` extra from source — see docs)"
                ),
            }
        }
    }

    impl std::error::Error for ChemfilesImportError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ChemfilesImportError::Io(e) => Some(e),
                ChemfilesImportError::InvalidFrame(_) | ChemfilesImportError::FeatureDisabled => {
                    None
                }
            }
        }
    }

    impl From<std::io::Error> for ChemfilesImportError {
        fn from(e: std::io::Error) -> Self {
            ChemfilesImportError::Io(e)
        }
    }

    fn disabled<T>() -> Result<T, ChemfilesImportError> {
        Err(ChemfilesImportError::FeatureDisabled)
    }

    /// Open a trajectory with chemfiles and convert every step to [`ConFrame`].
    ///
    /// Stub without the `chemfiles` feature — always returns [`ChemfilesImportError::FeatureDisabled`].
    pub fn con_frames_from_trajectory_path<P: AsRef<Path>>(
        _path: P,
    ) -> Result<Vec<ConFrame>, ChemfilesImportError> {
        disabled()
    }

    /// Read the first frame from a trajectory path.
    ///
    /// Stub without the `chemfiles` feature — always returns [`ChemfilesImportError::FeatureDisabled`].
    pub fn con_frame_from_trajectory_path<P: AsRef<Path>>(
        _path: P,
    ) -> Result<ConFrame, ChemfilesImportError> {
        disabled()
    }

    /// Read a trajectory from an in-memory buffer (chemfiles memory reader).
    ///
    /// Stub without the `chemfiles` feature — always returns [`ChemfilesImportError::FeatureDisabled`].
    pub fn con_frames_from_memory(
        _data: &str,
        _format: &str,
    ) -> Result<Vec<ConFrame>, ChemfilesImportError> {
        disabled()
    }

    /// Same as [`con_frames_from_trajectory_path`] with skip / stride / topology.
    pub fn con_frames_from_trajectory_path_with<P: AsRef<std::path::Path>>(
        _path: P,
        _opts: &super::ChemfilesReadOpts,
    ) -> Result<Vec<ConFrame>, ChemfilesImportError> {
        disabled()
    }

    /// Same as [`con_frames_from_memory`] with skip / stride / `guess_bonds`.
    pub fn con_frames_from_memory_with(
        _data: &str,
        _format: &str,
        _opts: &super::ChemfilesReadOpts,
    ) -> Result<Vec<ConFrame>, ChemfilesImportError> {
        disabled()
    }

    /// Read step `index` via chemfiles `Trajectory::read_step`.
    pub fn con_frame_from_trajectory_path_nth<P: AsRef<std::path::Path>>(
        _path: P,
        _index: usize,
    ) -> Result<ConFrame, ChemfilesImportError> {
        disabled()
    }

    /// Number of steps in a chemfiles trajectory (`Trajectory::nsteps`).
    pub fn nsteps_from_trajectory_path<P: AsRef<std::path::Path>>(
        _path: P,
    ) -> Result<usize, ChemfilesImportError> {
        disabled()
    }

    /// Whether this build linked libchemfiles and implements import/selection.
    pub const fn chemfiles_enabled() -> bool {
        false
    }
}

#[cfg(not(feature = "chemfiles"))]
pub use stubs::*;

#[cfg(feature = "chemfiles")]
/// Whether this build linked libchemfiles and implements import/selection.
pub const fn chemfiles_enabled() -> bool {
    true
}

#[cfg(test)]
mod stub_tests {
    use super::*;

    #[test]
    fn chemfiles_enabled_matches_feature() {
        assert_eq!(chemfiles_enabled(), cfg!(feature = "chemfiles"));
    }

    #[cfg(not(feature = "chemfiles"))]
    #[test]
    fn trajectory_path_stub_is_feature_disabled() {
        let err = con_frame_from_trajectory_path("nope.xyz").unwrap_err();
        assert!(matches!(err, ChemfilesImportError::FeatureDisabled));
        let msg = err.to_string();
        assert!(msg.contains("chemfiles"), "{msg}");
        let err = nsteps_from_trajectory_path("nope.xyz").unwrap_err();
        assert!(matches!(err, ChemfilesImportError::FeatureDisabled));
        let err = con_frame_from_trajectory_path_nth("nope.xyz", 1).unwrap_err();
        assert!(matches!(err, ChemfilesImportError::FeatureDisabled));
    }

    #[test]
    fn read_options_reject_a_zero_stride_and_keep_the_defaults() {
        let opts = ChemfilesReadOpts::default();
        assert_eq!(opts.start, 0);
        assert_eq!(opts.step, 1);
        assert_eq!(opts.stop, None);
        assert_eq!(opts.format, None);
        assert_eq!(opts.topology, None);
        assert_eq!(opts.topology_format, None);
        assert!(!opts.guess_bonds);
        assert_eq!(opts.stride().unwrap(), 1);
        let zero = ChemfilesReadOpts {
            step: 0,
            ..ChemfilesReadOpts::default()
        };
        let err = zero.stride().unwrap_err();
        assert!(err.contains("stride"), "{err}");
    }

    #[cfg(not(feature = "chemfiles"))]
    #[test]
    fn each_stub_reports_the_disabled_feature() {
        use std::io::{Error, ErrorKind};

        let frames = con_frames_from_trajectory_path("nope.xyz").unwrap_err();
        assert!(matches!(frames, ChemfilesImportError::FeatureDisabled));
        let mem = con_frames_from_memory("3\n\nO 0 0 0\n", "XYZ").unwrap_err();
        assert!(matches!(mem, ChemfilesImportError::FeatureDisabled));
        let opts = ChemfilesReadOpts::default();
        let with = con_frames_from_trajectory_path_with("nope.xyz", &opts).unwrap_err();
        assert!(matches!(with, ChemfilesImportError::FeatureDisabled));
        let mem_with = con_frames_from_memory_with("x", "XYZ", &opts).unwrap_err();
        assert!(matches!(mem_with, ChemfilesImportError::FeatureDisabled));

        let invalid = ChemfilesImportError::InvalidFrame("short".into());
        assert!(invalid.to_string().contains("short"));
        assert!(std::error::Error::source(&invalid).is_none());
        let io_err = ChemfilesImportError::from(Error::new(ErrorKind::NotFound, "gone"));
        assert!(matches!(io_err, ChemfilesImportError::Io(_)));
        assert!(io_err.to_string().contains("gone"));
        let src = std::error::Error::source(&io_err).unwrap();
        assert!(src.to_string().contains("gone"));
        assert!(std::error::Error::source(&io_err).is_some());
        assert!(
            std::error::Error::source(&ChemfilesImportError::FeatureDisabled).is_none()
        );
        assert_eq!(CHEMFILES_EXTRA_PREFIX, "chemfiles::");
        assert!(!CHEMFILES_ATOM_PROPS_KEY.is_empty());
        assert!(!CHEMFILES_ATOM_NAMES_KEY.is_empty());
        assert!(!CHEMFILES_ATOM_TYPES_KEY.is_empty());
        assert!(!CHEMFILES_RESIDUES_KEY.is_empty());
        assert!(!CHEMFILES_UNIT_SYSTEM_KEY.is_empty());
    }

    #[test]
    fn chemfiles_internal_units_are_angstrom_ps() {
        let u = chemfiles_internal_units_json();
        assert_eq!(u["length"], "angstrom");
        assert_eq!(u["time"], "ps");
        assert_eq!(u["mass"], "amu");
        assert_eq!(u["energy"], "eV");
    }
}
