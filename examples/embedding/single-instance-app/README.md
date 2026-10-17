# single-instance-app

An application that keeps one window per user. Starting it a second time does not open a second window: the new process hands its arguments and working directory to the one already running and exits, and the running one receives them as `instance/launched` and comes to the front ([ADR-035](../../../docs/adr/ADR-035-single-instance-is-a-per-user-local-socket-claimed-before-the-engine.md)).

```sh
cargo xtask example single-instance-app
```

Then start the binary the command names, and start it again from another shell:

```text
$ single-instance-app
this is the instance; start the app again to hand it arguments
launched again from C:\work: ["--x", "y"]      <- printed when the second start arrives

$ single-instance-app --x y                     # from C:\work
handed off to the running instance              <- exits with status 0 at once
```

## The whole of it

```rust
let handoff = Handoff { args: std::env::args().skip(1).collect(), cwd: std::env::current_dir()? };
match claim("myapp", handoff, Duration::from_secs(2))? {
    Claim::HandedOff => return Ok(()),
    Claim::Primary(primary) => Kernel::new().window("myapp", 800, 600).single_instance(primary).run()?,
}
```

- **The claim runs first,** in `main`, before the engine opens a window. A second instance that had already opened one would be too late.
- **The `app_uid` names the application.** Every launch of the same application passes the same one; another application, with its own `app_uid`, never interferes. It is scoped per user, so another account runs its own instance.
- **A hung instance never blocks a start.** If the running instance does not acknowledge within the timeout — 2 s here — the new process starts as an instance of its own.
- **A crashed instance leaves nothing behind** that a later start has to trust: the next start becomes the instance.
- **Startup arguments are not republished.** The application reads its own; `instance/launched` carries only later launches. Here `LaunchPrinter` stands in for what a real application does with one — open the file, switch to the repository.

The `app_uid` here is a literal. A real application passes the application's configured executable stem, so it follows the install identity.
