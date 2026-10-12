use crate::logging::Level;

/// Injected at kernel infrastructure construction (ADR-012). Implementations must be
/// synchronous: `log` returning is the only delivery guarantee callers get.
pub trait LogSink: Send + Sync {
    fn log(&self, level: Level, category: &str, message: &str);
}
