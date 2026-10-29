# Agent benchmark

Measures how well an AI agent builds a product on bones, given only a release archive and an empty folder. Each release runs it, so a change to the engine, its tools, or its documentation can be judged by whether agents do better.

## The tasks

Each task directory holds a `TASK.md`, the only prompt the agent sees, and `acceptance/`, tests the agent never sees. Some tasks also hold `starter/`, a project the agent starts from.

| Task | Asks for | Exercises |
| --- | --- | --- |
| [pong](tasks/pong/TASK.md) | Two paddles, a ball, and a score, drawn with `gfx/*` | input, drawing, game state, text |
| [notes](tasks/notes/TASK.md) | A notes list in an egui panel that survives a reload | `ui/*`, persistence |
| [calculator](tasks/calculator/TASK.md) | An extension that answers direct sends with JSON | direct `send`, replies |
| [repair](tasks/repair/TASK.md) | Find why a given extension stops drawing, and fix it | diagnostics, budgets |

`TASK.md` states the contract the acceptance tests check: key names, sizes, ids, and payload shapes. The agent is free in everything else, so the benchmark measures whether it can build to a stated contract, not whether it guesses one.

## Running one

```sh
cargo xtask dist --no-archive                 # the engine and tools under test
cargo xtask bench prepare pong <workspace>    # a clean folder: engine/ and TASK.md
# give an agent <workspace>/TASK.md and nothing else; let it work until it says it is done
cargo xtask bench score pong <workspace>      # copies acceptance/ in and runs it
```

`prepare` copies `dist/` into `<workspace>/engine/` and writes `TASK.md` with that path filled in. Until this release is tagged, generated projects need `--bones-path`, so `prepare --local` names this checkout in the prompt.

Give each workspace its own cargo target directory, which is the default. A target directory shared between workspaces can hold a build of a same-named extension from another run, and an agent's first `cargo test` may then test that build instead of its own code. `score` refreshes the extension's source timestamps before it builds, so scoring always rebuilds.

`score` finds the project under the workspace (the folder holding `bones.toml`), copies the acceptance tests into its `checks/tests/`, runs `cargo test`, and prints one JSON line with the counts.

## Recording a run

Add a row to [results.md](results.md) for every run: the date, the engine commit, the agent and model, the task, the acceptance count, how many times a person had to step in, and anything the run showed about bones. A run in which a person changed code or told the agent what to write does not count as a pass, whatever the tests say.
