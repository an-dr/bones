# Use cases

What people and agents use bones for, one goal at a time. A use case names who acts, what they want, the steps they take, and what they observe when it worked. It describes behaviour visible from outside — commands, files a user sees, results — never how a component implements it, so an internal refactoring never changes one.

Use cases sit above [architecture](../architecture/index.md) and [design](../index.md#detailed-design): they say what must keep working, and the layers below say how. A change that breaks a use case is a breaking change, whatever the code looks like; a new capability arrives with the use case it serves.

## Format

Each use case has a stable ID, `UC-<AREA>-<NN>`, kept when the use case is reworded and never reused after it is removed.

- **Actor** — who acts: a product developer (writes extensions), an application developer (links the engine into their own program; an *embedder* in the [glossary](../glossary.md)), an AI agent, CI, or a bones maintainer.
- **Goal** — what they want, in their words.
- **Flow** — the steps, as the commands or actions they take.
- **Success** — what they observe when it worked.

## Areas

- [bones-develop.md](bones-develop.md) — `UC-DEV`: installing bones, creating products, embedding the engine in an existing program, the development loop, and keeping a project current.
- [ui.md](ui.md) — `UC-UI`: building interfaces from bones widgets, icons, custom widgets, the native look, and the window's own title bar.
