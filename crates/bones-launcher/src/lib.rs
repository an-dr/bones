//! Selects and starts installed applications; performs no installation mutations.

mod launching;
mod reporting;
mod selection;

pub use launching::run;
pub use reporting::report_error;

#[cfg(test)]
mod tests;
