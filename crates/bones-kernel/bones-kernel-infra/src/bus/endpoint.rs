use std::sync::{Arc, Mutex};

use crate::bus::adapter::Adapter;

/// One persistent adapter per endpoint, for the engine's lifetime (ADR-013);
/// subscribe/release just mutate its pattern set.
#[derive(Clone)]
pub struct Endpoint {
    pub(crate) name: String,
    pub(crate) adapter: Arc<Mutex<Adapter>>,
}

impl Endpoint {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subscribe(&self, pattern: impl Into<String>) {
        self.adapter.lock().unwrap().patterns.push(pattern.into());
    }

    pub fn release(&self, pattern: &str) {
        self.adapter
            .lock()
            .unwrap()
            .patterns
            .retain(|p| p != pattern);
    }

    pub fn release_all(&self) {
        self.adapter.lock().unwrap().patterns.clear();
    }
}
