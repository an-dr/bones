
## Get this release

Install the tools of this release, then follow the [bones-develop manual]({manual}):

```sh
cargo install --git https://github.com/an-dr/bones --tag {tag} bones-develop
bones-develop new my-game   # a project pinned to {tag}
```

An existing project moves here with `bones-develop upgrade {tag}`. An embedder pins `bones-kernel = { git = "https://github.com/an-dr/bones", tag = "{tag}" }`.
