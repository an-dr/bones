use std::io::Read;

/// Largest request accepted, which bounds a page message posted through it.
const MAX_REQUEST: usize = 256 * 1024;

/// One HTTP request, as far as the channel cares: its request line and body.
pub(crate) struct Request {
    pub(crate) line: String,
    pub(crate) body: String,
}

impl Request {
    /// Reads until the body announced by `Content-Length` has arrived, or
    /// `None` for anything the channel refuses to serve.
    ///
    /// - A posted message is a second packet more often than not, so one
    ///   `read` is not enough.
    /// - Browser requests are refused — an `Origin` or `Sec-Fetch-Site`
    ///   header, or a `Host` other than loopback — which shuts out cross-site
    ///   form posts and DNS rebinding to a loopback address.
    /// - Ambiguous framing is refused: two lengths, a chunked body, or a body
    ///   over the cap.
    pub(crate) fn read(stream: &mut impl Read) -> Option<Self> {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 8192];
        let headers_end = loop {
            if let Some(end) = find(&buffer, b"\r\n\r\n") {
                break end + 4;
            }
            if buffer.len() > MAX_REQUEST {
                return None;
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return None,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        };
        let head = String::from_utf8_lossy(&buffer[..headers_end]).into_owned();
        if headers_end > MAX_REQUEST {
            return None;
        }
        let mut length = None;
        for line in head.lines().skip(1).filter(|line| !line.is_empty()) {
            let (name, value) = line.split_once(':')?;
            let value = value.trim();
            if name.eq_ignore_ascii_case("origin") || name.eq_ignore_ascii_case("sec-fetch-site") {
                return None;
            }
            if name.eq_ignore_ascii_case("host") {
                let (host, port) = value.split_once(':').unwrap_or((value, ""));
                if !(host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1")
                    || (!port.is_empty() && port.parse::<u16>().is_err())
                {
                    return None;
                }
            }
            if name.eq_ignore_ascii_case("transfer-encoding") {
                return None;
            }
            if name.eq_ignore_ascii_case("content-length") {
                if length.is_some() {
                    return None;
                }
                length = Some(value.parse::<usize>().ok()?);
            }
        }
        let length = length.unwrap_or(0);
        if length > MAX_REQUEST - headers_end {
            return None;
        }
        let end = headers_end + length;
        while buffer.len() < end {
            let remaining = (end - buffer.len()).min(chunk.len());
            match stream.read(&mut chunk[..remaining]) {
                Ok(0) | Err(_) => return None,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        }
        Some(Self {
            line: head.lines().next().unwrap_or_default().to_string(),
            body: String::from_utf8(buffer[headers_end..end].to_vec()).ok()?,
        })
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
