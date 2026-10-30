use bones_test::kernel::Kernel;
use bones_test::Stub;

#[test]
fn a_stub_answers_direct_sends_and_keeps_what_it_received() {
    let web = Stub::new("web").reply(b"ok".to_vec());
    let engine = Kernel::new().module(web.clone()).build().unwrap();

    let reply = engine.registry.call("guest", "web", b"open").unwrap();

    assert_eq!(reply, b"ok");
    assert_eq!(web.received(), [("guest".to_string(), b"open".to_vec())]);
}
