# ADR-035: Single instance is a per-user local socket, claimed before the engine

## Problem

A desktop application opened from many places — a shell, a file manager verb, another application launching it per repository — should keep one window per user. A second launch should hand its arguments to the window already open and bring it to the front, rather than start a second process with a second window.

Two facts shape where that belongs. The decision "am I the first?" must be made in the application's `main`, before `Kernel::new()` opens a window, because a second instance that has already opened a window has already failed. And the later launches the first instance receives are events its extensions and modules must see, which is bus traffic. No single tier holds both.

## Decision

Single instance is one module crate, `bones-module-single-instance`, whose two halves run at the two moments the facts name:

- **The claim and the handoff are plain functions in that crate.** `claim(app_uid, handoff, timeout)` either makes the process the instance for that application — returning a `Primary` that yields each later launch — or hands the arguments to the process that already is, and returns `HandedOff`. They use no bus and no window, so they run in `main` before the kernel exists.
- **The crate's native module delivers.** Built from a `Primary`, it publishes each later launch as `instance/launched` and raises the window.
- **Raising goes through a kernel infrastructure service, `window-control`.** The `window-surface` service is the SDL window itself, which the renderer consumes and which cannot leave the frame thread. `window-control` is a small, cloneable handle the platform provides beside it, so any module may raise the window without owning it.

The claim is a **named local endpoint scoped to the user**: a named pipe `\\.\pipe\bones-single-instance-<user-sid>-<app_uid>` on Windows, created with `FILE_FLAG_FIRST_PIPE_INSTANCE`, and a Unix domain socket named for the `app_uid` in `$XDG_RUNTIME_DIR` (or a private per-user temporary folder when that is unset) elsewhere. Holding the endpoint is holding the claim.

The `app_uid` names the application, not one run of it: every launch of one application passes the same value, which is how a later launch finds the running one and what keeps two applications built on bones apart. It is a plain string, not the self-update identity, so single instance does not depend on `bones-upgrader`.

A primary that does not acknowledge a handoff within the caller's timeout is treated as gone: the new process starts as an instance of its own. A hung window must never stop the user opening the application.

## Rationale

**The operating system already knows whether the claimant is alive.** A pipe instance disappears with the process that created it, and a socket with nothing accepting on it refuses connections. Liveness, crash recovery, and the transport for the arguments are one object rather than three, so a stale claim is detected by the same connect that would have used it.

**A generalised rendezvous directory was the alternative** — a lock or marker file per `app_uid`, the pattern Commits uses for its askpass helpers. It needs a liveness check of its own (a process id that may have been reused), a separate transport for the arguments, and cleanup after a crash. Each of those is a place for a race the socket does not have.

**No network port and no elevation.** A loopback TCP port would be reachable by every user on the machine and would need a port number nobody else picked. The pipe and the socket are per-user names in per-user namespaces.

**The claim belongs in the application, not the launcher.** A user or tool can start either executable, and the launcher may be older than the application it starts. The launcher keeps forwarding arguments unread and stays fast.

**`window-control` rather than a method on `window-surface`.** Adding `raise()` to the SDL window type is not possible — it is an alias for a foreign type — and a module holding the window would have to be the renderer. A service that carries only the capability keeps the module out of the renderer's ownership and works the same with no renderer at all.

## Consequences

- An application that wants single instance writes four lines in `main` and adds `.single_instance(primary)` to its composition.
- The claim lives in the module's crate rather than a crate of its own: every caller is a bones application that links the kernel anyway, so a second crate would add a name and nothing else.
- On Windows the handing-off process calls `AllowSetForegroundWindow` with the primary's process id, which the primary sends in the handshake; without it Windows refuses to raise another process's window.
- On Windows a client confirms the pipe's server runs as the same user before handing anything to it, so another account that creates the pipe name first cannot capture launches.
- Startup arguments are not republished. The application reads its own as it always has; the module carries only later launches.
