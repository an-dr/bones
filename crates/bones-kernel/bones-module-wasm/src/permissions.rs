/// The topics and endpoints one extension may not use (ADR-042).
///
/// - Entries are exact names (`files`) or prefixes ending in `*` (`os/*`),
///   the syntax of a subscription.
/// - The default denies nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Permissions {
    denied: Vec<String>,
}

impl Permissions {
    /// Denies every topic and endpoint `patterns` names.
    pub fn deny<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            denied: patterns.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether the extension may publish to `topic` or `send` to the endpoint
    /// `topic` names.
    pub fn allows(&self, topic: &str) -> bool {
        !self.denied.iter().any(|denied| overlaps(denied, topic))
    }

    /// Whether the extension may subscribe to `pattern`: refused when the
    /// pattern could match any denied topic, so `*` is refused once anything
    /// is denied.
    pub fn allows_subscription(&self, pattern: &str) -> bool {
        !self.denied.iter().any(|denied| overlaps(denied, pattern))
    }
}

/// Whether some topic matches both patterns.
fn overlaps(a: &str, b: &str) -> bool {
    match (a.strip_suffix('*'), b.strip_suffix('*')) {
        (Some(a), Some(b)) => a.starts_with(b) || b.starts_with(a),
        (Some(prefix), None) => b.starts_with(prefix),
        (None, Some(prefix)) => a.starts_with(prefix),
        (None, None) => a == b,
    }
}

#[cfg(test)]
mod tests;
