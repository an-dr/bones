# bones {version}

The bones engine, for {os}-{arch}.

Kernel version {version}; extension ABI bones:extension@{abi}. The two move
independently -- see docs/adr/ADR-029 in the repository.

Web panels (wry): {web}. A build without them refuses to start with
`web = true` in bones.toml; on Linux, a build with them needs webkit2gtk 4.1
and GTK 3 installed.

## Run it

    ./{exe}

It reads bones.toml from this directory and loads every .wasm file in
extensions/. Both paths resolve against the executable's own location, not the
shell's working directory, so a shortcut or a double-click behaves the same as
a terminal.

## Start a product

    ./bones-cli new my-game --kind game
    cd my-game
    ../bones-cli dev

bones-cli finds this engine as its sibling. See its --help.

## Contents

    {exe}                  the engine
    bones-cli                 developer tools: new, doctor, build, dev, screenshot, mcp
    agent-plugins/bones-user/ the Agent Plugins package for an AI agent building a product:
                              rules, the bones skill with the documentation, and the
                              bones-cli MCP server
    bones.toml                configuration; every value is the default
    extensions/               extensions loaded at startup
    wit/extension.wit         the extension ABI: the calls
    wit/wire-format.md        the extension ABI: the bytes those calls carry
    wit/wire-format.json      the same, as data to generate a codec from
    wit/vectors/              conformance vectors for the wire format
    LICENSE                   MIT, covering bones itself
    THIRD-PARTY-NOTICES.txt   the statically linked dependencies

## Write an extension

Drop any component built for wasm32-wasip2 into extensions/. In Rust, depend on
bones-wasm-sdk; in any other language, generate bindings from wit/extension.wit and
build payloads per wit/wire-format.md.

An extension built against a different bones:extension version will not load. That is
deliberate and is checked at instantiation, not at runtime.

## Where this came from

https://github.com/an-dr/bones
