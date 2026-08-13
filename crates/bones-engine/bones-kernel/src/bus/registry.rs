use std::sync::{Arc, Mutex};

use crate::bus::{Respond, SendError};

// `RegistryInner` stays in this file rather than splitting further: it's
// purely `Registry`'s own internal store (the `Arc<Mutex<...>>` payload),
// never constructed or named outside this file, never meaningful on its own.
struct RegistryInner {
    targets: std::collections::HashMap<String, Arc<dyn Respond>>,
    /// Endpoints currently inside a synchronous call chain, outermost
    /// first — lets a request see its own name further up the chain and
    /// fail immediately instead of deadlocking (ADR-010).
    call_chain: Vec<String>,
}

/// Directory of endpoints reachable by name for direct `send` (ADR-010) —
/// separate from `Bus`'s pub/sub registration, which is by topic, not name.
/// Cheap to clone; every clone shares the same directory.
#[derive(Clone)]
pub struct Registry {
    inner: Arc<Mutex<RegistryInner>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryInner {
                targets: std::collections::HashMap::new(),
                call_chain: Vec::new(),
            })),
        }
    }

    pub fn insert(&self, name: impl Into<String>, target: Arc<dyn Respond>) {
        self.inner
            .lock()
            .unwrap()
            .targets
            .insert(name.into(), target);
    }

    /// Inserts only when the endpoint name is vacant.
    pub fn try_insert(&self, name: impl Into<String>, target: Arc<dyn Respond>) -> bool {
        use std::collections::hash_map::Entry;

        match self.inner.lock().unwrap().targets.entry(name.into()) {
            Entry::Vacant(entry) => {
                entry.insert(target);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn remove(&self, name: &str) {
        self.inner.lock().unwrap().targets.remove(name);
    }

    /// Reports whether a direct-call endpoint is currently registered.
    pub fn contains(&self, name: &str) -> bool {
        self.inner.lock().unwrap().targets.contains_key(name)
    }

    /// Synchronously calls `to` on behalf of `from`, returning its reply
    /// (empty if `to` chose not to reply). Fails immediately — never waits —
    /// if `to` is already somewhere in the current call chain, `from`
    /// included (a cycle, self-sends counting as the shortest one, would
    /// otherwise deadlock both sides).
    pub fn call(&self, from: &str, to: &str, payload: &[u8]) -> Result<Vec<u8>, SendError> {
        let (target, pushed_root) = {
            let mut inner = self.inner.lock().unwrap();
            // `from` is already "running" even on the first call in a chain
            // (that's what's calling `send` right now) — pushing it before
            // the cycle check is what makes a self-send (from == to) count
            // as a cycle instead of slipping through.
            let pushed_root = inner.call_chain.is_empty();
            if pushed_root {
                inner.call_chain.push(from.to_string());
            }
            if inner.call_chain.iter().any(|name| name == to) {
                if pushed_root {
                    inner.call_chain.pop();
                }
                return Err(SendError::Cycle);
            }
            let Some(target) = inner.targets.get(to).cloned() else {
                if pushed_root {
                    inner.call_chain.pop();
                }
                return Err(SendError::UnknownEndpoint);
            };
            inner.call_chain.push(to.to_string());
            (target, pushed_root)
        };

        let reply = target.respond(from, payload);

        let mut inner = self.inner.lock().unwrap();
        inner.call_chain.pop(); // `to`
        if pushed_root {
            inner.call_chain.pop(); // `from`
        }

        Ok(reply.unwrap_or_default())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
