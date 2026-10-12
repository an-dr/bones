//! What an `app_uid` may contain: the id every launch of one application
//! claims, which keeps two applications built on bones apart.
//!
//! The app_uid becomes part of a pipe name or a socket path, so it is limited to
//! characters that mean nothing to either, and to a length that keeps a Unix
//! socket path under the 108 bytes `sockaddr_un` holds.

const MAX_LEN: usize = 64;

/// Accepts `app_uid` when it is 1 to 64 ASCII letters, digits, `.`, `_` or `-`.
pub(crate) fn validate(app_uid: &str) -> Result<(), String> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    if app_uid.is_empty() || app_uid.len() > MAX_LEN || !app_uid.chars().all(allowed) {
        return Err(format!(
            "app_uid {app_uid:?} must be 1 to {MAX_LEN} ASCII letters, digits, '.', '_' or '-'"
        ));
    }
    Ok(())
}
