//! The backup container: a fixed, versioned header followed by one AEAD-sealed payload.
//!
//! ```text
//!  0..8    magic            "MEDTRBAK"
//!  8..10   format version   u16 BE   (this app writes 1; anything higher is refused)
//! 10..14   app schema ver.  u32 BE   (SQLite user_version of the database inside)
//! 14       kdf id           u8       (1 = Argon2id, v1.3)
//! 15..19   argon2 memory    u32 BE   KiB
//! 19..23   argon2 passes    u32 BE
//! 23..27   argon2 lanes     u32 BE
//! 27..43   salt             16 random bytes (per backup)
//! 43..67   nonce            24 random bytes (per backup, XChaCha20-Poly1305)
//! 67..75   payload length   u64 BE   (ciphertext incl. 16-byte tag)
//! 75..91   key check        16 bytes: AEAD tag of an empty message under the derived key
//! 91..95   header CRC-32    u32 BE   over bytes 0..91 (accident detection only, not security)
//! 95..     ciphertext       XChaCha20-Poly1305, the WHOLE header (0..95) is the associated data
//! ```
//!
//! Why a key check: an AEAD tag failure cannot tell "wrong passphrase" from "file was altered".
//! The key check (a second tag over nothing, under the same derived key) can: if it verifies, the
//! passphrase is right, so a later failure of the main tag means the contents were changed or
//! damaged. The CRC separates accidental header damage from a wrong passphrase, because a flipped
//! salt byte would otherwise look like a mistyped passphrase.
//!
//! All cryptography is RustCrypto (`argon2`, `chacha20poly1305`); nothing here is a new primitive.

use crate::error::{AppError, AppResult};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{rand_core::RngCore, Aead, KeyInit, OsRng, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

pub const MAGIC: [u8; 8] = *b"MEDTRBAK";
pub const FORMAT_VERSION: u16 = 1;
pub const KDF_ARGON2ID: u8 = 1;
pub const HEADER_LEN: usize = 95;
const TAG_LEN: u64 = 16;
/// A restore never allocates more than this for the encrypted data (a health log is kilobytes to a
/// few megabytes; this only stops a crafted length field from exhausting memory).
pub const MAX_PAYLOAD: u64 = 512 * 1024 * 1024;
const KEY_CHECK_AAD: &[u8] = b"medtracker-backup-key-check-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

/// Argon2id, 64 MiB, 3 passes, 1 lane: the OWASP-recommended class of setting, still about a
/// second on a phone. Stored in every header, so it can be raised later without breaking old files.
pub const DEFAULT_KDF: KdfParams = KdfParams { m_kib: 64 * 1024, t: 3, p: 1 };

impl KdfParams {
    /// Limits applied to a header read from a file: low enough that a crafted file cannot make the
    /// app allocate gigabytes, never below Argon2's own minimums.
    fn in_bounds(&self) -> bool {
        (1..=16).contains(&self.p) && self.m_kib >= 8 * self.p && self.m_kib <= 512 * 1024 && (1..=16).contains(&self.t)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub format_version: u16,
    pub app_schema_version: u32,
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    pub nonce: [u8; 24],
    pub payload_len: u64,
    pub key_check: [u8; 16],
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

impl Header {
    fn encode(&self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[0..8].copy_from_slice(&MAGIC);
        h[8..10].copy_from_slice(&self.format_version.to_be_bytes());
        h[10..14].copy_from_slice(&self.app_schema_version.to_be_bytes());
        h[14] = KDF_ARGON2ID;
        h[15..19].copy_from_slice(&self.kdf.m_kib.to_be_bytes());
        h[19..23].copy_from_slice(&self.kdf.t.to_be_bytes());
        h[23..27].copy_from_slice(&self.kdf.p.to_be_bytes());
        h[27..43].copy_from_slice(&self.salt);
        h[43..67].copy_from_slice(&self.nonce);
        h[67..75].copy_from_slice(&self.payload_len.to_be_bytes());
        h[75..91].copy_from_slice(&self.key_check);
        let crc = crc32(&h[..91]);
        h[91..95].copy_from_slice(&crc.to_be_bytes());
        h
    }
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b.try_into().expect("4 bytes"))
}

fn derive_key(passphrase: &str, salt: &[u8; 16], kdf: KdfParams) -> AppResult<Zeroizing<[u8; 32]>> {
    let params = Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32))
        .map_err(|e| AppError::BackupCorrupt(format!("The backup's encryption settings are not valid ({e}).")))?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|e| AppError::Storage(format!("Could not derive the encryption key ({e}).")))?;
    Ok(key)
}

/// The key-check nonce differs from the payload nonce (first byte inverted), so one key never
/// encrypts under the same nonce twice.
fn key_check_nonce(nonce: &[u8; 24]) -> [u8; 24] {
    let mut n = *nonce;
    n[0] ^= 0xFF;
    n
}

fn key_check_tag(cipher: &XChaCha20Poly1305, nonce: &[u8; 24]) -> AppResult<[u8; 16]> {
    let tag = cipher
        .encrypt(XNonce::from_slice(&key_check_nonce(nonce)), Payload { msg: &[], aad: KEY_CHECK_AAD })
        .map_err(|_| AppError::Storage("Could not encrypt the backup.".into()))?;
    tag.try_into().map_err(|_| AppError::Storage("Could not encrypt the backup.".into()))
}

