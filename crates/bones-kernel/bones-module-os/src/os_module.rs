use crate::{OsBackend, SystemOsBackend};
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_messages::os::{Action, Reply, Request, Result as OsResult, ENDPOINT};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::thread;

/// Maximum simultaneously admitted OS operations, with no waiting queue.
pub const MAX_IN_FLIGHT: usize = 16;

// Private admission state, only constructed and used by OsModule.
#[derive(Default)]
struct WorkState {
    active: usize,
    closed: bool,
}

/// Bounded OS work: `os/request` in, caller-aware `os/reply` and legacy `os/result` out.
pub struct OsModule {
    bus: Option<Bus>,
    backend: Arc<dyn OsBackend>,
    work: Arc<Mutex<WorkState>>,
}

impl OsModule {
    /// Composes a backend with the fixed [`MAX_IN_FLIGHT`] admission limit.
    pub fn new(backend: Arc<dyn OsBackend>) -> Self {
        Self {
            bus: None,
            backend,
            work: Arc::default(),
        }
    }

    /// Runs one request off the bus thread and answers when it finishes.
    fn start(&self, requester: &str, request_id: u32, action: Action, value: String) {
        let Some(bus) = self.bus.clone() else { return };
        {
            let mut state = self.work.lock().unwrap();
            if state.closed {
                return;
            }
            if state.active == MAX_IN_FLIGHT {
                publish(&bus, requester, request_id, Err(format!(
                    "OS request from '{requester}' refused: {MAX_IN_FLIGHT} operations are active; wait for a completion before retrying"
                )));
                return;
            }
            state.active += 1;
        }
        let backend = self.backend.clone();
        let work = self.work.clone();
        let caller = requester.to_string();
        let worker_bus = bus.clone();
        let spawned = thread::Builder::new()
            .name("bones-os".into())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    execute(backend.as_ref(), action, &value)
                }))
                .unwrap_or_else(|_| {
                    Err(format!(
                        "OS backend panicked for '{caller}'; fix the host backend before retrying"
                    ))
                });
                let mut state = work.lock().unwrap();
                state.active -= 1;
                // Synchronize completion with shutdown so a closed module cannot
                // enqueue a late reply.
                if !state.closed {
                    publish(&worker_bus, &caller, request_id, outcome);
                }
            });
        if let Err(error) = spawned {
            let mut state = self.work.lock().unwrap();
            state.active -= 1;
            if !state.closed {
                publish(&bus, requester, request_id, Err(format!(
                    "OS worker for '{requester}' could not start: {error}; free host resources before retrying"
                )));
            }
        }
    }
}

impl Default for OsModule {
    fn default() -> Self {
        Self::new(Arc::new(SystemOsBackend))
    }
}

impl Handler for OsModule {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Request::TOPIC {
            return;
        }
        if let Ok(request) = Request::decode(&envelope.payload) {
            self.start(
                &envelope.sender,
                request.request_id,
                request.action,
                request.value.to_string(),
            );
        }
    }
}

impl Module for OsModule {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn begin_shutdown(&mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.work.lock().unwrap().closed = true;
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe(Request::TOPIC);
        self.bus = context.get_service::<Bus>().cloned();
        self.bus
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "no Bus service available".into())
    }
}

impl Drop for OsModule {
    fn drop(&mut self) {
        self.work.lock().unwrap().closed = true;
    }
}

/// Both wire contracts describe the same completion; sender remains the OS module.
fn publish(bus: &Bus, requester: &str, request_id: u32, outcome: Result<Option<String>, String>) {
    let (accepted, value, error) = match outcome {
        Ok(Some(value)) => (true, value, String::new()),
        Ok(None) => (false, String::new(), String::new()),
        Err(error) => (false, String::new(), error),
    };
    let result = OsResult {
        request_id,
        accepted,
        value: &value,
        error: &error,
    };
    let reply = Reply {
        requester,
        request_id,
        accepted,
        value: &value,
        error: &error,
    };
    for (topic, payload) in [
        (Reply::TOPIC, reply.encode()),
        (OsResult::TOPIC, result.encode()),
    ] {
        bus.publish(Envelope {
            topic: topic.into(),
            sender: ENDPOINT.into(),
            correlation: Some(u64::from(request_id)),
            payload,
        });
    }
}

/// Dispatches one action to the backend.
///
/// `Ok(None)` is the "nothing, and nothing wrong" answer: a cancelled dialog
/// or a URL holding nothing. Actions with no result of their own answer with
/// an empty string so the caller still learns they completed.
fn execute(backend: &dyn OsBackend, action: Action, value: &str) -> Result<Option<String>, String> {
    match action {
        Action::ReadClipboard => backend.read_clipboard().map(Some),
        Action::WriteClipboard => backend.write_clipboard(value).map(|_| Some(String::new())),
        Action::OpenUrl => backend.open_url(value).map(|_| Some(String::new())),
        Action::PickFile => backend.pick_file(value),
        Action::PickFolder => backend.pick_folder(value),
        Action::OpenDirectory => backend.open_directory(value).map(|_| Some(String::new())),
        Action::FetchUrl => backend.fetch_url(value),
    }
}
