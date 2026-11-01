use super::*;

fn ask(line: &str) -> Option<Value> {
    answer(line, &mut Tools::new())
}

#[test]
fn initialize_offers_tools() {
    let answer = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#).unwrap();
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["result"]["protocolVersion"], PROTOCOL_VERSION);
    assert!(answer["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn a_notification_gets_no_answer() {
    assert_eq!(
        ask(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
        None
    );
}

#[test]
fn tools_list_names_every_tool_with_a_schema() {
    let answer = ask(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
    let tools = answer["result"]["tools"].as_array().unwrap();
    for name in [
        "topics",
        "launch",
        "publish",
        "messages",
        "screenshot",
        "step",
    ] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(tool["inputSchema"]["type"], "object", "{name}");
    }
}

#[test]
fn an_unknown_method_is_a_json_rpc_error() {
    let answer = ask(r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#).unwrap();
    assert_eq!(answer["error"]["code"], -32601);
}

#[test]
fn a_tool_failure_is_a_result_flagged_as_an_error() {
    let answer = ask(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"publish","arguments":{}}}"#,
    )
    .unwrap();
    assert_eq!(answer["result"]["isError"], true);
}

#[test]
fn the_topics_tool_answers_without_an_engine() {
    let answer = ask(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"topics","arguments":{"prefix":"input/key-down"}}}"#,
    )
    .unwrap();
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("input/key-down"), "{text}");
}
