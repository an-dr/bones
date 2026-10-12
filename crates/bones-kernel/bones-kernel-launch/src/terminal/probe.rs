/// What choosing a terminal asks of the machine, behind a seam so tests can
/// describe a machine instead of reading this one.
pub(super) trait Probe {
    /// Whether `program` is installed.
    fn find(&self, program: &str) -> bool;
    /// An environment variable's value, such as `$TERMINAL`.
    fn variable(&self, name: &str) -> Option<String>;
    /// What a Debian-style alternatives link such as `x-terminal-emulator`
    /// finally points at.
    fn alternative(&self, link: &str) -> Option<String>;
}

/// The real machine.
pub(super) struct SystemProbe;

impl Probe for SystemProbe {
    fn find(&self, program: &str) -> bool {
        crate::resolve(program).is_some()
    }

    fn variable(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn alternative(&self, link: &str) -> Option<String> {
        let path = crate::resolve(link)?;
        let target = std::fs::canonicalize(path).ok()?;
        Some(target.to_string_lossy().into_owned())
    }
}
