//! `host-probe`: a WASM extension that speaks the embedder's own vocabulary.
//!
//! This guest is ordinary in every way except its dependency list. It has no
//! filesystem, no environment, and no OS handles — so it asks for what it
//! cannot reach, over a contract neither it nor bones defines: the embedder
//! does, in `host-facts-messages`, and both sides link the same crate.
//!
//! The facts land on screen through the core `ui/*` vocabulary, so one panel
//! shows a custom request/reply and a bones-owned message side by side.

use bones_wasm_sdk::bindings::bones::core::host_api::{log, send, Level};
use bones_wasm_sdk::messages::ui::{Spec, Widget};
use bones_wasm_sdk::messages::{DecodeMessage, EncodeMessage, Message};
use bones_wasm_sdk::Guest;
use host_facts_messages::{Fact, FactsReply, FactsRequest, ENDPOINT};

/// Asks the native module for one fact.
///
/// A direct `send` (ADR-010), so the answer arrives inside this call rather
/// than as a later delivery — which is what makes the whole thing readable as
/// a function call despite crossing the sandbox boundary.
fn ask(fact: Fact<'_>) -> String {
    let request = FactsRequest { fact }.encode();
    match send(ENDPOINT, &request) {
        Ok(reply) => match FactsReply::decode(&reply) {
            Ok(reply) if reply.value.is_empty() => "(unavailable)".to_string(),
            Ok(reply) => reply.value.to_string(),
            Err(error) => {
                log(Level::Error, &format!("undecodable reply: {error}"));
                "(bad reply)".to_string()
            }
        },
        // The endpoint is missing whenever this extension is dropped into the
        // *shipped* bones binary instead of the custom one — the failure an
        // embedder's users will actually hit, so it is worth handling plainly.
        Err(_) => "(no host-facts module: run the custom engine)".to_string(),
    }
}

struct Component;

impl Guest for Component {
    fn init() {
        let hostname = ask(Fact::Hostname);
        let working_directory = ask(Fact::WorkingDirectory);
        let path_length = ask(Fact::EnvironmentVariable("PATH")).len();

        log(Level::Info, &format!("hostname: {hostname}"));
        log(Level::Info, &format!("working directory: {working_directory}"));
        log(Level::Info, &format!("PATH is {path_length} bytes"));

        // Published, not sent: a widget spec is a broadcast to whichever ui
        // module is composed, which may be none. Core vocabulary this time,
        // to show both kinds of message in one extension.
        let spec = Spec {
            title: "host facts",
            widgets: vec![
                Widget::Label {
                    text: "Answered by a native module this engine was built with:",
                },
                Widget::Label { text: &hostname },
                Widget::Label {
                    text: &working_directory,
                },
            ],
        };
        bones_wasm_sdk::bindings::bones::core::host_api::publish(Spec::TOPIC, &spec.encode());
    }

    fn shutdown() {}

    fn on_tick(_dt: f32) {}

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

bones_wasm_sdk::export!(Component);
