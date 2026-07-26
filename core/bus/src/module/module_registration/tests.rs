use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use super::*;

struct TestModule {
    events: Arc<Mutex<Vec<&'static str>>>,
    dropped: Arc<AtomicBool>,
}

impl Handler for TestModule {
    fn handle(&mut self, _: &Envelope) {
        self.events.lock().unwrap().push("handle");
    }
}

impl Module for TestModule {
    fn name(&self) -> &str {
        "test-module"
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        self.events.lock().unwrap().push("init");
        context.subscribe("test/*");
        Ok(())
    }

    fn render(&mut self) {
        self.events.lock().unwrap().push("render");
    }

    fn shutdown(&mut self) {
        self.events.lock().unwrap().push("shutdown");
    }

    fn respond(&mut self, _: &str, _: &[u8]) -> Option<Vec<u8>> {
        Some(b"reply".to_vec())
    }
}

impl Drop for TestModule {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Relaxed);
    }
}

#[test]
fn attached_module_unregisters_and_shuts_down_on_detach() {
    let bus = Bus::new();
    let registry = Registry::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));
    let mut services = ServiceRegistry::new();
    let mut registration = ModuleRegistration::attach(
        bus.clone(),
        registry.clone(),
        &mut services,
        TestModule {
            events: Arc::clone(&events),
            dropped: Arc::clone(&dropped),
        },
    )
    .unwrap();

    bus.publish(Envelope {
        topic: "test/event".to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload: Vec::new(),
    });
    bus.dispatch();
    registration.render();
    assert_eq!(
        registry.call("caller", "test-module", b"request"),
        Ok(b"reply".to_vec())
    );

    registration.detach();
    registration.detach();
    assert!(!registration.is_attached());
    assert!(!registry.contains("test-module"));
    assert!(
        dropped.load(Ordering::Relaxed),
        "detach must release the module and its native resources"
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec!["init", "handle", "render", "shutdown"]
    );
}
