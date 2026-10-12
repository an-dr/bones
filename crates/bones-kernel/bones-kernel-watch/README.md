# bones-kernel-watch

Watch folders and report what changed once it settles. A commit made in a terminal, a file saved in an editor, or a `git checkout` touching thousands of files each arrive as one change per watch, not a stream of events.

A kernel library with no bus, reached through `bones-kernel`'s `watch` feature as `bones_kernel::watch`. Its consumers are native and each keeps its own topics and its own meaning for a path: the library reports that `.git/index` changed, and an application decides that means status is stale.

## API

```rust
let mut watcher = Watcher::new(DEFAULT_QUIET, |id, change| { /* post and return */ })?;
let id = watcher.watch(Path::new("/repo"), true)?;
watcher.unwatch(id);
```

| Item | Does |
| --- | --- |
| `Watcher::new(quiet, sink)` | Starts the debounce thread. `sink` is called on it once per settled burst per watch |
| `watch(path, recursive)` | Watches a folder and returns the `WatchId` its changes arrive under |
| `unwatch(id)` | Stops one watch; a change still settling for it is dropped |
| `Change` | `paths`: every path that changed during the burst, once each, in the order first seen. `kinds`: every `ChangeKind` seen, once each — a set, not parallel to `paths` |

Dropping the `Watcher` stops every watch and joins its thread; nothing is delivered after.

## Behaviour

- **Debounce.** A watch's events coalesce until it has been quiet for `quiet` — `DEFAULT_QUIET` is 200 ms — so a burst of a thousand writes arrives as a handful of changes. A burst that never pauses is reported when it does.
- **Only mutations.** Access events are dropped before debouncing, on every platform. A recursive watch's own scan reports reads on Linux, and passing them on made a consumer refresh, rescan, and refresh again (Commits' BUG-004).
- **Renames** arrive as `Removed` for the old path and `Created` for the new one. A rename the platform reports unpaired is judged by whether the path still exists.
- **Off the caller's thread.** Notifications and debouncing run on threads the `Watcher` owns. A recursive `watch` itself walks the tree on some platforms, which can take seconds for a large one, so call it off any thread that must stay responsive.

Network filesystems and polling fallbacks are out of scope: the platform's native notification is all there is.
