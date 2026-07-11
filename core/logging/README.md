# logging

Structured logging for the engine core. Every component logs through an
injected `LogSink` (ADR-012) instead of calling `println!` directly, so the
sink can be swapped without touching call sites.

- `Level` — Debug/Info/Warn/Error.
- `LogSink` — trait a sink implements; `StdoutSink` is the default.
- `Logger` — cheap-to-clone handle every component holds; forwards to its sink.
- `RecordingSink` — test double that captures calls instead of printing.
