use std::collections::HashSet;

use bones_kernel_infra::bus::Bus;
use bones_kernel_infra::logging::Logger;
use bones_messages::catalogue;

/// Reports the mistakes one extension makes without faulting
/// (design/extensions.md, "Diagnostics").
///
/// - Each kind of mistake is reported once per topic, so a mistake repeated
///   every frame produces one log line rather than sixty a second.
/// - Logged under the extension's own name, where its own `log` calls land.
pub(crate) struct Diagnostics {
    reported: HashSet<String>,
}

impl Diagnostics {
    pub(crate) fn new() -> Self {
        Self {
            reported: HashSet::new(),
        }
    }

    /// Checks a payload the extension is publishing on `topic`: against the
    /// specification of that topic when it is a core one, and against the
    /// bus for whether anything will receive it.
    pub(crate) fn check_publish(
        &mut self,
        logger: &Logger,
        bus: &Bus,
        extension: &str,
        topic: &str,
        payload: &[u8],
    ) {
        if let Some(Err(error)) = catalogue::validate(topic, payload) {
            if self.first_time("malformed", topic) {
                logger.warn(
                    extension,
                    &format!("published {}", catalogue::explain(topic, payload, &error)),
                );
            }
        }
        // A topic already reported is not looked up again.
        if self.already("unheard", topic) || bus.has_subscriber(topic) {
            return;
        }
        self.first_time("unheard", topic);
        let unheard = format!("published '{topic}', which no endpoint subscribes to");
        if catalogue::TOPICS.contains(&topic) {
            logger.warn(
                extension,
                &format!("{unheard}; the module that handles it is probably not enabled"),
            );
        } else if let Some(meant) = catalogue::closest(topic) {
            logger.warn(extension, &format!("{unheard}; did you mean '{meant}'?"));
        } else {
            logger.debug(extension, &format!("{unheard} yet"));
        }
    }

    /// Warns when the extension subscribes to a topic that is not a core one
    /// but is within a few edits of one: almost always a typo, which would
    /// otherwise deliver nothing and say nothing.
    pub(crate) fn check_subscribe(&mut self, logger: &Logger, extension: &str, pattern: &str) {
        if pattern.ends_with('*') || catalogue::TOPICS.contains(&pattern) {
            return;
        }
        if let Some(meant) = catalogue::closest(pattern) {
            if self.first_time("misspelt subscription", pattern) {
                logger.warn(
                    extension,
                    &format!(
                        "subscribed to '{pattern}', which is not a core topic; did you mean '{meant}'?"
                    ),
                );
            }
        }
    }

    /// Reports a call the extension's deny-list refused (ADR-042), once per
    /// call kind and target.
    pub(crate) fn report_denied(
        &mut self,
        logger: &Logger,
        extension: &str,
        action: &str,
        target: &str,
    ) {
        if self.first_time(action, target) {
            logger.warn(
                extension,
                &format!(
                    "denied: may not {action} '{target}'; the application's deny-list for \
                     this extension refuses it"
                ),
            );
        }
    }

    fn first_time(&mut self, kind: &str, topic: &str) -> bool {
        self.reported.insert(format!("{kind} {topic}"))
    }

    fn already(&self, kind: &str, topic: &str) -> bool {
        self.reported.contains(&format!("{kind} {topic}"))
    }
}

#[cfg(test)]
mod tests;
