//! The two version lines this build of `bones-cli` belongs to (ADR-029).

/// The engine line: this crate's own version.
pub const ENGINE: &str = env!("CARGO_PKG_VERSION");

/// `bones-messages`'s manifest, which carries the ABI line's version.
const MESSAGES_MANIFEST: &str = include_str!("../../bones-messages/Cargo.toml");

/// The ABI line: the version `bones-messages` and `bones-wasm-sdk` carry.
pub fn abi() -> &'static str {
    abi_from(MESSAGES_MANIFEST).expect("bones-messages/Cargo.toml has a version")
}

/// The first `version = "…"` in a manifest.
fn abi_from(manifest: &str) -> Option<&str> {
    manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
}

#[cfg(test)]
mod tests;
