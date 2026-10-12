use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel_infra::logging::Logger;

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
    pub(crate) messages: Arc<Mutex<Vec<String>>>,
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
            address: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Logs where the channel listens.
    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
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
        if envelope.topic == TICK_TOPIC {
            let now = Instant::now();
            if self
                .last_tick
                .is_some_and(|last| now.duration_since(last) < TICK_HEARTBEAT)
            {
                return;
            }
            self.last_tick = Some(now);
        }
        let line = format!(
            "{:>7}ms\t{}\t{}\t{}",
            self.started.elapsed().as_millis(),
            envelope.topic,
            envelope.sender,
            recording::preview(envelope, self.redact.as_deref())
        );
        let mut messages = self.messages.lock().unwrap();
        if messages.len() >= HISTORY {
            messages.remove(0);
        }
        messages.push(line);
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
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|error| format!("binding devtools channel: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("configuring devtools channel: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("reading devtools channel address: {error}"))?;
        let state = State {
            bus: context.get_service::<Bus>().cloned(),
            messages: Arc::clone(&self.messages),
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
