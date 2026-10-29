# Task: calculator

Build a calculator service as a bones product in this folder: an extension other extensions call with a direct `send` and get an answer from.

A bones release is unpacked at `{{engine}}`: the engine, `bones-cli`, the `bones-user` agent plugin in `agent-plugins/`, and the extension ABI in `wit/`. Create the project with:

```sh
{{engine}}/bones-cli new calculator --kind game{{bones_path}}
```

It needs no drawing; remove what the template draws. You are done when `cargo test` passes in the project and the service below works. Do not ask a person for help; the release's documentation and tools are all you have.

## The contract

A direct `send` to the extension carries a JSON object as UTF-8:

```json
{ "op": "add", "a": 2, "b": 3 }
```

`op` is `add`, `sub`, `mul`, or `div`, and `a` and `b` are numbers. The reply is a JSON object:

- `{ "result": <number> }` when the operation succeeds;
- `{ "error": "division by zero" }` for `div` with `b` equal to 0;
- `{ "error": "<any message>" }` for anything else that is not a valid request.

Every send gets a reply.
