//! Extension lifecycle transitions published on `core/lifecycle`.

mod event;
mod lifecycle_event;

pub use event::Event;
pub use lifecycle_event::LifecycleEvent;

#[cfg(test)]
mod tests;
