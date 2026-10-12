use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_messages::web::PageMessage;
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::devtools::{HISTORY, TICK_TOPIC};
use crate::recording::preview;
use crate::request::Request;
use crate::server::{parameter, route, State};
use crate::Devtools;

fn request(line: &str, body: &str) -> Request {
    Request {
        line: line.to_string(),
        body: body.to_string(),
    }
}

fn state() -> State {
    State {
        bus: None,
        messages: Arc::new(Mutex::new(Vec::new())),
        shutdown: Arc::new(AtomicBool::new(false)),
    }
}

fn page_message(json: &str) -> Envelope {
    Envelope {
        topic: PageMessage::TOPIC.to_string(),
        sender: "web".to_string(),
        correlation: None,
        payload: PageMessage {
            owner: "app",
            panel: "main",
            json,
        }
        .encode(),
    }
}

#[test]
fn health_answers_without_a_bus() {
    let (status, body) = route(&request("GET /health HTTP/1.1", ""), &state());

    assert_eq!(status, "200 OK");
    assert_eq!(body, "ok\n");
}

#[test]
fn recorded_traffic_can_be_filtered_by_topic() {
    let state = state();
    state.messages.lock().unwrap().extend([
        "      1ms\tweb/page-message\tweb\tapp/main {\"a\":1}".to_string(),
        "      2ms\tgit/completed\tgit\t[4 bytes] ....".to_string(),
    ]);

    let (status, body) = route(&request("GET /messages?topic=git/ HTTP/1.1", ""), &state);

    assert_eq!(status, "200 OK");
    assert_eq!(body, "      2ms\tgit/completed\tgit\t[4 bytes] ....\n");
}

#[test]
fn an_empty_post_is_rejected_rather_than_published_as_nothing() {
    let (status, _) = route(
        &request("POST /page-message?owner=app&panel=main HTTP/1.1", "  "),
        &state(),
    );

    assert_eq!(status, "400 Bad Request");
}

#[test]
fn a_post_names_its_owner_and_panel() {
    let (status, _) = route(
        &request("POST /page-message HTTP/1.1", "{\"command\":\"refresh\"}"),
        &state(),
    );

    assert_eq!(status, "400 Bad Request");
}

#[test]
fn a_post_without_a_bus_reports_that_rather_than_claiming_success() {
    let (status, _) = route(
        &request(
            "POST /page-message?owner=app&panel=main HTTP/1.1",
            "{\"command\":\"refresh\"}",
        ),
        &state(),
    );

    assert_eq!(status, "503 Service Unavailable");
}

#[test]
fn query_parameters_are_read_by_name() {
    assert_eq!(parameter("owner=x&panel=y", "owner").as_deref(), Some("x"));
    assert_eq!(parameter("owner=x&panel=y", "panel").as_deref(), Some("y"));
    assert_eq!(parameter("", "owner"), None);
}

#[test]
fn an_unknown_route_lists_the_ones_that_exist() {
    let (status, body) = route(&request("GET /nope HTTP/1.1", ""), &state());

    assert_eq!(status, "404 Not Found");
    assert!(body.contains("/page-message"), "body: {body}");
}

#[test]
fn a_recorded_message_keeps_its_panel_and_json() {
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_UNSET");
    module.handle(&page_message("{\"command\":\"refresh\"}"));

    let messages = module.messages.lock().unwrap();
    assert_eq!(messages.len(), 1);
    assert!(
        messages[0].contains("web/page-message\tweb\tapp/main {\"command\":\"refresh\"}"),
        "recorded: {}",
        messages[0]
    );
}

/// One tick is kept so a stall is visible as a gap between heartbeats;
/// the rest are dropped so they cannot evict everything else.
#[test]
fn the_frame_tick_is_recorded_only_as_a_heartbeat() {
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_UNSET");
    let tick = Envelope {
        topic: TICK_TOPIC.to_string(),
        sender: "runner".to_string(),
        correlation: None,
        payload: vec![0, 1, 2, 3],
    };

    for _ in 0..100 {
        module.handle(&tick);
    }

    assert_eq!(module.messages.lock().unwrap().len(), 1);
}

#[test]
fn history_is_bounded_so_a_long_session_cannot_grow_without_end() {
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_UNSET");
    for index in 0..HISTORY + 10 {
        module.handle(&page_message(&format!("{{\"n\":{index}}}")));
    }

    let messages = module.messages.lock().unwrap();
    assert_eq!(messages.len(), HISTORY);
    assert!(messages
        .last()
        .unwrap()
        .contains(&format!("{}", HISTORY + 9)));
}

