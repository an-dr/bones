//! The JSON-RPC 2.0 messages of MCP that a tools-only server answers:
//! `initialize`, `ping`, `tools/list`, and `tools/call`.

use serde_json::{json, Value};

use super::tools::Tools;
use crate::versions;

/// The MCP revision this server speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// The answer to one line from the client, or `None` for a notification.
pub fn answer(line: &str, tools: &mut Tools) -> Option<Value> {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return Some(failure(Value::Null, -32700, &format!("not JSON: {error}"))),
    };
    // A message without an id is a notification and gets no answer.
    let id = request.get("id")?.clone();
    let method = request["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "bones", "version": versions::ENGINE },
            "instructions": "Drives a running bones engine through its devtools channel. \
                Call `topics` to see the message contract, `launch` to start the project's \
                engine, then publish input, step frames, and read messages, frame, logs, \
                or a screenshot.",
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": Tools::list() }),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or_default();
            let arguments = &request["params"]["arguments"];
            tools.call(name, arguments)
        }
        _ => return Some(failure(id, -32601, &format!("unknown method {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests;
