use std::sync::{Arc, Mutex};

use crate::bus::adapter::{Adapter, Patterns};

/// One persistent adapter per endpoint, for the engine's lifetime (ADR-013);
/// subscribe/release just mutate its pattern set.
#[derive(Clone)]
pub struct Endpoint {
    pub(crate) name: String,
    pub(crate) adapter: Arc<Mutex<Adapter>>,
    pub(crate) patterns: Patterns,
}

impl Endpoint {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subscribe(&self, pattern: impl Into<String>) {
        self.patterns().lock().unwrap().push(pattern.into());
    }

    pub fn release(&self, pattern: &str) {
        self.patterns().lock().unwrap().retain(|p| p != pattern);
    }

    pub fn release_all(&self) {
        self.patterns().lock().unwrap().clear();
    }

    fn patterns(&self) -> &Patterns {
        &self.patterns
    }
}
