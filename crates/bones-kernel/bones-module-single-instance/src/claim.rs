use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::attempt::Attempt;
use crate::{app_uid, platform, wire, Handoff, Primary};

/// How [`claim`] ended.
#[derive(Debug)]
pub enum Claim {
    /// This process is the instance; start the application.
    Primary(Primary),
    /// The running instance has the arguments; exit with status 0.
    HandedOff,
}

/// How many times a claim is retried after finding an endpoint whose owner
/// has just gone away. Each retry is one bind and one connect, so this only
/// bounds a pathological race, never a normal start.
const STALE_RETRIES: usize = 3;

/// Becomes the instance for `app_uid`, or hands `handoff` to the process that is.
///
/// - `app_uid` names the application, not one run of it: every launch of the
///   same application passes the same value, which is how a later launch
///   finds the running one, and two applications built on bones pass
///   different ones so they never interfere.
/// - `app_uid` is scoped to the current user: two users each get an instance
///   of their own.
/// - An instance that does not acknowledge within `timeout` is treated as
///   hung, and this process starts as an instance of its own. Applications
///   pass 2 s: a hung window must never stop the user opening the app.
/// - An endpoint left behind by a crashed instance is detected and taken
///   over, never trusted.
///
/// `Err` is reserved for this process being unable to take part at all — an
/// invalid app_uid, or a runtime directory it cannot use — and is worth
/// reporting; a caller may still start normally.
pub fn claim(app_uid: &str, handoff: Handoff, timeout: Duration) -> Result<Claim, String> {
    app_uid::validate(app_uid)?;
    let deadline = Instant::now() + timeout;
    for _ in 0..STALE_RETRIES {
        match platform::attempt(app_uid, timeout)? {
            Attempt::Claimed(listener) => {
                let (sender, incoming) = mpsc::channel();
                let guard = listener.start(sender)?;
                return Ok(Claim::Primary(Primary::listening(incoming, guard)));
            }
            Attempt::Stale => continue,
            Attempt::Taken(stream) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                return Ok(match hand_off_within(stream, handoff, remaining) {
                    Ok(()) => Claim::HandedOff,
                    Err(_) => Claim::Primary(Primary::detached()),
                });
            }
            Attempt::Unavailable => break,
        }
    }
    Ok(Claim::Primary(Primary::detached()))
}

/// Runs the handshake on its own thread, so an instance that accepted the
/// connection and then stopped answering costs this launch `timeout` and
/// nothing more. The thread is left to finish or fail on its own.
fn hand_off_within(
    mut stream: platform::Stream,
    handoff: Handoff,
    timeout: Duration,
) -> Result<(), String> {
    let (done, outcome) = mpsc::channel();
    std::thread::Builder::new()
        .name("bones-single-instance-handoff".into())
        .spawn(move || {
            let result = wire::hand_off(&mut stream, &handoff, platform::allow_foreground);
            let _ = done.send(result);
        })
        .map_err(|error| error.to_string())?;
    outcome
        .recv_timeout(timeout)
        .map_err(|_| String::from("the running instance did not answer in time"))?
}
