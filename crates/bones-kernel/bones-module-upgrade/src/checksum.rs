use sha2::{Digest, Sha256};

/// Whether `data`'s SHA-256 digest matches `expected_hex` (case-insensitive).
pub(crate) fn verify_checksum(data: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let actual_hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    actual_hex.eq_ignore_ascii_case(expected_hex.trim())
}
