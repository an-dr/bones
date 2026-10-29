use super::*;

const LIMITS: BudgetLimits = BudgetLimits {
    max_inbound: 1024,
    max_publishes: 64,
};

#[test]
fn a_publish_overrun_names_the_limit_and_the_setting() {
    let drops = DropCounters {
        inbound: 0,
        publishes: 7,
    };
    let message = quarantine_message("flood", drops, LIMITS);
    assert!(
        message.starts_with("'flood' faulted and was quarantined"),
        "{message}"
    );
    assert!(message.contains("more than 64 messages"), "{message}");
    assert!(message.contains("7 dropped"), "{message}");
    assert!(message.contains("extension_max_publishes"), "{message}");
    assert!(!message.contains("extension_max_inbound"), "{message}");
}

#[test]
fn an_inbound_overrun_names_its_own_setting() {
    let drops = DropCounters {
        inbound: 3,
        publishes: 0,
    };
    let message = quarantine_message("busy", drops, LIMITS);
    assert!(message.contains("extension_max_inbound"), "{message}");
}

#[test]
fn a_fault_without_drops_points_at_the_trap() {
    let drops = DropCounters {
        inbound: 0,
        publishes: 0,
    };
    let message = quarantine_message("runaway", drops, LIMITS);
    assert!(
        message.contains("trapped or ran past its time budget"),
        "{message}"
    );
}
