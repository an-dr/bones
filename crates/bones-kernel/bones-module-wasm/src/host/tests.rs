use super::*;
use bones_kernel_infra::exit::ExitRequest;
use bones_kernel_infra::logging::RecordingSink;
use std::sync::{Arc, Mutex};

// Built by `cargo xtask test`, or alone by `cargo xtask example hello-demo`.
const HELLO_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../examples/extensions/hello-demo/target/wasm32-wasip2/release/hello_demo.wasm"
);
// Built by `cargo xtask test`, or alone by `cargo xtask example runaway-demo`.
const RUNAWAY_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../examples/extensions/runaway-demo/target/wasm32-wasip2/release/runaway_demo.wasm"
);

fn load_hello(bus: Bus, logger: Logger) -> Host {
    let engine = new_engine().unwrap();
    Host::load(
        &engine,
        HELLO_WASM,
        "hello",
        bus,
        Registry::new(),
        logger,
        ExitRequest::new(),
        DisplayInfo::default(),
        EndpointBudget::new(bones_kernel_infra::bus::BudgetLimits::default()),
        ExtensionTimeouts::default(),
        Permissions::default(),
    )
    .expect("build examples/extensions/hello-demo first: cargo xtask example hello-demo")
}

fn test_state(name: &str, registry: Registry) -> State {
    State {
        name: name.to_string(),
        logger: Logger::default(),
        bus: Bus::new(),
        registry,
        wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
        table: wasmtime_wasi::ResourceTable::new(),
        requested_topics: Vec::new(),
        budget: EndpointBudget::new(bones_kernel_infra::bus::BudgetLimits::default()),
        exit_requested: ExitRequest::new(),
        display_info: DisplayInfo::default(),
        diagnostics: Diagnostics::new(),
        permissions: Permissions::default(),
    }
}

struct EchoRespond;
impl bones_kernel_infra::bus::Respond for EchoRespond {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        Some([sender.as_bytes(), payload].concat())
    }
}

#[test]
fn send_reaches_a_registered_target_and_returns_its_reply() {
    let registry = Registry::new();
    registry.insert("echo", Arc::new(EchoRespond));
    let mut state = test_state("caller", registry);

    let reply = state.send("echo".to_string(), b"hi".to_vec()).unwrap();

    assert_eq!(reply, b"callerhi");
}

#[test]
fn send_to_an_unknown_endpoint_maps_to_the_wit_error() {
    let mut state = test_state("caller", Registry::new());

    assert_eq!(
        state.send("nobody".to_string(), Vec::new()),
        Err(SendError::UnknownEndpoint)
    );
}

#[test]
fn request_exit_sets_the_shared_flag() {
    let mut state = test_state("caller", Registry::new());
    let exit_requested = state.exit_requested.clone();
    assert!(!exit_requested.is_requested());

    state.request_exit();

    assert!(exit_requested.is_requested());
}

#[test]
fn send_to_self_maps_to_the_wit_cycle_error() {
    let registry = Registry::new();
    registry.insert("caller", Arc::new(EchoRespond));
    let mut state = test_state("caller", registry);

    assert_eq!(
        state.send("caller".to_string(), Vec::new()),
        Err(SendError::Cycle)
    );
}

#[test]
fn respond_calls_on_message_with_an_empty_topic() {
    let sink = RecordingSink::new();
    let mut host = load_hello(Bus::new(), Logger::new(Arc::new(sink.clone())));

    host.respond("someone", b"data");

    let records = sink.records();
    assert!(
        records
            .iter()
            .any(|(_, _, msg)| msg.contains("message on  from someone")),
        "expected on-message to run with an empty topic for a direct send, got {records:?}"
    );
}

#[test]
fn init_logs_through_the_engine() {
    let sink = RecordingSink::new();
    let _host = load_hello(Bus::new(), Logger::new(Arc::new(sink.clone())));

    let records = sink.records();
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("init")),
        "expected an init log line, got {records:?}"
    );
}

