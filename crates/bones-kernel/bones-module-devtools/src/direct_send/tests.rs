use std::sync::Arc;

use bones_kernel_infra::bus::Respond;

use super::*;

struct Echo;

impl Respond for Echo {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        Some([sender.as_bytes(), payload].concat())
    }
}

#[test]
fn a_send_round_trips_through_its_envelope() {
    let send = DirectSend {
        id: 7,
        endpoint: "calculator".to_string(),
        payload: b"{}".to_vec(),
    };
    let envelope = send.envelope();
    assert_eq!(envelope.topic, TOPIC);
    assert_eq!(DirectSend::decode(&envelope.payload), Ok(send));
}

#[test]
fn the_call_reaches_the_endpoint_as_devtools() {
    let registry = Registry::new();
    registry.insert("echo", Arc::new(Echo));
    let send = DirectSend {
        id: 1,
        endpoint: "echo".to_string(),
        payload: b"!".to_vec(),
    };
    assert_eq!(send.call(&registry), Ok(b"devtools!".to_vec()));
}

#[test]
fn an_unknown_endpoint_is_an_error() {
    let send = DirectSend {
        id: 1,
        endpoint: "nobody".to_string(),
        payload: Vec::new(),
    };
    assert!(send.call(&Registry::new()).is_err());
}
