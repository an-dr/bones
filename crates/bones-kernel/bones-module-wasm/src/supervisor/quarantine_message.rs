use bones_kernel_infra::bus::{BudgetLimits, DropCounters};

/// The log line for a quarantine: which extension, why, and how to change
/// it (design/extensions.md, "Faults and quarantine").
///
/// - A budget overrun names the budget, its per-frame limit, what was
///   dropped, and the `bones.toml` key and builder call that raise it.
/// - Otherwise the extension trapped or timed out, which the host logged
///   with the faulting call when it happened.
pub(crate) fn quarantine_message(name: &str, drops: DropCounters, limits: BudgetLimits) -> String {
    let mut causes = Vec::new();
    if drops.publishes > 0 {
        causes.push(format!(
            "published more than {} messages in one frame ({} dropped); publish less \
             often, or raise extension_max_publishes in bones.toml \
             (Kernel::extension_budget when embedding)",
            limits.max_publishes, drops.publishes
        ));
    }
    if drops.inbound > 0 {
        causes.push(format!(
            "received more than {} messages in one frame ({} dropped); subscribe to \
             fewer topics, or raise extension_max_inbound in bones.toml \
             (Kernel::extension_budget when embedding)",
            limits.max_inbound, drops.inbound
        ));
    }
    let cause = match causes.is_empty() {
        true => "it trapped or ran past its time budget; the call is logged above".to_string(),
        false => causes.join("; it also "),
    };
    format!("'{name}' faulted and was quarantined: {cause}")
}

#[cfg(test)]
mod tests;
