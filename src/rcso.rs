//! RCSO cooked SoA bytes for a caller-side `MPI_Bcast`.
//!
//! Layout matches readcon-db `cooked_soa` v1 (little-endian):
//! magic `RCSO`, version, natoms, flags, dtype, pad, reserved,
//! then positions, optional forces, optional velocities.
//!
//! This crate never calls `MPI_Init` or names WORLD. Rank 0 encodes;
//! the caller broadcasts; workers decode.

use crate::error::ParseError;
use crate::types::ConFrame;

/// Four-byte magic. Identical to readcon-db.
pub const RCSO_MAGIC: &[u8; 4] = b"RCSO";
/// Layout version 1.
pub const RCSO_VERSION: u32 = 1;
/// Positions/forces/velocities stored as f64.
pub const RCSO_DTYPE_F64: u8 = 0;
/// Bit 0: forces block present.
pub const RCSO_FLAG_FORCES: u32 = 1 << 0;
/// Bit 1: velocities block present.
pub const RCSO_FLAG_VELOCITIES: u32 = 1 << 1;

const HEADER_LEN: usize = 24;

/// Length-prefixed RCSO blobs for one collective (ADIOS BP5 grain: many frames).
pub const RCSB_MAGIC: &[u8; 4] = b"RCSB";
/// Batch envelope version 1.
pub const RCSB_VERSION: u32 = 1;

/// Decoded cooked numerics.
#[derive(Clone, Debug, PartialEq)]
pub struct Rcso {
    pub natoms: u32,
    pub positions: Vec<[f64; 3]>,
    pub forces: Option<Vec<[f64; 3]>>,
    pub velocities: Option<Vec<[f64; 3]>>,
}

fn err(msg: impl Into<String>) -> ParseError {
    ParseError::ValidationError(msg.into())
}

fn fit_u32(len: usize, msg: &str) -> Result<u32, ParseError> {
    u32::try_from(len).map_err(|_| err(msg))
}

fn mul_size(a: usize, b: usize) -> Result<usize, ParseError> {
    a.checked_mul(b).ok_or_else(|| err("RCSO overflow"))
}

fn add_size(a: usize, b: usize) -> Result<usize, ParseError> {
    a.checked_add(b).ok_or_else(|| err("RCSO overflow"))
}

