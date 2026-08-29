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

```sh
# Read a file: code files get a symbol outline
runuz read --file-path src/main.rs
runuz read --file-path src/main.rs --symbol main
runuz read --file-path src/ --pattern 'TODO'

# Author code: AST-grounded, symbol-scoped
runuz do_code --file-path src/main.rs --operation create --new-source 'fn main() {}'
runuz do_code --file-path src/main.rs --operation replace --symbol main --new-source 'fn main() { run(); }'
runuz do_code --file-path src/main.rs --operation insert_after --symbol main --new-source 'fn helper() {}'
runuz do_code --file-path src/main.rs --operation delete --symbol helper

# Author non-code: linguistic scope (word/phrase/sentence/paragraph)
runuz do_nocode --file-path .env --phrase DATABASE_URL --replace 'postgres://new/db'
runuz do_nocode --file-path README.md --word runuz --replace runuz2

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
| `do_code` | AST-grounded code authoring: `create` / `replace` / `insert_before` / `insert_after` / `delete`, symbol-scoped with the synthetic `imports` symbol |
| `do_nocode` | linguistic-scope edits: `word` / `phrase` / `sentence` / `paragraph`; JSON results re-validated as JSON |

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
- `humfs/`: the literal copy of hum's fs forager hive (follow-up:
  remote hive over the thrum protocol + humd, shelling out to this CLI).
