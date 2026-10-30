# devtools

An opt-in local channel for driving a running application from outside it: post a page message as if the page had sent it, then read back the bus traffic it caused. For end-to-end tests, and for an agent checking a change against the real application rather than a mock.

It works at the message layer because clicking does not: a webview under Xwayland ignores synthetic X input, and pixel positions drift with every layout change. Everything an interface does is a JSON message from the page to its extension, so posting that message is the click, and the recording is the screenshot.

## Security

**The channel is unauthenticated.** Anything that can reach its port can act as the user: open repositories, press any button the page has, read every message on the bus. So:

- **It is off unless asked for.** The composition root names an environment variable — the shipped engine's is `BONES_DEVTOOLS_PORT` — and the channel listens only when that variable holds a port number at startup. Unset, or not a number, the module subscribes to nothing and opens no socket. `Kernel::devtools_from_env` does not even register it then.
- **Loopback only.** It binds `127.0.0.1`, never another interface.
- **Browsers are refused.** A request with an `Origin` or `Sec-Fetch-Site` header, or a `Host` other than `localhost` or `127.0.0.1`, is dropped unanswered. That shuts out a web page posting a form to the port and DNS rebinding to a loopback address. Other local processes of any user can still connect; that is what "unauthenticated" means.
- **Never in a shipped launch.** No installer, shortcut, or launcher sets the variable, and none should. It exists for a developer's shell and a test harness.
- **Secrets can be kept out of the recording.** `Devtools::redacting(predicate)` records only the topic, sender, and length of every envelope the predicate matches — a credential prompt's answer, for example.

## Routes

| Route | Does |
| --- | --- |
| `GET /health` | `ok` |
| `POST /page-message?owner=<owner>&panel=<panel>` | Publishes the body, which must be JSON, as `web/page-message` from the `web` endpoint — indistinguishable from a click, on purpose |
| `POST /publish?topic=<topic>[&sender=<sender>]` | Publishes the body, a payload as hex, on any topic. The sender defaults to `devtools`; name `platform` to stand in for real input |
| `GET /messages[?topic=<prefix>&limit=<n>&format=json]` | Recorded bus traffic, oldest first. As text, one tab-separated line per envelope: elapsed time, topic, sender, payload preview. As JSON, an array of `{ms, topic, sender, preview, hex}`, where `hex` is the whole payload, or `null` when redacted |
| `POST /messages/clear` | Empties the recording |
| `GET /extensions` | JSON: each extension's last lifecycle state (`loaded`, `faulted`, `reloading`, `reloaded`, `stopped`), from the `core/lifecycle` events recorded since startup |
| `POST /extensions/load?name=<name>`, `/unload`, `/reload` | Publishes the runtime command; it takes effect when the composition names `devtools` as its extension controller, which the shipped engine does when the channel is on |
| `GET /logs[?category=<name>&level=<level>&limit=<n>]` | JSON: the last log lines, oldest first, as `{level, category, message}`. Info and above: the per-tick debug lines would push everything else out. Only lines logged through the composition's logger, which `Kernel::devtools_from_env` taps for the channel |
| `POST /pause`, `POST /resume` | Holds the simulation still, or lets it run again. Input, bus delivery, rendering, and this channel keep working while it is paused; a runtime extension command waits for the next step |
| `POST /step?frames=<n>` | While paused, runs `n` frames at the fixed tick period and answers once they have run, so the next read sees their effect |
| `GET /frame` | The draw commands of the last frame, one per line, grouped by the sender that published them: each sender's most recent non-empty `gfx/*` batch, which is what the renderer composites |
| `GET /screenshot[?scope=window]` | The next presented frame as a PNG, through `renderer/capture`: the canvas, or the whole window with its web panels (see below). `X-Bones-Capture-Scope` says which. Fails with `503` when no renderer answers within five seconds |
| `POST /send?endpoint=<name>` | Makes a direct `send` of the body, a payload as hex, to an endpoint or extension, as `devtools`, and answers with the reply as hex |
| `POST /exit` | Asks the application to shut down in order, as closing the window would |

```text
BONES_DEVTOOLS_PORT=8642 ./bones
curl -s localhost:8642/health
curl -s -X POST --data-binary '{"command":"refresh"}' 'localhost:8642/page-message?owner=dashboard_demo&panel=main'
curl -s 'localhost:8642/messages?topic=git/&limit=20'
```

The recording keeps the last 500 envelopes. The frame tick is kept once a second as a heartbeat rather than sixty times, so a stall shows as a gap without evicting everything else. A request is capped at 256 KiB.

## What a screenshot shows

`/screenshot` returns what the renderer drew: every `gfx/*` layer and the egui widgets above it. It works everywhere, an offscreen window included, but web panels are not in it: they are child windows the renderer never draws.

`/screenshot?scope=window` returns the whole window as it is shown, web panels included. The renderer reads it back from the system once the frame is on screen:

- **Windows** — `PrintWindow` with full-content rendering, which includes WebView2.
- **X11** — `GetImage` on the window. Under a Wayland session, web panels put the window on X11 through Xwayland, so the same path reads it.
- **Native Wayland** — the window holds nothing but the canvas, since web panels need X11, so the canvas is the window image.

The window must be visible: an offscreen engine cannot have web panels at all. Where the window cannot be read, the canvas is returned instead, and the log says why. The `X-Bones-Capture-Scope` header says which one the image shows, `canvas` or `window`.

Off by default. Enable the kernel's `devtools` feature, then `.devtools_from_env("MYAPP_DEVTOOLS_PORT")` on the builder, adding `.devtools_redacting(predicate)` to keep secrets out of the recording. A composition that builds the module itself gets `Devtools` and `LogHistory` from `bones_kernel::devtools`.

The shipped `bones` executable has the channel built in and reads `BONES_DEVTOOLS_PORT`. With it set, the engine also makes `devtools` its extension controller, so the `/extensions` routes can load, unload, and reload extensions.
