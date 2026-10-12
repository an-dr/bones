//! Starting programs against a folder: a program detached from this
//! process, or a terminal opened there.
//!
//! - No knowledge of tools, settings, or argument templates: an application
//!   decides what to run and passes it here already expanded.
//! - No bus. Callers are native code; a guest that could start any program
//!   would reach past every boundary the sandbox draws (ADR-034).
//! - The environment is inherited untouched, plus [`Program::env`]: the
//!   engine never changes its own process environment, so a child sees what
//!   the user's shell would have given it.

mod launch;
mod launched;
mod program;
mod resolve;
mod terminal;

pub use launch::launch;
pub use launched::Launched;
pub use program::Program;
pub use resolve::resolve;
pub use terminal::open_terminal;

#[cfg(test)]
mod tests;
