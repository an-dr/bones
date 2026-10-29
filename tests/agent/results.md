# Agent benchmark results

One row per run; see [README.md](README.md) for the procedure. Acceptance is passing tests over total. Interventions count every time a person had to act, other than starting the run.

| Date | Engine commit | Agent and model | Task | Acceptance | Interventions | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-10-29 | 849d9fd | Claude Code general-purpose agent, Claude Opus 5.5 | pong | 6/6 | 0 | 11 own checks; used devtools /publish, /frame, and a screenshot. Asked for key names, Rust field shapes (color tuple, TextAlign), and step timing |
| 2026-10-29 | 849d9fd | Claude Code general-purpose agent, Claude Opus 5.5 | notes | 4/4 | 0 | Verified a restart in the real engine. Headless persistence location and isolation were undocumented; a devtools port clash with another run misled a health check |
| 2026-10-29 | 849d9fd | Claude Code general-purpose agent, Claude Opus 5.5 | calculator | 3/3 | 0 | Found App::request by guessing; no devtools or MCP route for a direct send. First score 2/3 was an acceptance-test JSON parser bug, fixed and rescored |
| 2026-10-29 | 849d9fd | Claude Code general-purpose agent, Claude Opus 5.5 | repair | 2/2 | 0 | The quarantine message named the fix. A misspelt subscription gets no warning. The run shared a target directory, and the first test hit a stale build of the same name; the agent caught it |
