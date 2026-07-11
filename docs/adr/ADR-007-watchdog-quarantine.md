# ADR-007: Extension watchdog and quarantine

## Problem

A misbehaving extension — a handler that never returns, or one that floods the
bus — must not stall the frame loop or starve other extensions. The engine
needs a stated policy, not an implementation accident.

## Decision

Two budgets, enforced by the core:

- **Time budget** — every handler call runs under a per-call time budget,
  enforced by the WASM runtime's interruption mechanism.
- **Queue budget** — every extension has a bounded inbound message queue and a
  bounded per-frame publish allowance.

On violation the extension is moved to a **Faulted** state: its instance is
dropped, its subscriptions are released, and a lifecycle event is published so
other extensions and the user can react. Reloading a faulted extension is an
explicit action (user request or host policy), never an automatic silent loop.
The engine itself never dies because of an extension.

## Rationale

- Keeps the frame loop real-time regardless of extension quality.
- Faulting is observable (lifecycle event) instead of silent, which makes
  debugging third-party extensions tractable.
- Explicit reload avoids crash-loops without needing a backoff scheduler.

## Rejected alternatives

- **Kill and auto-reload** — simpler UX but invites crash-loops; would need a
  backoff policy that adds more machinery than it removes.
- **Trust extensions (no budgets)** — acceptable only while all extensions are
  first-party; contradicts the any-language extension goal.
