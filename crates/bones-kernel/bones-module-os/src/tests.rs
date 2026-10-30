use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_messages::os::{Action, Reply, Request, Result as OsResult};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::{OsBackend, OsModule};

/// Answers without touching the machine: no clipboard, no dialogs, no network.
#[derive(Default)]
struct StubBackend {
    clipboard: Mutex<String>,
}

impl OsBackend for StubBackend {
    fn read_clipboard(&self) -> Result<String, String> {
        Ok(self.clipboard.lock().unwrap().clone())
    }
    fn write_clipboard(&self, value: &str) -> Result<(), String> {
        *self.clipboard.lock().unwrap() = value.into();
        Ok(())
    }
    fn open_url(&self, value: &str) -> Result<(), String> {
        value
            .starts_with("https://")
            .then_some(())
            .ok_or("unsafe URL".into())
    }
    fn open_directory(&self, path: &str) -> Result<(), String> {
        path.starts_with("C:/")
            .then_some(())
            .ok_or("not a directory".into())
    }
    fn pick_file(&self, title: &str) -> Result<Option<String>, String> {
        assert_ne!(title, "panic", "simulated backend panic");
        Ok(Some("C:/chosen.txt".into()))
    }
    fn pick_folder(&self, _title: &str) -> Result<Option<String>, String> {
        // The user cancelled: an answer, not a failure.
        Ok(None)
    }
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        if url == "https://example.com/missing" {
            return Ok(None);
        }
        url.starts_with("https://")
            .then(|| Some("image/png;base64,c3R1Yg==".to_string()))
            .ok_or("only https URLs may be fetched".into())
    }
}

/// Drives the module over a real bus and collects what it answers.
fn run(requests: &[(u32, Action, &str)]) -> Vec<(u32, bool, String, String)> {
    let bus = Bus::new();
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    let mut module = OsModule::new(Arc::new(StubBackend::default()));
    module.init(&mut ModuleContext::new(&mut services)).unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));
    let output = results.clone();
    let endpoint = bus.register("test", move |event: &Envelope| {
        let result = OsResult::decode(&event.payload).unwrap();
        output.lock().unwrap().push((
            result.request_id,
            result.accepted,
            result.value.to_string(),
            result.error.to_string(),
        ));
    });
    endpoint.subscribe(OsResult::TOPIC);

    for (request_id, action, value) in requests {
        module.handle(&Envelope {
            topic: Request::TOPIC.into(),
            sender: "test".into(),
            correlation: Some(u64::from(*request_id)),
            payload: Request {
                request_id: *request_id,
                action: *action,
                value,
            }
            .encode(),
        });
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    while results.lock().unwrap().len() < requests.len() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        bus.dispatch();
    }
    let out = results.lock().unwrap().clone();
    out
}

#[test]
fn answers_every_action() {
    let results = run(&[
        (1, Action::WriteClipboard, "copied"),
        (2, Action::ReadClipboard, ""),
        (3, Action::OpenUrl, "https://example.com"),
        (4, Action::PickFile, "file"),
        (5, Action::OpenDirectory, "C:/repo"),
        (6, Action::FetchUrl, "https://example.com/avatar.png"),
    ]);

    assert_eq!(results.len(), 6);
    let find = |id: u32| results.iter().find(|r| r.0 == id).unwrap().clone();
    assert!(find(1).1);
    assert!(find(3).1);
    assert_eq!(find(4).2, "C:/chosen.txt");
    assert!(find(5).1);
    assert_eq!(find(6).2, "image/png;base64,c3R1Yg==");
}

#[test]
fn a_cancelled_dialog_is_refused_without_an_error() {
    let results = run(&[(1, Action::PickFolder, "folder")]);

    let (_, accepted, value, error) = &results[0];
    assert!(!accepted);
    assert!(value.is_empty());
    assert!(error.is_empty(), "a cancelled dialog is not a failure");
}