impl Rcso {
    /// Pack one parsed frame. CON text stays the authority.
    pub fn encode_frame(frame: &ConFrame) -> Result<Vec<u8>, ParseError> {
        let n = frame.atom_data.len();
        let natoms = fit_u32(n, "too many atoms for RCSO")?;
        let has_f = frame.atom_data.iter().any(|a| a.force().is_some());
        let has_v = frame.atom_data.iter().any(|a| a.velocity().is_some());
        let mut flags = 0u32;
        if has_f {
            flags |= RCSO_FLAG_FORCES;
        }
        if has_v {
            flags |= RCSO_FLAG_VELOCITIES;
        }

        let mut out =
            Vec::with_capacity(HEADER_LEN + n * 3 * 8 * (1 + has_f as usize + has_v as usize));
        out.extend_from_slice(RCSO_MAGIC);
        out.extend_from_slice(&RCSO_VERSION.to_le_bytes());
        out.extend_from_slice(&natoms.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.push(RCSO_DTYPE_F64);
        out.extend_from_slice(&[0u8; 3]);
        out.extend_from_slice(&0u32.to_le_bytes());

        for a in &frame.atom_data {
            for c in [a.x, a.y, a.z] {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        if has_f {
            for a in &frame.atom_data {
                let f = a.force().unwrap_or([0.0; 3]);
                for c in f {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        if has_v {
            for a in &frame.atom_data {
                let v = a.velocity().unwrap_or([0.0; 3]);
                for c in v {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        Ok(out)
    }

    /// Decode a v1 RCSO blob.
    pub fn decode(bytes: &[u8]) -> Result<Self, ParseError> {
        if bytes.len() < HEADER_LEN {
            return Err(err("RCSO truncated header"));
        }
        if &bytes[0..4] != RCSO_MAGIC {
            return Err(err("RCSO bad magic"));
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version != RCSO_VERSION {
            return Err(err(format!("RCSO unsupported version {version}")));
        }
        let natoms = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let flags = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let dtype = bytes[16];
        if dtype != RCSO_DTYPE_F64 {
            return Err(err(format!("RCSO unsupported dtype {dtype}")));
        }
        let n = natoms as usize;
        let block = mul_size(n, 3)?;
        let block_bytes = mul_size(block, 8)?;
        let mut need = add_size(HEADER_LEN, block_bytes)?;
        let has_f = flags & RCSO_FLAG_FORCES != 0;
        let has_v = flags & RCSO_FLAG_VELOCITIES != 0;
        if has_f {
            need = add_size(need, block_bytes)?;
        }
        if has_v {
            need = add_size(need, block_bytes)?;
        }
        if bytes.len() < need {
            return Err(err("RCSO truncated body"));
        }

        let mut off = HEADER_LEN;
        let positions = read_vec3_block(&bytes[off..off + block_bytes], n)?;
        off += block_bytes;
        let forces = if has_f {
            let f = read_vec3_block(&bytes[off..off + block_bytes], n)?;
            off += block_bytes;
            Some(f)
        } else {
            None
        };
        let velocities = if has_v {
            Some(read_vec3_block(&bytes[off..off + block_bytes], n)?)
        } else {
            None
        };
        Ok(Self {
            natoms,
            positions,
            forces,
            velocities,
        })
    }
}

fn read_vec3_block(bytes: &[u8], n: usize) -> Result<Vec<[f64; 3]>, ParseError> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 24;
        let x = f64::from_le_bytes(bytes[base..base + 8].try_into().unwrap());
        let y = f64::from_le_bytes(bytes[base + 8..base + 16].try_into().unwrap());
        let z = f64::from_le_bytes(bytes[base + 16..base + 24].try_into().unwrap());
        out.push([x, y, z]);
    }
    Ok(out)
}

/// Pack many RCSO blobs into one RCSB envelope (one Bcast).
pub fn encode_batch(blobs: &[Vec<u8>]) -> Result<Vec<u8>, ParseError> {
    let count = fit_u32(blobs.len(), "too many frames in RCSB batch")?;
    let mut out = Vec::new();
    out.extend_from_slice(RCSB_MAGIC);
    out.extend_from_slice(&RCSB_VERSION.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    for b in blobs {
        let ln = fit_u32(b.len(), "RCSO blob exceeds u32 length")?;
        out.extend_from_slice(&ln.to_le_bytes());
        out.extend_from_slice(b);
    }
    Ok(out)
}

/// Split an RCSB envelope into RCSO blobs.
pub fn decode_batch(bytes: &[u8]) -> Result<Vec<Vec<u8>>, ParseError> {
    if bytes.len() < 12 {
        return Err(err("RCSB truncated header"));
    }
    if &bytes[0..4] != RCSB_MAGIC {
        return Err(err("RCSB bad magic"));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != RCSB_VERSION {
        return Err(err(format!("RCSB unsupported version {version}")));
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut off = 12usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if off + 4 > bytes.len() {
            return Err(err("RCSB truncated length"));
        }
        let ln = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        if off + ln > bytes.len() {
            return Err(err("RCSB truncated blob"));
        }
        out.push(bytes[off..off + ln].to_vec());
        off += ln;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterators::ConFrameIterator;

    fn first_frame(text: &str) -> ConFrame {
        ConFrameIterator::new(text)
            .next()
            .expect("frame")
            .expect("parse")
    }

    #[test]
    fn v2_minimal_roundtrip_positions() {
        let text = include_str!("../resources/conformance/valid/v2_minimal.con");
        let frame = first_frame(text);
        let blob = Rcso::encode_frame(&frame).unwrap();
        assert_eq!(&blob[0..4], b"RCSO");
        assert_eq!(u32::from_le_bytes(blob[4..8].try_into().unwrap()), 1);
        let got = Rcso::decode(&blob).unwrap();
        assert_eq!(got.natoms as usize, frame.atom_data.len());
        assert!(got.forces.is_none());
        assert!(got.velocities.is_none());
        for (a, p) in frame.atom_data.iter().zip(&got.positions) {
            assert_eq!([a.x, a.y, a.z], *p);
        }
    }

    #[test]
    fn forces_fixture_keeps_force_block() {
        let text = include_str!("../resources/test/tiny_cuh2_forces.con");
        let frame = first_frame(text);
        assert!(frame.atom_data.iter().any(|a| a.force().is_some()));
        let blob = Rcso::encode_frame(&frame).unwrap();
        let flags = u32::from_le_bytes(blob[12..16].try_into().unwrap());
        assert_ne!(flags & RCSO_FLAG_FORCES, 0);
        let got = Rcso::decode(&blob).unwrap();
        let forces = got.forces.expect("forces");
        for (a, f) in frame.atom_data.iter().zip(&forces) {
            assert_eq!(a.force().unwrap_or([0.0; 3]), *f);
        }
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(Rcso::decode(b"NOPE").is_err());
    }

    #[test]
    fn rejects_truncated_and_bad_fields() {
        assert!(Rcso::decode(&[0u8; 8]).is_err());
        let frame = first_frame(include_str!(
            "../resources/conformance/valid/v2_minimal.con"
        ));
        let good = Rcso::encode_frame(&frame).unwrap();
        let mut ver = good.clone();
        ver[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert!(Rcso::decode(&ver).is_err());
        let mut dt = good.clone();
        dt[16] = 1;
        assert!(Rcso::decode(&dt).is_err());
        assert!(Rcso::decode(&good[..20]).is_err());
        assert!(decode_batch(b"xxxx").is_err());
        assert!(decode_batch(b"RCSB\x02\x00\x00\x00\x00\x00\x00\x00").is_err());
    }

    #[test]
    fn velocities_fixture_keeps_velocity_block() {
        let text = include_str!("../resources/test/tiny_cuh2.convel");
        let frame = first_frame(text);
        assert!(frame.atom_data.iter().any(|a| a.velocity().is_some()));
        let blob = Rcso::encode_frame(&frame).unwrap();
        let flags = u32::from_le_bytes(blob[12..16].try_into().unwrap());
        assert_ne!(flags & RCSO_FLAG_VELOCITIES, 0);
        let got = Rcso::decode(&blob).unwrap();
        let vels = got.velocities.expect("velocities");
        for (a, v) in frame.atom_data.iter().zip(&vels) {
            assert_eq!(a.velocity().unwrap_or([0.0; 3]), *v);
        }
    }

    #[test]
    fn rcsb_batch_holds_two_blobs() {
        let text = include_str!("../resources/conformance/valid/v2_minimal.con");
        let frame = first_frame(text);
        let a = Rcso::encode_frame(&frame).unwrap();
        let b = a.clone();
        let batch = encode_batch(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(&batch[0..4], b"RCSB");
        let parts = decode_batch(&batch).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], a);
        assert_eq!(parts[1], b);
    }

    #[test]
    fn length_helpers_and_truncated_batches_fail_closed() {
        let too_many = fit_u32(u32::MAX as usize + 1, "too many atoms for RCSO").unwrap_err();
        assert!(too_many.to_string().contains("too many atoms"), "{too_many}");
        assert_eq!(fit_u32(4, "too many atoms for RCSO").unwrap(), 4);
        let overflow = mul_size(usize::MAX, 2).unwrap_err();
        assert!(overflow.to_string().contains("overflow"), "{overflow}");
        assert_eq!(mul_size(3, 8).unwrap(), 24);
        let wrapped = add_size(usize::MAX, 1).unwrap_err();
        assert!(wrapped.to_string().contains("overflow"), "{wrapped}");
        assert_eq!(add_size(24, 8).unwrap(), 32);

        let mut bad_magic = vec![0u8; 24];
        bad_magic[..4].copy_from_slice(b"NOPE");
        let err = Rcso::decode(&bad_magic).unwrap_err();
        assert!(err.to_string().contains("bad magic"), "{err}");
        let mut short_body = bad_magic.clone();
        short_body[0..4].copy_from_slice(RCSO_MAGIC);
        short_body[4..8].copy_from_slice(&RCSO_VERSION.to_le_bytes());
        short_body[8..12].copy_from_slice(&1u32.to_le_bytes());
        let err = Rcso::decode(&short_body).unwrap_err();
        assert!(err.to_string().contains("truncated body"), "{err}");

        let mut batch = encode_batch(&[vec![1, 2, 3]]).unwrap();
        batch[0] = b'X';
        let err = decode_batch(&batch).unwrap_err();
        assert!(err.to_string().contains("bad magic"), "{err}");

        let truncated_len = decode_batch(b"RCSB\x01\x00\x00\x00\x01\x00\x00\x00").unwrap_err();
        assert!(truncated_len.to_string().contains("truncated length"), "{truncated_len}");
        let truncated_blob =
            decode_batch(b"RCSB\x01\x00\x00\x00\x01\x00\x00\x00\x05\x00\x00\x00ab").unwrap_err();
        assert!(truncated_blob.to_string().contains("truncated blob"), "{truncated_blob}");

        let frame = first_frame(include_str!("../resources/test/tiny_cuh2.con"));
        let mut with_force = frame.clone();
        with_force.atom_data[0].set_force(Some([1.0, 0.0, 0.0]));
        let blob = Rcso::encode_frame(&with_force).unwrap();
        let got = Rcso::decode(&blob).unwrap();
        assert_eq!(got.forces.unwrap()[0], [1.0, 0.0, 0.0]);
        for atom in with_force.atom_data.iter_mut().skip(1) {
            atom.set_force(None);
        }
        let partial = Rcso::encode_frame(&with_force).unwrap();
        let decoded = Rcso::decode(&partial).unwrap();
        assert_eq!(decoded.forces.unwrap()[1], [0.0, 0.0, 0.0]);
    }
}
