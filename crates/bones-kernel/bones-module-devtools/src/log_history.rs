use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bones_kernel_infra::logging::{Level, LogSink};

/// Most lines kept; older ones are dropped first.
const LINES: usize = 1000;

/// The last log lines, for the channel's `/logs` route.
///
/// A [`LogSink`]: tee the composition's logger into it
/// (`Logger::tee`) so the channel sees what the console sees.
#[derive(Clone, Default)]
pub struct LogHistory {
    lines: Arc<Mutex<VecDeque<(Level, String, String)>>>,
}

impl LogHistory {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// The kept lines whose category and level match the filters, oldest
    /// first, at most `limit` of the newest.
    pub(crate) fn select(
        &self,
        category: Option<&str>,
        level: Option<&str>,
        limit: usize,
    ) -> Vec<(Level, String, String)> {
        let lines = self.lines.lock().unwrap();
        let selected: Vec<_> = lines
            .iter()
            .filter(|(_, line_category, _)| category.is_none_or(|c| c == line_category))
            .filter(|(line_level, _, _)| {
                level.is_none_or(|l| l.eq_ignore_ascii_case(name(*line_level)))
            })
            .cloned()
            .collect();
        let skip = selected.len().saturating_sub(limit);
        selected.into_iter().skip(skip).collect()
    }
}

impl LogSink for LogHistory {
    /// Keeps info and above: the runner logs every tick at debug, which
    /// would push every other line out of the history within seconds.
    fn log(&self, level: Level, category: &str, message: &str) {
        if level == Level::Debug {
            return;
        }
        let mut lines = self.lines.lock().unwrap();
        if lines.len() >= LINES {
            lines.pop_front();
        }
        lines.push_back((level, category.to_string(), message.to_string()));
    }
}

/// The level as the JSON and the filter spell it.
pub(crate) fn name(level: Level) -> &'static str {
    match level {
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}
