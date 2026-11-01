//! The git repository engines are built from and projects depend on.

pub const REPOSITORY_VAR: &str = "BONES_REPOSITORY";

const DEFAULT: &str = "https://github.com/an-dr/bones";

/// `BONES_REPOSITORY` when set (a mirror, a fork, or a local repository in
/// tests and CI), else the public repository.
pub fn url() -> String {
    std::env::var(REPOSITORY_VAR).unwrap_or_else(|_| DEFAULT.to_string())
}