#[test]
fn shutdown_calls_the_guest_cleanup_hook() {
    let sink = RecordingSink::new();
    let mut host = load_hello(Bus::new(), Logger::new(Arc::new(sink.clone())));

    host.shutdown().unwrap();

    let records = sink.records();
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("shutdown")),
        "expected the shutdown export to run, got {records:?}"
    );
}

#[test]
fn requested_topics_reflects_what_init_subscribed_to() {
    let mut host = load_hello(Bus::new(), Logger::default());
    assert_eq!(
        host.requested_topics(),
        vec![
            Tick::TOPIC.to_string(),
            bones_messages::window::CloseRequested::TOPIC.to_string(),
        ]
    );
    assert!(host.requested_topics().is_empty(), "must drain, not repeat");
}

#[test]
fn on_message_is_dispatched_for_non_tick_topics() {
    let sink = RecordingSink::new();
    let bus = Bus::new();
    let host = load_hello(bus.clone(), Logger::new(Arc::new(sink.clone())));

    let ep = bus.register("hello", host);
    ep.subscribe("test/event");

    bus.publish(Envelope {
        topic: "test/event".to_string(),
        sender: "someone".to_string(),
        correlation: None,
        payload: Vec::new(),
    });
    bus.dispatch();

    let records = sink.records();
    assert!(
        records
            .iter()
            .any(|(_, _, msg)| msg.contains("message on test/event from someone")),
        "expected an on-message log line, got {records:?}"
    );
}

#[test]
fn on_tick_logs_through_the_engine_as_an_ordinary_bus_endpoint() {
    let sink = RecordingSink::new();
    let bus = Bus::new();
    let host = load_hello(bus.clone(), Logger::new(Arc::new(sink.clone())));

    let ep = bus.register("hello", host);
    ep.subscribe(Tick::TOPIC);

    bus.publish(Envelope {
        topic: Tick::TOPIC.to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload: (1.0f32 / 60.0).to_le_bytes().to_vec(),
    });
    bus.dispatch();

    let records = sink.records();
    assert!(
        records.iter().any(|(_, _, msg)| msg.contains("tick")),
        "expected a tick log line, got {records:?}"
    );
}

#[test]
fn publish_reaches_other_subscribers_on_the_same_bus() {
    let bus = Bus::new();
    let host = load_hello(bus.clone(), Logger::default());
    let hello_ep = bus.register("hello", host);
    hello_ep.subscribe("test/event");

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    let listener_ep = bus.register("listener", move |e: &Envelope| {
        sink.lock().unwrap().push(e.clone())
    });
    listener_ep.subscribe("hello/received");

    bus.publish(Envelope {
        topic: "test/event".to_string(),
        sender: "someone".to_string(),
        correlation: None,
        payload: Vec::new(),
    });
    bus.dispatch(); // delivers test/event to hello; its publish() only enqueues (ADR-015)
    bus.dispatch(); // delivers hello/received to listener

    let got = received.lock().unwrap();
    assert!(
        got.iter()
            .any(|e| e.topic == "hello/received" && e.sender == "hello"),
        "expected hello's publish to reach another subscriber, got {got:?}"
    );
}

fn tick_envelope() -> Envelope {
    Envelope {
        topic: Tick::TOPIC.to_string(),
        sender: "test".to_string(),
        correlation: None,
        payload: (1.0f32 / 60.0).to_le_bytes().to_vec(),
    }
}

