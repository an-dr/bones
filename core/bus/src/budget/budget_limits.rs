/// Per-frame message allowances for one untrusted endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetLimits {
    pub max_inbound: u32,
    pub max_publishes: u32,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            max_inbound: 1_024,
            max_publishes: 1_024,
        }
    }
}
