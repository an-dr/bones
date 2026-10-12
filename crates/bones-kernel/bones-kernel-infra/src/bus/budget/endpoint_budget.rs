use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use super::{BudgetLimits, DropCounters};

/// Shared per-frame allowances and cumulative drop accounting for one endpoint.
#[derive(Clone)]
pub struct EndpointBudget {
    inner: Arc<State>,
}

struct State {
    limits: BudgetLimits,
    inbound: AtomicU32,
    publishes: AtomicU32,
    dropped_inbound: AtomicU64,
    dropped_publishes: AtomicU64,
    exceeded: AtomicBool,
}

impl EndpointBudget {
    pub fn new(limits: BudgetLimits) -> Self {
        Self {
            inner: Arc::new(State {
                limits,
                inbound: AtomicU32::new(0),
                publishes: AtomicU32::new(0),
                dropped_inbound: AtomicU64::new(0),
                dropped_publishes: AtomicU64::new(0),
                exceeded: AtomicBool::new(false),
            }),
        }
    }

    /// Resets only this frame's allowances; violations and drops are cumulative.
    pub fn begin_frame(&self) {
        self.inner.inbound.store(0, Ordering::Relaxed);
        self.inner.publishes.store(0, Ordering::Relaxed);
    }

    /// Accepts one inbound delivery, or records and rejects it over budget.
    pub fn accept_inbound(&self) -> bool {
        Self::accept(
            &self.inner.inbound,
            self.inner.limits.max_inbound,
            &self.inner.dropped_inbound,
            &self.inner.exceeded,
        )
    }

    /// Accepts one publish, or records and rejects it over budget.
    pub fn accept_publish(&self) -> bool {
        Self::accept(
            &self.inner.publishes,
            self.inner.limits.max_publishes,
            &self.inner.dropped_publishes,
            &self.inner.exceeded,
        )
    }

    pub fn has_exceeded(&self) -> bool {
        self.inner.exceeded.load(Ordering::Relaxed)
    }

    pub fn get_drop_counters(&self) -> DropCounters {
        DropCounters {
            inbound: self.inner.dropped_inbound.load(Ordering::Relaxed),
            publishes: self.inner.dropped_publishes.load(Ordering::Relaxed),
        }
    }

    fn accept(used: &AtomicU32, limit: u32, dropped: &AtomicU64, exceeded: &AtomicBool) -> bool {
        let count = used.fetch_add(1, Ordering::Relaxed);
        if count < limit {
            return true;
        }
        dropped.fetch_add(1, Ordering::Relaxed);
        exceeded.store(true, Ordering::Relaxed);
        false
    }
}