#[test]
fn a_call_that_never_returns_traps_and_faults_instead_of_hanging_forever() {
    let sink = RecordingSink::new();
    let engine = new_engine().unwrap();
    let mut host = Host::load(
        &engine,
        RUNAWAY_WASM,
        "runaway",
        Bus::new(),
        Registry::new(),
        Logger::new(Arc::new(sink.clone())),
        ExitRequest::new(),
        DisplayInfo::default(),
        EndpointBudget::new(bones_kernel_infra::bus::BudgetLimits::default()),
        ExtensionTimeouts::default(),
        Permissions::default(),
    )
    .expect("build examples/extensions/runaway-demo first: cargo xtask example runaway-demo");
    assert!(!host.is_faulted(), "must not start out faulted");

    host.handle(&tick_envelope()); // blocks for ~the time budget, then traps

    assert!(
        host.is_faulted(),
        "an on-tick that never returns must fault the extension"
    );
    let records = sink.records();
    assert!(
        records
            .iter()
            .any(|(_, _, msg)| msg.contains("trapped in on-tick")),
        "expected a trap log line, got {records:?}"
    );
}

#[test]
fn a_faulted_host_ignores_further_deliveries_instead_of_hanging_again() {
    let engine = new_engine().unwrap();
    let mut host = Host::load(
        &engine,
        RUNAWAY_WASM,
        "runaway",
        Bus::new(),
        Registry::new(),
        Logger::default(),
        ExitRequest::new(),
        DisplayInfo::default(),
        EndpointBudget::new(bones_kernel_infra::bus::BudgetLimits::default()),
        ExtensionTimeouts::default(),
        Permissions::default(),
    )
    .expect("build examples/extensions/runaway-demo first: cargo xtask example runaway-demo");
    host.handle(&tick_envelope());
    assert!(host.is_faulted());

    // If this didn't short-circuit on is_faulted(), it would hang for
    // another full time budget (or forever, without epoch interruption
    // active a second time) instead of returning immediately.
    host.handle(&tick_envelope());
}

#[test]
fn a_budget_converts_to_whole_epoch_ticks_and_never_to_zero() {
    assert_eq!(timeout_ticks(ExtensionTimeouts::default().load), 200);
    assert_eq!(timeout_ticks(Duration::from_secs(10)), 2_000);
    // Rounds up rather than down: a sub-tick budget still buys one tick,
    // because a zero deadline traps on the very first epoch check.
    assert_eq!(timeout_ticks(Duration::from_millis(1)), 1);
    assert_eq!(timeout_ticks(Duration::ZERO), 1);
    // Saturates instead of wrapping, so "effectively unlimited" stays that
    // way rather than becoming a tiny deadline.
    assert_eq!(timeout_ticks(Duration::MAX), u64::MAX);
}

#[test]
fn a_denied_send_fails_as_an_unknown_endpoint_even_when_it_exists() {
    let registry = Registry::new();
    registry.insert("files", Arc::new(EchoRespond));
    let mut state = test_state("plugin", registry);
    state.permissions = Permissions::deny(["files"]);

    assert_eq!(
        state.send("files".to_string(), Vec::new()),
        Err(SendError::UnknownEndpoint)
    );
}

#[test]
fn a_denied_publish_is_dropped_and_an_allowed_one_is_not() {
    let mut state = test_state("plugin", Registry::new());
    state.permissions = Permissions::deny(["os/*"]);
    let log = bones_kernel_infra::logging::RecordingSink::new();
    state.logger = Logger::new(Arc::new(log.clone()));
    let received = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = received.clone();
    let endpoint = state.bus.register("listener", move |e: &Envelope| {
        sink.lock().unwrap().push(e.topic.clone());
    });
    endpoint.subscribe("*");

    state.publish("os/request".to_string(), Vec::new());
    state.publish("plugin/hello".to_string(), Vec::new());
    state.bus.dispatch();

    assert_eq!(*received.lock().unwrap(), ["plugin/hello"]);
    assert!(log
        .records()
        .iter()
        .any(|(_, _, line)| line.contains("may not publish to 'os/request'")));
}

#[test]
fn a_subscription_that_could_reach_a_denied_topic_is_not_requested() {
    let mut state = test_state("plugin", Registry::new());
    state.permissions = Permissions::deny(["os/*"]);

    state.subscribe("*".to_string());
    state.subscribe("os/result".to_string());
    state.subscribe("core/tick".to_string());

    assert_eq!(state.requested_topics, ["core/tick"]);
}
