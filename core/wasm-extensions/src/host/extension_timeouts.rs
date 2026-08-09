use std::time::Duration;

/// Wall-clock budgets one extension's guest calls are held to (ADR-007).
///
/// - Wall clock, not work: a loaded host spends these as fast as the guest
///   does, so both need room for the machine being busy, not just for the
///   code being slow.
/// - Overrunning either is a trap, not a retry. `load` leaves the extension
///   unattached; `call` faults and quarantines a running one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionTimeouts {
    /// Covers `instantiate` + `init` together. Cold compilation is a
    /// legitimate one-time cost that `call` is not meant to absorb.
    ///
    /// The default suits a small component. One carrying an embedded
    /// language runtime takes longer than this to start.
    pub load: Duration,
    /// Covers a single `on-message`, `on-tick` or `respond`.
    ///
    /// Has to fit the slowest thing the extension legitimately does in one
    /// call, not the average: an extension that answers most messages in
    /// microseconds but occasionally reads a repository is judged on the
    /// read. The default is deliberately tight, on the assumption that a
    /// call blocking this long is a runaway rather than a workload -- raise
    /// it for an extension where that assumption is wrong.
    pub call: Duration,
}

impl Default for ExtensionTimeouts {
    fn default() -> Self {
        Self {
            load: Duration::from_secs(1),
            call: Duration::from_millis(50),
        }
    }
}
