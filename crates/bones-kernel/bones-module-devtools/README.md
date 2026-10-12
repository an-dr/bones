# devtools

An opt-in local channel for driving a running application from outside it: post a page message as if the page had sent it, then read back the bus traffic it caused. For end-to-end tests, and for an agent checking a change against the real application rather than a mock.

It works at the message layer because clicking does not: a webview under Xwayland ignores synthetic X input, and pixel positions drift with every layout change. Everything an interface does is a JSON message from the page to its extension, so posting that message is the click, and the recording is the screenshot.

## Security

**The channel is unauthenticated.** Anything that can reach its port can act as the user: open repositories, press any button the page has, read every message on the bus. So:

- **It is off unless asked for.** The composition root names an environment variable — `COMMITS_DEVTOOLS_PORT`, say — and the channel listens only when that variable holds a port number at startup. Unset, or not a number, the module subscribes to nothing and opens no socket. `Kernel::devtools_from_env` does not even register it then.
- **Loopback only.** It binds `127.0.0.1`, never another interface.
- **Browsers are refused.** A request with an `Origin` or `Sec-Fetch-Site` header, or a `Host` other than `localhost` or `127.0.0.1`, is dropped unanswered. That shuts out a web page posting a form to the port and DNS rebinding to a loopback address. Other local processes of any user can still connect; that is what "unauthenticated" means.
- **Never in a shipped launch.** No installer, shortcut, or launcher sets the variable, and none should. It exists for a developer's shell and a test harness.
- **Secrets can be kept out of the recording.** `Devtools::redacting(predicate)` records only the topic, sender, and length of every envelope the predicate matches — a credential prompt's answer, for example.

## Routes

| Route | Does |
| --- | --- |
| `GET /health` | `ok` |
| `POST /page-message?owner=<owner>&panel=<panel>` | Publishes the body, which must be JSON, as `web/page-message` from the `web` endpoint — indistinguishable from a click, on purpose |
| `GET /messages[?topic=<prefix>&limit=<n>]` | Recorded bus traffic, oldest first, one tab-separated line per envelope: elapsed time, topic, sender, payload preview |
| `POST /messages/clear` | Empties the recording |

```text
COMMITS_DEVTOOLS_PORT=8642 ./commits
curl -s localhost:8642/health
curl -s -X POST --data-binary '{"command":"refresh"}' 'localhost:8642/page-message?owner=commits&panel=main'
curl -s 'localhost:8642/messages?topic=git/&limit=20'
```

The recording keeps the last 500 envelopes. The frame tick is kept once a second as a heartbeat rather than sixty times, so a stall shows as a gap without evicting everything else. A request is capped at 256 KiB.

## No screenshot route

A `GET /screenshot` route was considered and left out: wry has no API that captures a panel's pixels on both Windows and Linux. WebView2's capture and WebKitGTK's snapshot are separate platform calls with different threading rules, and neither is reachable through wry. Capture the window with the operating system's own tools instead.

Off by default. Enable the kernel's `devtools` feature, then `.devtools_from_env("MYAPP_DEVTOOLS_PORT")` on the builder, or `.module(Devtools::from_env(..).redacting(..))` for redaction.
