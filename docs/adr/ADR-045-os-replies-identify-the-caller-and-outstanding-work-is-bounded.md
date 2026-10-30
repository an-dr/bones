# ADR-045: OS replies identify the caller and outstanding work is bounded

## Problem

Independent extensions can reuse request IDs, but the legacy OS result does not identify the requester. Detached work per request also lets blocked dialogs accumulate across frames.

## Decision

- Add an OS reply topic containing the host-stamped requester and the existing result fields. Keep the legacy request and result payloads unchanged; publish both reply formats for compatibility.
- Callers match their name and outstanding request ID. Replies are broadcast, not a confidentiality boundary.
- Admit at most 16 active OS requests per module with no waiting queue. Refuse excess work with an actionable busy error.
- Close admission and suppress late replies at shutdown without waiting for backend operations that may require user interaction.

## Rationale

Caller-scoped correlation lets independently developed extensions compose without a global ID allocator. An additive topic preserves compiled extensions. Bounded admission limits outstanding work while keeping the frame thread responsive. Shutdown cannot safely cancel arbitrary native backend calls.

## Rejected alternatives

- Changing the existing result payload breaks compiled guests.
- Global caller coordination makes independent extensions depend on application-specific conventions.
- An unbounded worker queue still permits memory growth.
- Joining dialog workers at shutdown can prevent the application from exiting.
