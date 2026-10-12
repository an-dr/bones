use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{claim, Claim, Handoff, Primary};
use bones_kernel_infra::bus::{Bus, Envelope, Module, ModuleContext, ServiceRegistry};
use bones_kernel_infra::logging::Logger;
use bones_kernel_infra::window_control::WindowControl;
use bones_messages::instance::Launched;
use bones_messages::{DecodeMessage, Message};

use crate::InstanceModule;

const TIMEOUT: Duration = Duration::from_secs(2);

fn handoff(args: &[&str]) -> Handoff {
    Handoff {
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd: PathBuf::from("/work"),
    }
}

fn primary(app_uid: &str) -> Primary {
    match claim(app_uid, handoff(&[]), TIMEOUT).unwrap() {
        Claim::Primary(primary) => primary,
        Claim::HandedOff => panic!("{app_uid} was already claimed"),
    }
}

struct Harness {
    bus: Bus,
    module: InstanceModule,
    raised: Arc<AtomicUsize>,
    published: Arc<Mutex<Vec<Envelope>>>,
    _endpoint: bones_kernel_infra::bus::Endpoint,
}

/// The module over a real bus, with a fake `window-control` that counts.
fn harness(primary: Primary, with_window: bool) -> Harness {
    let bus = Bus::new();
    let raised = Arc::new(AtomicUsize::new(0));
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    if with_window {
        let counter = Arc::clone(&raised);
        services
            .provide(WindowControl::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .unwrap();
    }
    let mut module = InstanceModule::new(primary, Logger::default());
    module.init(&mut ModuleContext::new(&mut services)).unwrap();

    let published = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&published);
    let endpoint = bus.register("observer", move |envelope: &Envelope| {
        sink.lock().unwrap().push(envelope.clone());
    });
    endpoint.subscribe(Launched::TOPIC);
    Harness {
        bus,
        module,
        raised,
        published,
        _endpoint: endpoint,
    }
}

/// Runs frames until something is published, as the engine would.
fn frames_until_published(harness: &mut Harness) {
    for _ in 0..200 {
        harness.module.render();
        harness.bus.dispatch();
        if !harness.published.lock().unwrap().is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("nothing was published");
}

#[test]
fn a_later_launch_is_published_and_raises_the_window() {
    let app_uid = format!("bones-module-test-{}-raise", std::process::id());
    let mut harness = harness(primary(&app_uid), true);

    assert!(matches!(
        claim(&app_uid, handoff(&["--x", "y"]), TIMEOUT).unwrap(),
        Claim::HandedOff
    ));
    frames_until_published(&mut harness);

    let published = harness.published.lock().unwrap();
    let launched = Launched::decode(&published[0].payload).unwrap();
    assert_eq!(launched.args, ["--x", "y"]);
    assert_eq!(launched.cwd, "/work");
    assert_eq!(published[0].sender, "instance");
    assert_eq!(harness.raised.load(Ordering::SeqCst), 1);
}

#[test]
fn a_frame_without_a_launch_publishes_nothing_and_raises_nothing() {
    let app_uid = format!("bones-module-test-{}-idle", std::process::id());
    let mut harness = harness(primary(&app_uid), true);

    harness.module.render();
    harness.bus.dispatch();

    assert!(harness.published.lock().unwrap().is_empty());
    assert_eq!(harness.raised.load(Ordering::SeqCst), 0);
}

#[test]
fn a_headless_composition_still_publishes() {
    let app_uid = format!("bones-module-test-{}-headless", std::process::id());
    let mut harness = harness(primary(&app_uid), false);

    assert!(matches!(
        claim(&app_uid, handoff(&["z"]), TIMEOUT).unwrap(),
        Claim::HandedOff
    ));
    frames_until_published(&mut harness);

    assert_eq!(harness.published.lock().unwrap().len(), 1);
}
