use std::collections::{BTreeMap, HashMap};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, Registry};
use bones_kernel_infra::exit::ExitRequest;
use bones_kernel_infra::frame_control::FrameControl;
use bones_kernel_infra::logging::Logger;
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::renderer::Captured;
use bones_messages::{DecodeMessage, Message};

use crate::direct_send::{self, DirectSend};
use crate::frames::Frames;
use crate::log_history::LogHistory;
use crate::recorded::Recorded;
use crate::recording::{self, Redact};
use crate::server::{self, State};

/// Bus endpoint name.
pub const ENDPOINT: &str = "devtools";

/// Cap on the recorded traffic, so a long session cannot grow without bound
/// while nothing reads it.
pub(crate) const HISTORY: usize = 500;

/// The engine's frame tick. Recording every one would evict the history this
/// exists to keep — sixty a second — but dropping them entirely hides
/// whether the engine is still turning during a stall, so one per
/// `TICK_HEARTBEAT` is kept as a heartbeat.
pub(crate) const TICK_TOPIC: &str = "core/tick";
const TICK_HEARTBEAT: Duration = Duration::from_secs(1);

/// The module. Inert unless its variable names a port when `init` runs.
pub struct Devtools {
    variable: String,
    logger: Option<Logger>,
    redact: Option<Arc<Redact>>,
    started: Instant,
    last_tick: Option<Instant>,
    pub(crate) messages: Arc<Mutex<Vec<Recorded>>>,
    pub(crate) extensions: Arc<Mutex<BTreeMap<String, String>>>,
    logs: Option<LogHistory>,
    frames: Frames,
    captures: crate::server::Captures,
    /// Where `/send` requests are made, once `init` has it.
    registry: Option<Registry>,
    /// `/send` replies not yet collected, by request id.
    replies: crate::direct_send::Replies,
    address: Option<SocketAddr>,
    shutdown: Arc<AtomicBool>,
}

impl Devtools {
    /// A channel that listens on `127.0.0.1:<port>` when the environment
    /// variable `variable` holds a port number at `init`, and does nothing
    /// at all otherwise.
    ///
    /// The composition root names the variable — `COMMITS_DEVTOOLS_PORT`, for
    /// example — so each application keeps its own.
    pub fn from_env(variable: impl Into<String>) -> Self {
        Self {
            variable: variable.into(),
            logger: None,
            redact: None,
            started: Instant::now(),
            last_tick: None,
            messages: Arc::new(Mutex::new(Vec::new())),
            extensions: Arc::new(Mutex::new(BTreeMap::new())),
            logs: None,
            frames: Frames::new(),
            captures: Arc::new(Mutex::new(HashMap::new())),
            registry: None,
            replies: Arc::new(Mutex::new(HashMap::new())),
            address: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Logs where the channel listens.
    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Serves `history` on `/logs`. Tee the composition's logger into the
    /// same history (`Logger::tee`) for it to hold anything.
    pub fn log_history(mut self, history: LogHistory) -> Self {
        self.logs = Some(history);
        self
    }

    /// Keeps the payload of every envelope `redact` matches out of the
    /// recording, for traffic that carries secrets — a credential prompt's
    /// answer, say. Only the topic and sender are recorded for those.
    pub fn redacting(mut self, redact: impl Fn(&Envelope) -> bool + Send + Sync + 'static) -> Self {
        self.redact = Some(Arc::new(redact));
        self
    }

    /// Where the channel listens, once `init` has bound it.
    pub fn address(&self) -> Option<SocketAddr> {
        self.address
    }

    /// Whether `variable` holds a port number now -- whether a channel built
    /// from it would listen.
    pub fn requested(variable: &str) -> bool {
        Self::port(variable).is_some()
    }

    /// The port `variable` names, if it is set and holds one.
    pub(crate) fn port(variable: &str) -> Option<u16> {
        std::env::var(variable)
            .ok()
            .and_then(|port| port.trim().parse().ok())
    }
}

impl Handler for Devtools {
    /// Records the bus, which is the other half of driving the app: a test
    /// can post a message and then read what it caused without a screenshot,
    /// and a stall shows up as the topic the traffic stops at.
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic.starts_with("gfx/") {
            self.frames.record(envelope);
        }
        if envelope.topic == Captured::TOPIC {
            self.keep_capture(&envelope.payload);
        }
        if envelope.topic == direct_send::TOPIC && envelope.sender == ENDPOINT {
            self.make_send(&envelope.payload);
        }
        if envelope.topic == TICK_TOPIC {
            self.frames.end_frame();
            let now = Instant::now();
            if self
                .last_tick
                .is_some_and(|last| now.duration_since(last) < TICK_HEARTBEAT)
            {
                return;
            }
            self.last_tick = Some(now);
        }
        if envelope.topic == LifecycleEvent::TOPIC {
            self.track_lifecycle(&envelope.payload);
        }
        let redacted = self
            .redact
            .as_deref()
            .is_some_and(|redact| redact(envelope));
        let recorded = Recorded {
            ms: self.started.elapsed().as_millis(),
            topic: envelope.topic.clone(),
            sender: envelope.sender.clone(),
            preview: recording::preview(envelope, self.redact.as_deref()),
            payload: (!redacted).then(|| envelope.payload.clone()),
        };
        let mut messages = self.messages.lock().unwrap();
        if messages.len() >= HISTORY {
            messages.remove(0);
        }
        messages.push(recorded);
    }
}

impl Devtools {
    /// Makes a `/send` request on the frame thread and keeps the reply
    /// until the route collects it.
    fn make_send(&mut self, payload: &[u8]) {
        let (Ok(send), Some(registry)) = (DirectSend::decode(payload), &self.registry) else {
            return;
        };
        let reply = send.call(registry);
        self.replies.lock().unwrap().insert(send.id, reply);
    }

