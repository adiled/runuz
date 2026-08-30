https://github.com/user-attachments/assets/625883a7-7ddf-4ce6-961f-740dac5e4c1d

# runuz

A standalone filesystem tool: **`do_code`**, **`do_nocode`**, **`do_read`**
for any project on Earth. AST-grounded via tree-sitter; zero hum
dependencies.

`runuz` was extracted from hum's `humfs` forager hive (copied literally
into [`humfs/`](humfs/) as the follow-up remote hive). The CLI here is the
standalone, installable-anywhere form of that same tool surface.

## Build / install

```sh
cargo build --release        # binary: target/release/runuz
cargo install --path .       # install `runuz` on PATH
```

## Usage

All tool operations are TOP-LEVEL subcommands, so they're discoverable
and hard to forget:

```sh
# Read a file: code files get a symbol outline
runuz read --file-path src/main.rs
runuz read --file-path src/main.rs --symbol main
runuz read --file-path src/ --pattern 'TODO'

# Author code: AST-grounded, symbol-scoped (top-level ops)
runuz create  --file-path src/main.rs --new-source 'fn main() {}'
runuz replace --file-path src/main.rs --symbol main --new-source 'fn main() { run(); }'
runuz insert_before --file-path src/main.rs --symbol main --new-source 'fn helper() {}'
runuz insert_after  --file-path src/main.rs --symbol main --new-source 'fn helper() {}'
runuz delete --file-path src/main.rs --symbol helper

# Author non-code: linguistic scopes are top-level subcommands
runuz phrase --file-path .env DATABASE_URL --replace 'postgres://new/db'
runuz word  --file-path README.md runuz --replace runuz2

# Machine-readable output
runuz read --file-path src/main.rs --json
```

Every write is re-parsed for syntax errors (code) or re-validated as
JSON (non-code) before landing; a broken edit is rejected and the
original is left untouched.

## Tool surface

| subcommand | what it does |
|---|---|
| `read` | filesystem analysis: file / directory / glob; symbol outline for code, anchor outline for configs/docs; `--symbol` (exact), `--query` (fuzzy name), `--pattern` (regex content) |
| `create` / `replace` / `insert_before` / `insert_after` / `delete` | AST-grounded code authoring, symbol-scoped with the synthetic `imports` symbol |
| `word` / `phrase` / `sentence` / `paragraph` | linguistic-scope non-code edits; JSON results re-validated as JSON |

(bash intentionally dropped; not part of the standalone CLI.)

## Languages

tree-sitter-backed AST for `rs`, `py`/`pyi`, `go`, `js`/`jsx`/`mjs`/`cjs`,
`ts`, `tsx`. Sub-symbol walks (`body`/`when`/`otherwise`/`loop`/`try`/
`return`/`call`) compose with dots and disambiguate with `#N`.

## Layout

- `src/lib.rs`: the reusable core (AST + tools) plus a local
  `ToolDef`/`ToolResult` contract replicating `nest_common`, so the
  follow-up hive re-integration is drop-in.
- `src/main.rs` / `src/cli.rs`: the `runuz` binary.
- `humfs/`: the **runuz remote hive** — a standalone forager over the
  thrum protocol + humd. Advertises `humfs_read` / `humfs_do_code` /
  `humfs_do_noncode`, routes `chi:"tool-call"` tones from humd, and
  shells out to this `runuz` CLI for file ops (no in-process fs work).
  Vendored wire machinery under `humfs/src/wire/` (Hid, bee identity,
  serve_forager, XDG paths) — zero hum internals.
