/// What a route answers: a status line, a content type, a body, and any
/// headers of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Response {
    pub(crate) status: &'static str,
    pub(crate) content_type: &'static str,
    /// Bytes rather than text, because `/screenshot` answers with a PNG.
    pub(crate) body: Vec<u8>,
    /// Written after the standard headers, such as what a capture shows.
    pub(crate) headers: Vec<(&'static str, &'static str)>,
}

impl Response {
    /// A plain-text answer.
    pub(crate) fn text(status: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into().into_bytes(),
            headers: Vec::new(),
        }
    }

    /// A JSON answer, with a trailing newline for terminals.
    pub(crate) fn json(value: &serde_json::Value) -> Self {
        Self {
            status: "200 OK",
            content_type: "application/json",
            body: format!("{value}\n").into_bytes(),
            headers: Vec::new(),
        }
    }

    /// A PNG image.
    pub(crate) fn png(png: Vec<u8>) -> Self {
        Self {
            status: "200 OK",
            content_type: "image/png",
            body: png,
            headers: Vec::new(),
        }
    }

    /// The same answer with one more header.
    pub(crate) fn with_header(mut self, name: &'static str, value: &'static str) -> Self {
        self.headers.push((name, value));
        self
    }

    /// The body as text, for tests reading a text or JSON answer.
    #[cfg(test)]
    pub(crate) fn text_body(&self) -> String {
        String::from_utf8(self.body.clone()).expect("a text answer is UTF-8")
    }
}
