//! The handshake between a launch and the running instance.
//!
//! 1. The instance writes its process id (`u32`, little-endian), so the
//!    launch can grant it the foreground on Windows before handing over.
//! 2. The launch writes one frame: a `u32` byte length, then the working
//!    directory and each argument as `u32`-length-prefixed UTF-8, preceded
//!    by a `u32` argument count.
//! 3. The instance writes one byte, [`ACK`], once the handoff is queued.
//!
//! Both ends are processes of the same user, but a frame is still bounded,
//! so a confused client cannot make the instance allocate without limit.

use std::io::{Read, Write};
use std::path::PathBuf;

use crate::Handoff;

/// The byte that acknowledges a queued handoff.
pub(crate) const ACK: u8 = 1;

/// Largest frame accepted: far beyond any real command line.
const MAX_FRAME: u32 = 1024 * 1024;

/// The instance's side: greet, read one handoff, and acknowledge it once
/// `deliver` accepted it.
pub(crate) fn serve(
    stream: &mut (impl Read + Write),
    deliver: impl FnOnce(Handoff) -> bool,
) -> Result<(), String> {
    stream
        .write_all(&std::process::id().to_le_bytes())
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    let handoff = read_frame(stream)?;
    if deliver(handoff) {
        stream
            .write_all(&[ACK])
            .map_err(|error| error.to_string())?;
        stream.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// The launch's side: read the greeting, let `greeted` act on the
/// instance's process id, send the handoff, and wait for the acknowledgement.
pub(crate) fn hand_off(
    stream: &mut (impl Read + Write),
    handoff: &Handoff,
    greeted: impl FnOnce(u32),
) -> Result<(), String> {
    let mut pid = [0; 4];
    stream
        .read_exact(&mut pid)
        .map_err(|error| format!("reading the instance's greeting: {error}"))?;
    greeted(u32::from_le_bytes(pid));
    stream
        .write_all(&encode(handoff)?)
        .map_err(|error| format!("sending the handoff: {error}"))?;
    stream.flush().map_err(|error| error.to_string())?;
    let mut ack = [0; 1];
    stream
        .read_exact(&mut ack)
        .map_err(|error| format!("reading the acknowledgement: {error}"))?;
    if ack[0] != ACK {
        return Err(format!("unexpected acknowledgement byte {}", ack[0]));
    }
    Ok(())
}

pub(crate) fn encode(handoff: &Handoff) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let cwd = handoff.cwd.to_string_lossy();
    push_str(&mut body, &cwd);
    body.extend_from_slice(&(handoff.args.len() as u32).to_le_bytes());
    for arg in &handoff.args {
        push_str(&mut body, arg);
    }
    let len = u32::try_from(body.len())
        .ok()
        .filter(|len| *len <= MAX_FRAME)
        .ok_or_else(|| format!("the launch's arguments exceed {MAX_FRAME} bytes"))?;
    let mut frame = len.to_le_bytes().to_vec();
    frame.extend(body);
    Ok(frame)
}

fn read_frame(stream: &mut impl Read) -> Result<Handoff, String> {
    let mut len = [0; 4];
    stream
        .read_exact(&mut len)
        .map_err(|error| format!("reading a handoff: {error}"))?;
    let len = u32::from_le_bytes(len);
    if len > MAX_FRAME {
        return Err(format!("a handoff of {len} bytes exceeds {MAX_FRAME}"));
    }
    let mut body = vec![0; len as usize];
    stream
        .read_exact(&mut body)
        .map_err(|error| format!("reading a handoff: {error}"))?;
    decode(&body)
}

pub(crate) fn decode(body: &[u8]) -> Result<Handoff, String> {
    let mut rest = body;
    let cwd = take_str(&mut rest)?;
    let count = take_u32(&mut rest)?;
    let mut args = Vec::new();
    for _ in 0..count {
        args.push(take_str(&mut rest)?);
    }
    if !rest.is_empty() {
        return Err("a handoff carried trailing bytes".into());
    }
    Ok(Handoff {
        args,
        cwd: PathBuf::from(cwd),
    })
}

fn push_str(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn take_u32(rest: &mut &[u8]) -> Result<u32, String> {
    let (head, tail) = rest
        .split_first_chunk::<4>()
        .ok_or("a handoff ended early")?;
    *rest = tail;
    Ok(u32::from_le_bytes(*head))
}

fn take_str(rest: &mut &[u8]) -> Result<String, String> {
    let len = take_u32(rest)? as usize;
    if rest.len() < len {
        return Err("a handoff ended early".into());
    }
    let (head, tail) = rest.split_at(len);
    *rest = tail;
    String::from_utf8(head.to_vec()).map_err(|_| "a handoff was not UTF-8".into())
}
