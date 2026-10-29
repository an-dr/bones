//! The MCP tools: each one a devtools route, with JSON fields translated to
//! payload bytes and back through the wire format specification.

use std::process::Child;

use serde_json::{json, Map, Value};

use crate::catalogue::Catalogue;
use crate::devtools_client::{DevtoolsClient, Reply};
use crate::project::Project;
use crate::{base64, dev, hex};

/// The tools, the channel they call, and the engine `launch` started.
pub struct Tools {
    client: DevtoolsClient,
    catalogue: Catalogue,
    engine: Option<Child>,
}

impl Tools {
    pub fn new() -> Self {
        Self {
            client: DevtoolsClient::from_env(),
            catalogue: Catalogue::load(),
            engine: None,
        }
    }

    /// Every tool, with its description and input schema.
    pub fn list() -> Value {
        let tool = |name: &str, description: &str, properties: Value, required: &[&str]| {
            json!({
                "name": name,
                "description": description,
                "inputSchema": { "type": "object", "properties": properties, "required": required },
            })
        };
        let string = |description: &str| json!({ "type": "string", "description": description });
        let integer = |description: &str| json!({ "type": "integer", "description": description });
        let name = json!({ "name": string("The extension's name: its .wasm file stem.") });
        json!([
            tool("topics", "List core topics with their payload layout from wit/wire-format.json: the message contract. Use before publishing a core message.", json!({ "prefix": string("Only topics starting with this, such as gfx/ or input/.") }), &[]),
            tool("launch", "Build the project in the current directory and start its engine with devtools open. Waits until it answers.", json!({
                "engine": string("Path of the bones executable, when it is not beside bones-cli."),
                "offscreen": { "type": "boolean", "description": "Draw in memory rather than in a visible window; screenshot still works. For machines without a display." },
            }), &[]),
            tool("stop", "Stop the engine launch started.", json!({}), &[]),
            tool("publish", "Publish a message on the bus. Give fields for a core topic, or hex for any other topic. A union takes its tag as a field beside the variant's fields; a list is an array, without its count.", json!({
                "topic": string("The topic, such as input/key-down."),
                "fields": { "type": "object", "description": "Field values by name, for a core topic." },
                "hex": string("The payload as hex, for any other topic."),
                "sender": string("Sender name; platform for input, devtools otherwise."),
            }), &["topic"]),
            tool("send", "Make a direct send to an endpoint or extension and return its reply as text when it is UTF-8, else as hex.", json!({
                "endpoint": string("The endpoint or extension name, such as calculator."),
                "text": string("The payload as UTF-8 text, such as a JSON request."),
                "hex": string("The payload as hex, for binary payloads."),
            }), &["endpoint"]),
            tool("messages", "Recent bus traffic, oldest first, with payloads decoded into fields where the catalogue allows.", json!({ "topic": string("Only topics starting with this."), "limit": integer("At most this many of the newest.") }), &[]),
            tool("extensions", "Each extension's state: loaded, faulted, reloading, reloaded, or stopped.", json!({}), &[]),
            tool("load", "Load a cataloged extension.", name.clone(), &["name"]),
            tool("unload", "Unload a running extension.", name.clone(), &["name"]),
            tool("reload", "Reload an extension from its file.", name, &["name"]),
            tool("logs", "Recent log lines at info and above.", json!({ "category": string("An extension's name, engine, or a module."), "level": string("info, warn, or error."), "limit": integer("At most this many of the newest.") }), &[]),
            tool("frame", "The last frame's draw commands, one per line, by sender.", json!({}), &[]),
            tool("screenshot", "The next presented frame as an image. The canvas by default; scope window captures the whole visible window, web panels included, and says which one it returned.", json!({ "scope": string("canvas (default) or window.") }), &[]),
            tool("pause", "Hold the simulation still; rendering and the bus keep running.", json!({}), &[]),
            tool("resume", "Let the simulation run again.", json!({}), &[]),
            tool("step", "While paused, run this many frames at the fixed tick period, then answer.", json!({ "frames": integer("How many frames; 1 when omitted.") }), &[]),
        ])
    }

    /// Runs one tool; a failure is a result flagged `isError`, as MCP asks.
    pub fn call(&mut self, name: &str, arguments: &Value) -> Value {
        match self.run(name, arguments) {
            Ok(content) => json!({ "content": content }),
            Err(error) => json!({ "content": [text(&error)], "isError": true }),
        }
    }

