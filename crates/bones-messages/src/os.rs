//! `os/*`: the desktop capabilities a WASM extension cannot reach on its own.
//!
//! An extension has no OS API — no clipboard, no browser, no native file
//! dialog — because the sandbox that makes it safe to load also cuts it off
//! from the machine. This module is how it asks the trusted `os` native
//! module to act on its behalf, the same module-vs-extension trust split
//! `persistence/*` rests on.
//!
//! Every request carries a caller-chosen id, and the reply repeats it, so a
//! guest with several requests in flight can tell the answers apart. The
//! native module answers off the bus thread — a file dialog blocks until the
//! user chooses, which must not stall the engine — so replies arrive in
//! whatever order the work finishes, never necessarily the order asked.
//!
//! A host is free to offer only some actions: an unimplemented one answers
//! with [`Result::error`] rather than failing to answer, so a guest never
//! waits forever on a capability this host does not have.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// The bus endpoint name the `os/*` native module registers under.
pub const ENDPOINT: &str = "os";

/// What the caller wants done.
///
/// Tags are part of the wire format: add new actions at the end, and never
/// renumber an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Reads the system clipboard's text. `value` is ignored.
    ReadClipboard,
    /// Replaces the system clipboard's text with `value`.
    WriteClipboard,
    /// Opens `value` in the user's browser. A host is expected to refuse a
    /// scheme it does not trust rather than hand an arbitrary string to the
    /// shell.
    OpenUrl,
    /// Asks the user to choose one file, with `value` as the dialog title.
    PickFile,
    /// Asks the user to choose one folder, with `value` as the dialog title.
    PickFolder,
    /// Reveals the directory `value` in the desktop's file manager.
    OpenDirectory,
    /// Fetches `value` over HTTPS, answering with
    /// `"{content-type};base64,{data}"` — a self-describing shape the caller
    /// can turn into a data URI or decode directly. Intended for small payloads
    /// such as an avatar or a manifest, never a bulk download; a host caps
    /// the body size and the wait.
    FetchUrl,
}

impl Action {
    fn tag(self) -> u8 {
        match self {
            Action::ReadClipboard => 0,
            Action::WriteClipboard => 1,
            Action::OpenUrl => 2,
            Action::PickFile => 3,
            Action::PickFolder => 4,
            Action::OpenDirectory => 5,
            Action::FetchUrl => 6,
        }
    }

    fn from_tag(tag: u8) -> core::result::Result<Self, DecodeError> {
        match tag {
            0 => Ok(Action::ReadClipboard),
            1 => Ok(Action::WriteClipboard),
            2 => Ok(Action::OpenUrl),
            3 => Ok(Action::PickFile),
            4 => Ok(Action::PickFolder),
            5 => Ok(Action::OpenDirectory),
            6 => Ok(Action::FetchUrl),
            tag => Err(DecodeError::InvalidTag {
                message: "os action",
                tag,
            }),
        }
    }
}

/// Asks the `os` module to perform one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request<'a> {
    /// Echoed back on the matching [`Result`], so several requests can be in
    /// flight at once.
    pub request_id: u32,
    pub action: Action,
    /// The action's one argument: a URL, a path, a dialog title, or the text
    /// to copy. Empty for an action that takes none.
    pub value: &'a str,
}

impl Message for Request<'_> {
    const TOPIC: &'static str = "os/request";
}

impl EncodeMessage for Request<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(self.action.tag())
            .str(self.value)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Request<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let action = Action::from_tag(reader.read_u8()?)?;
        let value = reader.read_str()?;
        reader.finish()?;
        Ok(Self {
            request_id,
            action,
            value,
        })
    }
}

/// One request's outcome.
///
/// `accepted` separates "there is no answer" from "the answer is empty",
/// which the two string fields cannot do alone. A cancelled file dialog, a
/// URL holding nothing, and a missing file are all ordinary outcomes: they
/// answer `accepted: false` with an empty `error`. Only a genuine failure
/// carries `error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Result<'a> {
    pub request_id: u32,
    pub accepted: bool,
    pub value: &'a str,
    pub error: &'a str,
}

impl Message for Result<'_> {
    const TOPIC: &'static str = "os/result";
}

impl EncodeMessage for Result<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.request_id)
            .u8(u8::from(self.accepted))
            .str(self.value)
            .str(self.error)
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Result<'a> {
    fn decode(payload: &'a [u8]) -> core::result::Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let request_id = reader.read_u32()?;
        let accepted = match reader.read_u8()? {
            0 => false,
            1 => true,
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "os result accepted",
                    tag,
                })
            }
        };
        let value = reader.read_str()?;
        let error = reader.read_str()?;
        reader.finish()?;
        Ok(Self {
            request_id,
            accepted,
            value,
            error,
        })
    }
}

#[cfg(test)]
mod tests;
