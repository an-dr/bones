use super::*;

#[test]
fn an_answer_splits_into_status_type_and_body() {
    let reply = parse_reply(
        b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 4\r\n\r\n\x89PNG",
    )
    .unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(reply.content_type, "image/png");
    assert_eq!(reply.body, b"\x89PNG");
}

#[test]
fn headers_are_found_by_lower_case_name() {
    let reply = parse_reply(
        b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nX-Bones-Capture-Scope: window\r\n\r\n",
    )
    .unwrap();
    assert_eq!(reply.header("x-bones-capture-scope"), Some("window"));
    assert_eq!(reply.header("x-missing"), None);
}

#[test]
fn a_closed_port_says_how_to_start_the_engine() {
    // Port 1 is reserved and never listening on loopback.
    let error = DevtoolsClient { port: 1 }
        .request("GET", "/health", b"")
        .unwrap_err();
    assert!(error.contains("launch"), "{error}");
}
