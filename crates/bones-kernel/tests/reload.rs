//! Reload cleanup and failure isolation through the real extension host.
#![cfg(feature = "wasm")]

use bones_kernel::{
    bus::Envelope,
    logging::{Logger, RecordingSink},
    Kernel,
};
use bones_messages::{extension_control::Reload, EncodeMessage, Message};
use std::sync::{Arc, Mutex};

const HELLO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/extensions/hello-demo/target/wasm32-wasip2/release/hello_demo.wasm"
);

#[test]
fn failed_file_replacement_keeps_the_old_instance_without_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.wasm");
    std::fs::copy(HELLO, &path).unwrap();
    let sink = RecordingSink::new();
    let mut engine = Kernel::new()
        .extensions_dir(dir.path())
        .logger(Logger::new(Arc::new(sink.clone())))
        .build()
        .unwrap();
    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::fs::write(&path, b"invalid component").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(mtime + std::time::Duration::from_secs(1))
        .unwrap();
    engine.step(0.016);
    assert!(engine
        .registry
        .call("test", "app", b"still running")
        .is_ok());
    let records = sink.records();
    assert!(records
        .iter()
        .any(|(_, _, text)| text.contains("reload of 'app' failed")));
    assert!(!records
        .iter()
        .any(|(_, category, text)| category == "app" && text == "shutdown"));
}

#[test]
fn commanded_reload_initialises_then_cleans_up_once_and_preserves_cleanup_messages() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(HELLO, dir.path().join("app.wasm")).unwrap();
    let sink = RecordingSink::new();
    let mut engine = Kernel::new()
        .extensions_dir(dir.path())
        .extension_controller("app")
        .logger(Logger::new(Arc::new(sink.clone())))
        .build()
        .unwrap();
    let cleanups = Arc::new(Mutex::new(Vec::new()));
    let output = cleanups.clone();
    let observer = engine
        .runner
        .bus()
        .register("observer", move |event: &Envelope| {
            output.lock().unwrap().push(event.payload.clone());
        });
    observer.subscribe("hello/cleanup");
    engine.runner.bus().publish(Envelope {
        topic: Reload::TOPIC.into(),
        sender: "app".into(),
        correlation: None,
        payload: Reload { extension: "app" }.encode(),
    });
    engine.step(0.016);
    engine.step(0.016);
    let records = sink.records();
    let calls: Vec<_> = records
        .iter()
        .filter(|(_, category, text)| category == "app" && (text == "init" || text == "shutdown"))
        .map(|(_, _, text)| text.as_str())
        .collect();
    assert_eq!(calls, ["init", "init", "shutdown"]);
    assert_eq!(*cleanups.lock().unwrap(), vec![b"complete".to_vec()]);
}
