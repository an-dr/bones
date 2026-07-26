/// Cumulative messages rejected by one endpoint's allowances.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DropCounters {
    pub inbound: u64,
    pub publishes: u64,
}