/// Encrypts `plaintext` into a complete backup file image.
pub fn seal(plaintext: &[u8], passphrase: &str, app_schema_version: u32, kdf: KdfParams) -> AppResult<Vec<u8>> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let key = derive_key(passphrase, &salt, kdf)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
    let header = Header {
        format_version: FORMAT_VERSION,
        app_schema_version,
        kdf,
        salt,
        nonce,
        payload_len: plaintext.len() as u64 + TAG_LEN,
        key_check: key_check_tag(&cipher, &nonce)?,
    };
    let head = header.encode();
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), Payload { msg: plaintext, aad: &head })
        .map_err(|_| AppError::Storage("Could not encrypt the backup.".into()))?;
    let mut out = Vec::with_capacity(HEADER_LEN + ct.len());
    out.extend_from_slice(&head);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn not_a_backup() -> AppError {
    AppError::BackupNotABackup("This file is not a MedTracker backup. Choose a file that MedTracker made with Create backup.".into())
}

fn truncated(what: &str) -> AppError {
    AppError::BackupTruncated(format!(
        "This backup file is incomplete ({what}). It may not have finished copying or downloading. Copy it again from where it came from."
    ))
}

/// Parses and checks the header only. Needs no passphrase and does no expensive work, so the
/// caller can reject a newer or damaged file before asking for anything.
pub fn read_header(bytes: &[u8], supported_schema: u32) -> AppResult<Header> {
    let n = bytes.len().min(MAGIC.len());
    if bytes[..n] != MAGIC[..n] {
        return Err(not_a_backup());
    }
    if bytes.len() < MAGIC.len() {
        return Err(truncated("the file is shorter than its own label"));
    }
    if bytes.len() < HEADER_LEN {
        return Err(truncated("the file ends inside its header"));
    }
    let version = u16::from_be_bytes([bytes[8], bytes[9]]);
    if version > FORMAT_VERSION {
        return Err(AppError::BackupUnsupportedFormat(format!(
            "This backup was made by a newer version of MedTracker (backup format {version}; this version reads format {FORMAT_VERSION}). \
             Update MedTracker, then try again. Nothing was changed."
        )));
    }
    if version == 0 {
        return Err(AppError::BackupCorrupt("The backup's header is damaged. Nothing was changed.".into()));
    }
    if crc32(&bytes[..91]) != be32(&bytes[91..95]) {
        return Err(AppError::BackupCorrupt(
            "The backup's header is damaged, so the file cannot be trusted. Nothing was changed. Use another copy of the backup.".into(),
        ));
    }
    if bytes[14] != KDF_ARGON2ID {
        return Err(AppError::BackupUnsupportedFormat(
            "This backup uses an encryption setting this version of MedTracker does not know. Update MedTracker, then try again. Nothing was changed."
                .into(),
        ));
    }
    let kdf = KdfParams { m_kib: be32(&bytes[15..19]), t: be32(&bytes[19..23]), p: be32(&bytes[23..27]) };
    if !kdf.in_bounds() {
        return Err(AppError::BackupCorrupt("The backup's encryption settings are outside safe limits, so it was not opened. Nothing was changed.".into()));
    }
    let app_schema_version = be32(&bytes[10..14]);
    if i64::from(app_schema_version) > i64::from(supported_schema) {
        return Err(AppError::BackupNewerSchema(format!(
            "This backup holds data from a newer version of MedTracker (data format {app_schema_version}; this version supports {supported_schema}). \
             Update MedTracker, then restore it. Nothing was changed."
        )));
    }
    let payload_len = u64::from_be_bytes(bytes[67..75].try_into().expect("8 bytes"));
    if !(TAG_LEN..=MAX_PAYLOAD).contains(&payload_len) {
        return Err(AppError::BackupCorrupt("The backup's size field is not valid, so the file cannot be trusted. Nothing was changed.".into()));
    }
    let have = (bytes.len() - HEADER_LEN) as u64;
    if have < payload_len {
        return Err(truncated(&format!("{have} of {payload_len} bytes of data present")));
    }
    if have > payload_len {
        return Err(AppError::BackupCorrupt("The backup has extra bytes after its data, so it was changed or damaged. Nothing was changed.".into()));
    }
    Ok(Header {
        format_version: version,
        app_schema_version,
        kdf,
        salt: bytes[27..43].try_into().expect("16"),
        nonce: bytes[43..67].try_into().expect("24"),
        payload_len,
        key_check: bytes[75..91].try_into().expect("16"),
    })
}

/// Checks the passphrase and authenticates + decrypts the payload.
pub fn open(bytes: &[u8], passphrase: &str, supported_schema: u32) -> AppResult<(Header, Zeroizing<Vec<u8>>)> {
    let header = read_header(bytes, supported_schema)?;
    let key = derive_key(passphrase, &header.salt, header.kdf)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
    if key_check_tag(&cipher, &header.nonce)? != header.key_check {
        return Err(AppError::BackupWrongPassphrase(
            "That passphrase does not open this backup. Check it and try again. MedTracker cannot recover a forgotten passphrase.".into(),
        ));
    }
    let plain = cipher
        .decrypt(XNonce::from_slice(&header.nonce), Payload { msg: &bytes[HEADER_LEN..], aad: &bytes[..HEADER_LEN] })
        .map_err(|_| {
            AppError::BackupCorrupt(
                "The passphrase is right, but the backup's contents were changed or damaged, so they cannot be trusted. Nothing was changed. Use another copy of the backup."
                    .into(),
            )
        })?;
    Ok((header, Zeroizing::new(plain)))
}
