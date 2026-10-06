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

## Usage

The command line is the documentation. `runuz --help` prints the whole
surface:

```sh
runuz --help
```

To learn more, see `man runuz`.

## Use as a hum hive (experimental)

If you run hum, ship [`hive/`](hive/)'s `Orchfile` and run
`hum hive install <target>` to build `runuz-hive` and register the bee
with orchd. See [`hive/README.md`](hive/README.md).

## Supported files

- [x] Rust (`rs`)
- [x] Python (`py`, `pyi`)
- [x] Go (`go`)
- [x] JavaScript (`js`, `jsx`, `mjs`, `cjs`)
- [x] TypeScript (`ts`)
- [x] TSX (`tsx`)
- [x] Structured files (`json`, `yaml`, `toml`, `env`)
- [x] All text files

## License

MIT. See `LICENSE`.
