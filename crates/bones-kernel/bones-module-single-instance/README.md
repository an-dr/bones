# single-instance

One instance per user for a desktop application. A second launch hands its arguments and working directory to the process already running and exits, instead of opening a second window ([ADR-035](../../../docs/adr/ADR-035-single-instance-is-a-per-user-local-socket-claimed-before-the-engine.md)).

The crate has two halves that run at different moments:

- **The claim, in `main`.** `claim` decides whether this process is the instance. It uses no bus and no window, so it runs before `Kernel::new()`: a second instance that has opened a window has already failed.
- **Later launches, on the bus.** `InstanceModule` carries each handoff the running instance receives onto the bus and raises the window.

Off by default. Enable the kernel's `instance` feature: the claim is `bones_kernel::instance::claim`, and `.single_instance(primary)` adds the module.

## The claim


```rust
use std::time::Duration;
use bones_kernel::instance::{claim, Claim, Handoff};

let handoff = Handoff { args: std::env::args().skip(1).collect(), cwd: std::env::current_dir()? };
match claim("myapp", handoff, Duration::from_secs(2))? {
    Claim::HandedOff => return Ok(()),
    Claim::Primary(primary) => Kernel::new().window("myapp", 800, 600).web().single_instance(primary).run()?,
}
```

- **The `app_uid`** identifies the application, not one run of it: every launch of the same application passes the same value, which is how a later launch finds the running one, and it is what keeps two applications built on bones apart. It is 1 to 64 ASCII letters, digits, `.`, `_` or `-`, and scoped to the current user by the crate. An application normally passes the application's configured executable stem.
- **`Primary::incoming()`** yields one `Handoff` per later launch, in arrival order. Startup arguments are not on it; the application reads its own.
- **Dropping the `Primary`** releases the claim; the next launch becomes the instance.
- **`Err`** means this process could not take part at all: an invalid `app_uid`, or a runtime folder it cannot use. Starting normally is still a valid response.

### Behaviour

| Situation | Outcome |
| --- | --- |
| Nobody holds the `app_uid` | `Primary`, listening |
| A live instance holds it | The handoff is delivered and acknowledged; `HandedOff` |
| The instance crashed | Its endpoint is gone (Windows) or refuses connections (Unix); taken over as `Primary` |
| The instance does not acknowledge within `timeout` | `Primary` that is not listening: the application starts as an instance of its own |
| Another account created the pipe name (Windows) | `Primary` that is not listening; nothing is handed to it |

### Mechanism

| Platform | Endpoint |
| --- | --- |
| Windows | Named pipe `\\.\pipe\bones-single-instance-<user-sid>-<app_uid>`; its first instance is the claim, created with `FILE_FLAG_FIRST_PIPE_INSTANCE` and refusing remote clients |
| Linux, macOS | Unix domain socket `bones-single-instance-<app_uid>.sock` in `$XDG_RUNTIME_DIR`, or in a private `bones-single-instance-<uid>` folder under the temporary directory when that is unset; a `.lock` file beside it serialises stale-claim recovery |

The handshake: the instance sends its process id, the launch sends one length-prefixed frame of the working directory and the arguments, and the instance acknowledges once the handoff is queued. On Windows the launch calls `AllowSetForegroundWindow` with that process id before sending, because Windows otherwise refuses to raise another process's window. A frame is capped at 1 MiB.

A working directory that is not valid UTF-8 is passed lossily.

macOS compiles and is untested; it is not a release target.

## Later launches, on the bus

The module is built from the `Primary` the claim returned:

- Once per frame it drains the launches that handed their arguments over, without blocking.
- Each becomes `instance/launched` from the `instance` endpoint, carrying the arguments and the launching process's working directory ([wit/wire-format.md](../../../wit/wire-format.md)).
- After publishing, it raises the window through the `window-control` service: un-minimise, raise, and focus where the platform allows. A headless composition has no such service and only publishes.

The application's startup arguments are never published; it reads its own.
