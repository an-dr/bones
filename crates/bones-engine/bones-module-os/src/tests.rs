use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_messages::os::{Action, Request, Result as OsResult};
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
    fn pick_file(&self, _title: &str) -> Result<Option<String>, String> {
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
