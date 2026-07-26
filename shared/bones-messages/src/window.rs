//! Window lifecycle events published by the platform.

mod close_requested;

pub use close_requested::CloseRequested;

#[cfg(test)]
mod tests;
