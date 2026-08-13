//! Argon2id password hashing.

use crate::error::AppError;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Hash a password using Argon2id with the provided cost (m-cost in KiB).
pub fn hash_password(plain: &str, m_cost_kib: u32) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let params = argon2::Params::new(m_cost_kib, 3, 1, Some(32))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 params: {e}")))?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let hash = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 hash: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a password against a PHC-formatted hash.
pub fn verify_password(plain: &str, phc: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(phc)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 parse: {e}")))?;
    Ok(Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok())
}

/// Lowercase-hex helper used for token hashing.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// SHA-256 hex of input — used to fingerprint refresh tokens before storing.
pub fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex(&h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let h = hash_password("hunter2", 4096).unwrap();
        assert!(verify_password("hunter2", &h).unwrap());
        assert!(!verify_password("nope", &h).unwrap());
    }
}
