# runuz

runuz is a headless AST-based coding and semantic prose editing agent.
It reads any project on Earth (code, configs, docs, data) and makes
surgical edits to it, guided by the real structure of the file rather
than by guessing at strings. Code is handled through its AST via
tree-sitter; everything else is handled structurally. It ships as a
single binary you can install anywhere.

## Install

```sh
cargo install runuz
```

or build it yourself:

```sh
make build                # target/release/runuz
make install              # copies it into ~/.local/bin
```

If your terminal can't find the binary, add `~/.local/bin` to your path:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Check the build with `runuz --version`.

## Usage

The command line is the documentation. `runuz --help` prints the whole
surface:

```sh
runuz --help
```

The symbol and scope vocabulary, for the parts of the CLI that resolve
structural names, lives in [`grammar.md`](grammar.md).

## Use as a hum hive

runuz started as hum's filesystem-forager surface, and this standalone
binary is the installable-anywhere version of that same surface. If you
run hum, [`hive/`](hive/) holds `runuz-hive`, a formal remote forager
hive: it advertises the CLI's tool surface on the fly (one `runuz_<sub>`
tool per subcommand, from `runuz tools --json`) and shells out to the
`runuz` binary for the actual file work. The CLI stays the single source
of truth, so the hive never needs a change when the CLI grows. See
[`hive/README.md`](hive/README.md) for how to install it.

## Languages

AST-backed today: `rs`, `py`/`pyi`, `go`, `js`/`jsx`/`mjs`/`cjs`,
`ts`, `tsx`. Everything else is handled by the structure-aware
non-code path. `bash` is intentionally not part of the standalone CLI.

## License

MIT. See `LICENSE`.