    /// Keeps a capture until `/screenshot` collects it by request id.
    fn keep_capture(&mut self, payload: &[u8]) {
        if let Ok(captured) = Captured::decode(payload) {
            self.captures
                .lock()
                .unwrap()
                .insert(captured.request_id, (captured.scope, captured.png.to_vec()));
        }
    }

    /// Keeps each extension's last lifecycle state for `/extensions`.
    fn track_lifecycle(&mut self, payload: &[u8]) {
        let Ok(event) = LifecycleEvent::decode(payload) else {
            return;
        };
        let state = match event.event {
            Event::Loaded => "loaded",
            Event::Faulted => "faulted",
            Event::Reloading => "reloading",
            Event::Reloaded => "reloaded",
            Event::Stopped => "stopped",
        };
        self.extensions
            .lock()
            .unwrap()
            .insert(event.extension.to_string(), state.to_string());
    }
}

impl Module for Devtools {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        let Some(port) = Self::port(&self.variable) else {
            return Ok(());
        };
        // Everything: a trailing `*` is a prefix match, so this is the whole
        // bus. Costly only when the channel is switched on.
        context.subscribe("*");
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
            format!(
                "binding the devtools channel on 127.0.0.1:{port}: {error}; another process, \
                 often another engine, holds the port: set {} to a free one",
                self.variable
            )
        })?;
        self.registry = context.get_service::<Registry>().cloned();
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("configuring devtools channel: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("reading devtools channel address: {error}"))?;
        let state = State {
            bus: context.get_service::<Bus>().cloned(),
            messages: Arc::clone(&self.messages),
            extensions: Arc::clone(&self.extensions),
            logs: self.logs.clone(),
            frame_control: context.get_service::<FrameControl>().cloned(),
            frames: self.frames.last(),
            captures: Arc::clone(&self.captures),
            next_capture: Arc::new(AtomicU32::new(0)),
            exit: context.get_service::<ExitRequest>().cloned(),
            replies: Arc::clone(&self.replies),
            next_send: Arc::new(AtomicU32::new(0)),
            shutdown: Arc::clone(&self.shutdown),
        };
        std::thread::Builder::new()
            .name("bones-devtools".to_string())
            .spawn(move || server::serve(listener, state))
            .map_err(|error| format!("starting devtools channel: {error}"))?;
        self.address = Some(address);
        if let Some(logger) = &self.logger {
            logger.info(
                ENDPOINT,
                &format!("devtools channel listening on {address}"),
            );
        }
        Ok(())
    }

    fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
