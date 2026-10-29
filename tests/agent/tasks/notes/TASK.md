# Task: notes

Build a notes tool as a bones product in this folder: type a note, add it to a list, and find the list again after a restart.

A bones release is unpacked at `{{engine}}`: the engine, `bones-cli`, the `bones-user` agent plugin in `agent-plugins/`, and the extension ABI in `wit/`. Create the project with:

```sh
{{engine}}/bones-cli new notes --kind egui{{bones_path}}
```

You are done when `cargo test` passes in the project and the tool below works. Do not ask a person for help; the release's documentation and tools are all you have.

## The contract

Every frame publishes a `ui/spec` titled `notes` whose widgets are, in order:

1. a text edit with id 1, holding the text being typed;
2. a button with id 2 labelled `Add`;
3. one label per note, oldest first.

When `ui/changed` arrives for id 1, its text becomes the text being typed. When `ui/clicked` arrives for id 2 and the text being typed is not empty, the text is appended to the list and the text edit becomes empty. Clicking `Add` with nothing typed changes nothing.

The list is saved with the engine's persistence whenever it changes, and loaded in `init`, so a fresh start shows the notes added before it. The storage format is yours to choose.