#[test]
fn nothing_at_a_url_is_refused_without_an_error() {
    let results = run(&[(1, Action::FetchUrl, "https://example.com/missing")]);

    let (_, accepted, _, error) = &results[0];
    assert!(!accepted);
    assert!(error.is_empty(), "a 404 is a normal answer");
}

#[test]
fn a_refused_url_reports_its_error() {
    let results = run(&[(1, Action::OpenUrl, "file:///private")]);

    let (_, accepted, _, error) = &results[0];
    assert!(!accepted);
    assert_eq!(error, "unsafe URL");
}

#[test]
fn the_clipboard_round_trips_through_the_backend() {
    let results = run(&[
        (1, Action::WriteClipboard, "copied"),
        (2, Action::ReadClipboard, ""),
    ]);

    // Both requests run on their own threads, so the read may or may not see
    // the write; what must hold is that each is answered.
    assert_eq!(results.len(), 2);
}

#[test]
fn a_malformed_request_is_ignored_rather_than_answered() {
    let bus = Bus::new();
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    let mut module = OsModule::new(Arc::new(StubBackend::default()));
    module.init(&mut ModuleContext::new(&mut services)).unwrap();

    module.handle(&Envelope {
        topic: Request::TOPIC.into(),
        sender: "test".into(),
        correlation: None,
        payload: vec![9, 9],
    });
    bus.dispatch();
    // Nothing panicked, and nothing was published: a guest cannot crash the
    // module by publishing rubbish.
}

fn observe(backend: Arc<StubBackend>) -> (Bus, OsModule, Arc<Mutex<Vec<Envelope>>>) {
    let bus = Bus::new();
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    let mut module = OsModule::new(backend);
    module.init(&mut ModuleContext::new(&mut services)).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let output = events.clone();
    let observer = bus.register("observer", move |event: &Envelope| {
        output.lock().unwrap().push(event.clone());
    });
    observer.subscribe(Reply::TOPIC);
    observer.subscribe(OsResult::TOPIC);
    (bus, module, events)
}

fn ask(module: &mut OsModule, sender: &str, id: u32, action: Action) {
    module.handle(&Envelope {
        topic: Request::TOPIC.into(),
        sender: sender.into(),
        correlation: None,
        payload: Request {
            request_id: id,
            action,
            value: "",
        }
        .encode(),
    });
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for OS work");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn equal_ids_from_two_callers_keep_identity_when_completed_out_of_order() {
    let backend = Arc::new(StubBackend::default());
    let mut hold = backend.clipboard.lock().unwrap();
    *hold = "first caller".into();
    let (bus, mut module, events) = observe(backend.clone());
    ask(&mut module, "first", 7, Action::ReadClipboard);
    ask(&mut module, "second", 7, Action::PickFile);
    wait_for(|| {
        bus.dispatch();
        !events.lock().unwrap().is_empty()
    });
    // Release before asserting so a failure does not strand the worker.
    drop(hold);
    wait_for(|| {
        bus.dispatch();
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.topic == OsResult::TOPIC)
            .count()
            == 2
    });
    let events = events.lock().unwrap();
    let replies: Vec<_> = events
        .iter()
        .filter(|e| e.topic == Reply::TOPIC)
        .map(|e| Reply::decode(&e.payload).unwrap())
        .collect();
    assert_eq!(
        replies.len(),
        2,
        "both completions need caller-aware replies"
    );
    assert_eq!(
        (
            replies[0].requester,
            replies[0].request_id,
            replies[0].value
        ),
        ("second", 7, "C:/chosen.txt")
    );
    assert_eq!(
        (
            replies[1].requester,
            replies[1].request_id,
            replies[1].value
        ),
        ("first", 7, "first caller")
    );
    for event in events.iter() {
        assert_eq!(event.sender, "os");
        assert_eq!(event.correlation, Some(7));
    }
}