    fn run(&mut self, name: &str, arguments: &Value) -> Result<Vec<Value>, String> {
        let argument = |key: &str| arguments.get(key).and_then(Value::as_str);
        match name {
            "topics" => Ok(vec![text(
                &self
                    .catalogue
                    .describe(argument("prefix").unwrap_or(""))
                    .to_string(),
            )]),
            "launch" => self.launch(
                argument("engine"),
                arguments["offscreen"].as_bool().unwrap_or(false),
            ),
            "stop" => {
                self.stop_engine();
                Ok(vec![text("stopped")])
            }
            "send" => {
                let endpoint = argument("endpoint").ok_or("endpoint is required")?;
                let payload = match (argument("text"), argument("hex")) {
                    (Some(text), _) => text.as_bytes().to_vec(),
                    (None, Some(hex)) => {
                        hex::parse(hex).ok_or("hex must be two digits per byte")?
                    }
                    (None, None) => Vec::new(),
                };
                let reply = self.expect_ok(self.client.request(
                    "POST",
                    &format!("/send?endpoint={endpoint}"),
                    hex::format(&payload).as_bytes(),
                )?)?;
                let bytes = hex::parse(reply.text().trim()).unwrap_or_default();
                Ok(vec![text(&match String::from_utf8(bytes.clone()) {
                    Ok(utf8) => utf8,
                    Err(_) => hex::format(&bytes),
                })])
            }
            "publish" => self.publish(arguments),
            "messages" => self.messages(
                argument("topic"),
                arguments.get("limit").and_then(Value::as_u64),
            ),
            "extensions" => self.get_text("/extensions"),
            "load" | "unload" | "reload" => {
                let extension = argument("name").ok_or("name is required")?;
                self.post_text(&format!("/extensions/{name}?name={extension}"))
            }
            "logs" => {
                let mut query = vec![];
                for key in ["category", "level"] {
                    if let Some(value) = argument(key) {
                        query.push(format!("{key}={value}"));
                    }
                }
                if let Some(limit) = arguments.get("limit").and_then(Value::as_u64) {
                    query.push(format!("limit={limit}"));
                }
                self.get_text(&format!("/logs?{}", query.join("&")))
            }
            "frame" => self.get_text("/frame"),
            "screenshot" => {
                let scope = argument("scope").unwrap_or("canvas");
                let target = format!("/screenshot?scope={scope}");
                let reply = self.expect_ok(self.client.request("GET", &target, b"")?)?;
                let shows = reply.header("x-bones-capture-scope").unwrap_or("canvas");
                Ok(vec![
                    json!({ "type": "image", "data": base64::encode(&reply.body), "mimeType": "image/png" }),
                    text(&format!("captured the {shows}")),
                ])
            }
            "pause" => self.post_text("/pause"),
            "resume" => self.post_text("/resume"),
            "step" => {
                let frames = arguments.get("frames").and_then(Value::as_u64).unwrap_or(1);
                self.post_text(&format!("/step?frames={frames}"))
            }
            _ => Err(format!("no tool named {name}; tools/list names them all")),
        }
    }

    fn launch(&mut self, engine: Option<&str>, offscreen: bool) -> Result<Vec<Value>, String> {
        self.stop_engine();
        let project = Project::find()?;
        project.build()?;
        let window = match offscreen {
            true => dev::Window::Offscreen,
            false => dev::Window::Visible,
        };
        let port = self.client.port.to_string();
        let mut child = dev::start(&project, engine, &port, dev::Output::StderrOnly, window)?;
        let ready = dev::wait_until_ready(&project, &self.client, &mut child);
        self.engine = Some(child);
        ready?;
        Ok(vec![text(&format!(
            "{} is running with devtools on port {}",
            project.name, self.client.port
        ))])
    }

    /// Stops the engine `launch` started, if it still runs.
    pub fn stop_engine(&mut self) {
        if let Some(engine) = self.engine.take() {
            dev::stop(&self.client, engine);
        }
    }

    fn publish(&self, arguments: &Value) -> Result<Vec<Value>, String> {
        let topic = arguments["topic"].as_str().ok_or("topic is required")?;
        let payload = match (arguments.get("fields"), arguments.get("hex")) {
            (Some(fields), _) => {
                let empty = Map::new();
                self.catalogue
                    .encode(topic, fields.as_object().unwrap_or(&empty))?
            }
            (None, Some(hex)) => hex::parse(hex.as_str().unwrap_or_default())
                .ok_or("hex must be two digits per byte")?,
            (None, None) => self.catalogue.encode(topic, &Map::new())?,
        };
        let sender = match arguments["sender"].as_str() {
            Some(sender) => sender.to_string(),
            // Input is published by the platform; an extension filtering
            // on the sender then treats this as real input.
            None if topic.starts_with("input/") => "platform".to_string(),
            None => "devtools".to_string(),
        };
        self.post_body(
            &format!("/publish?topic={topic}&sender={sender}"),
            hex::format(&payload).as_bytes(),
        )
    }

    fn messages(&self, topic: Option<&str>, limit: Option<u64>) -> Result<Vec<Value>, String> {
        let mut target = String::from("/messages?format=json");
        if let Some(topic) = topic {
            target.push_str(&format!("&topic={topic}"));
        }
        target.push_str(&format!("&limit={}", limit.unwrap_or(50)));
        let reply = self.expect_ok(self.client.request("GET", &target, b"")?)?;
        let mut list: Value = serde_json::from_slice(&reply.body)
            .map_err(|error| format!("unreadable messages: {error}"))?;
        for message in list.as_array_mut().into_iter().flatten() {
            let decoded = message["hex"]
                .as_str()
                .and_then(hex::parse)
                .and_then(|bytes| {
                    self.catalogue
                        .decode(message["topic"].as_str().unwrap_or_default(), &bytes)
                });
            if let Some(fields) = decoded {
                message["fields"] = fields;
            }
        }
        Ok(vec![text(&list.to_string())])
    }

    fn get_text(&self, target: &str) -> Result<Vec<Value>, String> {
        let reply = self.expect_ok(self.client.request("GET", target, b"")?)?;
        Ok(vec![text(&reply.text())])
    }

    fn post_text(&self, target: &str) -> Result<Vec<Value>, String> {
        self.post_body(target, b"")
    }

    fn post_body(&self, target: &str, body: &[u8]) -> Result<Vec<Value>, String> {
        let reply = self.expect_ok(self.client.request("POST", target, body)?)?;
        Ok(vec![text(&reply.text())])
    }

    /// The reply when the route succeeded, its own message otherwise.
    fn expect_ok(&self, reply: Reply) -> Result<Reply, String> {
        match reply.status {
            200..=299 => Ok(reply),
            status => Err(format!(
                "devtools answered {status}: {}",
                reply.text().trim()
            )),
        }
    }
}

impl Drop for Tools {
    fn drop(&mut self) {
        self.stop_engine();
    }
}

/// An MCP text content block.
fn text(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}
