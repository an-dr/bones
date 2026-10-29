//! A minimal HTTP client for the engine's loopback devtools channel.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The variable naming the channel's port, as the engine reads it.
pub const PORT_VAR: &str = "BONES_DEVTOOLS_PORT";
/// The port `bones-cli dev` and `launch` open when none is given.
pub const DEFAULT_PORT: u16 = 8642;

/// Longer than the channel's own five-second waits for `/step` and
/// `/screenshot`, so their answer arrives rather than a timeout here.
const TIMEOUT: Duration = Duration::from_secs(15);

/// An answer from the channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub content_type: String,
    /// Every header, names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The value of header `name`, given in lower case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The channel on one port of `127.0.0.1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevtoolsClient {
    pub port: u16,
}

impl DevtoolsClient {
    /// The port `BONES_DEVTOOLS_PORT` names, or the default.
    pub fn from_env() -> Self {
        let port = std::env::var(PORT_VAR)
            .ok()
            .and_then(|port| port.trim().parse().ok())
            .unwrap_or(DEFAULT_PORT);
        Self { port }
    }

    /// Sends one request and reads the whole answer.
    pub fn request(&self, method: &str, target: &str, body: &[u8]) -> Result<Reply, String> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).map_err(|error| {
            format!(
                "no devtools channel on port {}: {error}; start the engine with the launch tool or bones-cli dev",
                self.port
            )
        })?;
        stream.set_read_timeout(Some(TIMEOUT)).ok();
        let head = format!(
            "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .and_then(|()| stream.write_all(body))
            .map_err(|error| format!("writing to the devtools channel: {error}"))?;
        let mut answer = Vec::new();
        stream
            .read_to_end(&mut answer)
            .map_err(|error| format!("reading from the devtools channel: {error}"))?;
        parse_reply(&answer)
            .ok_or_else(|| "the devtools channel sent a malformed answer".to_string())
    }
}

/// Splits a `Connection: close` HTTP answer into status, type, and body.
fn parse_reply(answer: &[u8]) -> Option<Reply> {
    let end = answer.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&answer[..end]).ok()?;
    let mut lines = head.lines();
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    let content_type = headers
        .iter()
        .find(|(name, _)| name == "content-type")
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    Some(Reply {
        status,
        content_type,
        headers,
        body: answer[end + 4..].to_vec(),
    })
}

#[cfg(test)]
mod tests;
