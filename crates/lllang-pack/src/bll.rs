//! BLL v2 portable bitcode container encode/decode.

use crate::target::{BLL_ABI_EPOCH, BLL_FORMAT_VERSION};
use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

pub const MAGIC: &[u8; 4] = b"BLL\x02";
pub const FLAG_PORTABLE_BITCODE: u32 = 0x1;

/// Build a BLL v2 container from raw LLVM bitcode bytes.
pub fn encode_portable_bll(package_name: &str, version: &str, bitcode: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + bitcode.len() + package_name.len() + version.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&BLL_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&BLL_ABI_EPOCH.to_le_bytes());
    out.extend_from_slice(&FLAG_PORTABLE_BITCODE.to_le_bytes());
    write_str(&mut out, package_name);
    write_str(&mut out, version);
    out.extend_from_slice(&(bitcode.len() as u64).to_le_bytes());
    out.extend_from_slice(bitcode);
    let hash = Sha256::digest(&out);
    out.extend_from_slice(&hash);
    out
}

/// Parsed BLL v2 payload.
pub struct PortableBll {
    pub package_name: String,
    pub version: String,
    pub abi_epoch: u16,
    pub bitcode: Vec<u8>,
}

/// Detect and parse BLL v2; reject v1 / raw native objects.
pub fn decode_portable_bll(bytes: &[u8]) -> Result<PortableBll> {
    if bytes.len() < 4 {
        bail!("`.bll` too short; rebuild with `lang pack` (BLL v2 portable)");
    }
    if &bytes[0..4] == b"BLL\x01" {
        bail!("`.bll` is BLL v1 (native-per-triple); rebuild with `lang pack` (BLL v2 portable)");
    }
    if &bytes[0..4] != MAGIC {
        bail!(
            "`.bll` is not a portable BLL v2 container (missing magic BLL\\x02); \
             rebuild with `lang pack` (BLL v2 portable)"
        );
    }
    let mut i = 4;
    let fmt = read_u16(bytes, &mut i)?;
    if fmt != BLL_FORMAT_VERSION {
        bail!("unsupported BLL container version {fmt} (expected {BLL_FORMAT_VERSION})");
    }
    let abi_epoch = read_u16(bytes, &mut i)?;
    let flags = read_u32(bytes, &mut i)?;
    if flags & FLAG_PORTABLE_BITCODE == 0 {
        bail!("`.bll` missing PORTABLE_BITCODE flag; rebuild with `lang pack`");
    }
    let package_name = read_str(bytes, &mut i)?;
    let version = read_str(bytes, &mut i)?;
    let bc_len = read_u64(bytes, &mut i)? as usize;
    if i + bc_len + 32 > bytes.len() {
        bail!("`.bll` truncated bitcode section");
    }
    let bitcode = bytes[i..i + bc_len].to_vec();
    i += bc_len;
    let expected = &bytes[i..i + 32];
    let got = Sha256::digest(&bytes[..i]);
    if got.as_slice() != expected {
        bail!("`.bll` integrity hash mismatch");
    }
    Ok(PortableBll {
        package_name,
        version,
        abi_epoch,
        bitcode,
    })
}

/// True when bytes look like BLL v2 portable.
pub fn is_portable_bll(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == MAGIC
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn read_u16(bytes: &[u8], i: &mut usize) -> Result<u16> {
    if *i + 2 > bytes.len() {
        bail!("`.bll` truncated");
    }
    let v = u16::from_le_bytes([bytes[*i], bytes[*i + 1]]);
    *i += 2;
    Ok(v)
}

fn read_u32(bytes: &[u8], i: &mut usize) -> Result<u32> {
    if *i + 4 > bytes.len() {
        bail!("`.bll` truncated");
    }
    let v = u32::from_le_bytes(bytes[*i..*i + 4].try_into().unwrap());
    *i += 4;
    Ok(v)
}

fn read_u64(bytes: &[u8], i: &mut usize) -> Result<u64> {
    if *i + 8 > bytes.len() {
        bail!("`.bll` truncated");
    }
    let v = u64::from_le_bytes(bytes[*i..*i + 8].try_into().unwrap());
    *i += 8;
    Ok(v)
}

fn read_str(bytes: &[u8], i: &mut usize) -> Result<String> {
    let len = read_u32(bytes, i)? as usize;
    if *i + len > bytes.len() {
        bail!("`.bll` truncated string");
    }
    let s = std::str::from_utf8(&bytes[*i..*i + len])
        .map_err(|_| anyhow::anyhow!("`.bll` non-utf8 package id"))?
        .to_string();
    *i += len;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let bc = b"fake-bitcode-bytes";
        let enc = encode_portable_bll("acme.lib", "0.1.0", bc);
        let dec = decode_portable_bll(&enc).unwrap();
        assert_eq!(dec.package_name, "acme.lib");
        assert_eq!(dec.version, "0.1.0");
        assert_eq!(dec.bitcode, bc);
    }

    #[test]
    fn reject_v1() {
        let mut v1 = b"BLL\x01".to_vec();
        v1.extend_from_slice(&[0u8; 16]);
        assert!(decode_portable_bll(&v1).is_err());
    }
}