#[test]
fn a_redacted_envelope_keeps_its_payload_out_of_the_recording() {
    let redact = |envelope: &Envelope| envelope.topic == "os/prompt-response";
    let secret = Envelope {
        topic: "os/prompt-response".into(),
        sender: "test".into(),
        correlation: None,
        payload: b"1\nsecret".to_vec(),
    };

    let text = preview(&secret, Some(&redact));

    assert!(!text.contains("secret"));
    assert!(text.contains("redacted"));
}

fn parse(input: &str) -> Option<Request> {
    Request::read(&mut input.as_bytes())
}

#[test]
fn rejects_incomplete_oversized_and_ambiguous_bodies() {
    for headers in [
        "Content-Length: 9",
        "Content-Length: 999999",
        "Content-Length: nope",
        "Content-Length: 2\r\nContent-Length: 3",
        "Transfer-Encoding: chunked",
    ] {
        assert!(parse(&format!(
            "POST /page-message HTTP/1.1\r\n{headers}\r\n\r\n{{}}"
        ))
        .is_none());
    }
}

#[test]
fn reads_only_the_announced_body() {
    let request = parse("POST /page-message HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}extra").unwrap();
    assert_eq!(request.body, "{}");
}

#[test]
fn rejects_browser_requests_and_rebound_hosts() {
    for header in [
        "Origin: https://example.com",
        "Sec-Fetch-Site: cross-site",
        "Host: attacker.example",
    ] {
        assert!(parse(&format!("POST /page-message HTTP/1.1\r\n{header}\r\n\r\n")).is_none());
    }
    assert!(parse("GET /health HTTP/1.1\r\nHost: localhost:8642\r\n\r\n").is_some());
}

/// Initialises the module against a real bus, the way the engine would.
fn init(module: &mut Devtools, bus: &Bus) -> Vec<String> {
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    let mut context = ModuleContext::new(&mut services);
    module.init(&mut context).unwrap();
    context.into_subscriptions()
}

#[test]
fn with_the_variable_unset_nothing_listens_and_nothing_is_subscribed() {
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_NEVER_SET");

    let subscriptions = init(&mut module, &Bus::new());

    assert_eq!(module.address(), None);
    assert!(subscriptions.is_empty());
}

#[test]
fn a_variable_that_is_not_a_port_is_the_same_as_unset() {
    std::env::set_var("BONES_DEVTOOLS_TEST_NOT_A_PORT", "eighty");
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_NOT_A_PORT");

    let subscriptions = init(&mut module, &Bus::new());

    assert_eq!(module.address(), None);
    assert!(subscriptions.is_empty());
}

fn http(address: std::net::SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn a_posted_page_message_reaches_the_bus_and_is_read_back_from_the_recording() {
    // A port that was free a moment ago: bind, read it, release it.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::env::set_var("BONES_DEVTOOLS_TEST_PORT", port.to_string());
    let bus = Bus::new();
    let mut module = Devtools::from_env("BONES_DEVTOOLS_TEST_PORT");
    let subscriptions = init(&mut module, &bus);
    let address = module.address().expect("the channel should listen");
    let shared = Arc::new(Mutex::new(module));
    let recorder = Arc::clone(&shared);
    let endpoint = bus.register("devtools", move |envelope: &Envelope| {
        recorder.lock().unwrap().handle(envelope);
    });
    for topic in subscriptions {
        endpoint.subscribe(topic);
    }
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&received);
    let page_owner = bus.register("app", move |envelope: &Envelope| {
        sink.lock().unwrap().push(envelope.clone());
    });
    page_owner.subscribe(PageMessage::TOPIC);

    let body = "{\"command\":\"refresh\"}";
    let response = http(
        address,
        &format!(
            "POST /page-message?owner=app&panel=main HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    let deadline = Instant::now() + Duration::from_secs(5);
    while received.lock().unwrap().is_empty() {
        assert!(
            Instant::now() < deadline,
            "the message never reached the bus"
        );
        bus.dispatch();
        std::thread::sleep(Duration::from_millis(5));
    }

    let delivered = received.lock().unwrap()[0].clone();
    let message = PageMessage::decode(&delivered.payload).unwrap();
    assert_eq!(
        (message.owner, message.panel, message.json),
        ("app", "main", body)
    );
    assert_eq!(delivered.sender, "web");
    let recording = http(
        address,
        "GET /messages?topic=web/ HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(
        recording.contains("app/main {\"command\":\"refresh\"}"),
        "{recording}"
    );

    shared.lock().unwrap().shutdown();
}