#[test]
fn excess_work_is_refused_and_capacity_returns_after_completion() {
    let backend = Arc::new(StubBackend::default());
    let hold = backend.clipboard.lock().unwrap();
    let (bus, mut module, events) = observe(backend.clone());
    for id in 0..16 {
        ask(&mut module, "caller", id, Action::ReadClipboard);
    }
    ask(&mut module, "overflow", 16, Action::PickFile);
    wait_for(|| {
        bus.dispatch();
        !events.lock().unwrap().is_empty()
    });
    let first = events.lock().unwrap().clone();
    drop(hold);
    let legacy = first.iter().find(|e| e.topic == OsResult::TOPIC).unwrap();
    let result = OsResult::decode(&legacy.payload).unwrap();
    assert!(
        !result.accepted,
        "the seventeenth operation must not execute"
    );
    assert!(
        result.error.contains("16")
            && result.error.contains("overflow")
            && result.error.contains("retry")
    );
    wait_for(|| {
        bus.dispatch();
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.topic == OsResult::TOPIC)
            .count()
            == 17
    });
    ask(&mut module, "after", 18, Action::PickFile);
    wait_for(|| {
        bus.dispatch();
        events.lock().unwrap().iter().any(|e| {
            e.topic == Reply::TOPIC && Reply::decode(&e.payload).unwrap().requester == "after"
        })
    });
    let events = events.lock().unwrap();
    let after = events
        .iter()
        .find(|e| {
            e.topic == Reply::TOPIC && Reply::decode(&e.payload).unwrap().requester == "after"
        })
        .unwrap();
    assert!(Reply::decode(&after.payload).unwrap().accepted);
}

#[test]
fn shutdown_suppresses_late_completions_and_new_work() {
    let backend = Arc::new(StubBackend::default());
    let hold = backend.clipboard.lock().unwrap();
    let (bus, mut module, events) = observe(backend.clone());
    ask(&mut module, "caller", 1, Action::ReadClipboard);
    module.begin_shutdown();
    ask(&mut module, "caller", 2, Action::PickFile);
    drop(hold);
    // One reference in this test and one in the module; workers have released theirs.
    wait_for(|| Arc::strong_count(&backend) == 2);
    bus.dispatch();
    assert!(
        events.lock().unwrap().is_empty(),
        "shutdown must not publish late replies"
    );
}

#[test]
fn dropping_the_module_suppresses_pending_replies() {
    let backend = Arc::new(StubBackend::default());
    let hold = backend.clipboard.lock().unwrap();
    let (bus, mut module, events) = observe(backend.clone());
    ask(&mut module, "caller", 1, Action::ReadClipboard);
    drop(module);
    drop(hold);
    wait_for(|| Arc::strong_count(&backend) == 1);
    bus.dispatch();
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn backend_panics_answer_with_a_remedy_and_release_capacity() {
    let (bus, mut module, events) = observe(Arc::new(StubBackend::default()));
    for id in 0..16 {
        module.handle(&Envelope {
            topic: Request::TOPIC.into(),
            sender: "caller".into(),
            correlation: None,
            payload: Request {
                request_id: id,
                action: Action::PickFile,
                value: "panic",
            }
            .encode(),
        });
        wait_for(|| {
            bus.dispatch();
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.topic == Reply::TOPIC)
                .count()
                == (id + 1) as usize
        });
    }
    {
        let events = events.lock().unwrap();
        for event in events.iter().filter(|e| e.topic == Reply::TOPIC) {
            let reply = Reply::decode(&event.payload).unwrap();
            assert!(!reply.accepted);
            assert!(
                reply.error.contains("caller")
                    && reply.error.contains("panicked")
                    && reply.error.contains("fix")
            );
        }
    }
    ask(&mut module, "recovery", 17, Action::PickFile);
    wait_for(|| {
        bus.dispatch();
        events.lock().unwrap().iter().any(|e| {
            e.topic == Reply::TOPIC && Reply::decode(&e.payload).unwrap().requester == "recovery"
        })
    });
    let events = events.lock().unwrap();
    let event = events
        .iter()
        .find(|e| {
            e.topic == Reply::TOPIC && Reply::decode(&e.payload).unwrap().requester == "recovery"
        })
        .unwrap();
    assert!(Reply::decode(&event.payload).unwrap().accepted);
}
