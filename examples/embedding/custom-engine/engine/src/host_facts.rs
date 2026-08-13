//! The native module: answers [`host_facts_messages`] requests with things a
//! WASM extension cannot find out for itself.

use bones_engine::bus::{Envelope, Handler, Module, ModuleContext};
use bones_engine::logging::Logger;
use bones_engine::messages::{DecodeMessage, EncodeMessage};
use host_facts_messages::{Fact, FactsReply, FactsRequest, ENDPOINT};

/// Reads host facts on behalf of extensions.
///
/// This is the shape of every reason to embed. An extension is sandboxed by
/// design — no filesystem, no environment, no OS handles — so a capability it
/// genuinely cannot have must come from native code the embedder composes in.
/// The module is the trusted half; the vocabulary is the contract between them.
pub struct HostFacts {
    logger: Logger,
}

impl HostFacts {
    /// Takes the engine's logger so its output interleaves with everything
    /// else, rather than going somewhere of its own.
    pub fn new(logger: Logger) -> Self {
        Self { logger }
    }

    /// Resolves one fact, or `None` when the host cannot answer.
    ///
    /// `std` only, deliberately: a clipboard or a native file dialog would make
    /// a flashier example and would drag in a bought dependency, which ADR-019
    /// treats as a decision deserving its own ADR rather than something an
    /// example introduces in passing.
    fn resolve(fact: Fact<'_>) -> Option<String> {
        match fact {
            // No portable std hostname, so ask the environment the way a real
            // module would, falling back across platforms.
            Fact::Hostname => std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .ok(),
            Fact::WorkingDirectory => std::env::current_dir()
                .ok()
                .map(|path| path.display().to_string()),
            Fact::EnvironmentVariable(name) => std::env::var(name).ok(),
        }
    }
}

impl Handler for HostFacts {
    // Nothing is published at this module; every request is a direct send, so
    // there is no topic to subscribe to and nothing to do here.
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for HostFacts {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
        self.logger
            .info(ENDPOINT, "ready: answering host-fact requests");
        Ok(())
    }

    /// Answers a direct `send` (ADR-010), which completes inside the caller's
    /// own call — the same mechanism `persistence` and `files` use, and the
    /// same one a WASM extension reaches through the WIT `send` import.
    fn respond(&mut self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        let request = match FactsRequest::decode(payload) {
            Ok(request) => request,
            Err(error) => {
                // A malformed request is the sender's bug, not a reason to
                // stop: log it and reply empty, which is what an extension
                // already has to handle.
                self.logger
                    .warn(ENDPOINT, &format!("{sender} sent an undecodable request: {error}"));
                return Some(FactsReply { value: "" }.encode());
            }
        };

        let value = Self::resolve(request.fact).unwrap_or_default();
        self.logger
            .info(ENDPOINT, &format!("{sender} asked for {:?}", request.fact));
        Some(FactsReply { value: &value }.encode())
    }
}
