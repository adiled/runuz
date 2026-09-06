# runuz

A small command-line filesystem tool that **reads and edits any project on
Earth** — code, configs, docs, data — using the real structure of the file
rather than guessing at strings. It's AST-grounded for code (tree-sitter),
and structure-aware for everything else. Zero dependencies on any hum
infrastructure; it's a single binary you can install anywhere.

You use it to answer "what's in this file?" and to make surgical edits
without touching anything you didn't mean to touch.

## Install it

```sh
cargo install --path .    # puts `runuz` on your PATH
```

or build it yourself:

```sh
make build                # target/release/runuz
make install              # copies it into ~/.local/bin
```

That gives you a program called `runuz`. If your terminal can't find it,
add this to your `~/.zshrc` or `~/.bashrc`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Check which build you're on with `runuz --version`.

## The two ideas

runuz has two halves, and each one is dead simple:

- **Reading.** `runuz read` opens a file and gives you a *symbol outline*:
  the functions, classes, structs, and fields inside it, one line each.
  Point it at a directory or a glob and it inventories what's there. Ask
  for a specific symbol, a fuzzy name, or a regex over the content.
- **Editing.** `runuz create / replace / insert_before / insert_after /
  delete` make AST-grounded, symbol-scoped edits. You say *which symbol*
  and *what new source*, and runuz splices exactly that byte range — not a
  text search-and-replace that can hit the wrong spot. Non-code files
  (`.env`, TOML, JSON, markdown, READMEs) get linguistic-scope edits:
  `word`, `phrase`, `sentence`, `paragraph`.

Every write is re-parsed before it lands. A broken edit is rejected and
the original file is left untouched.

## Reading

```sh
# A code file: get its symbol outline
runuz read --file-path src/main.rs

# Just one symbol
runuz read --file-path src/main.rs --symbol main

# A fuzzy name match
runuz read --file-path src/main.rs --query auth

# A regex over the content
runuz read --file-path src/ --pattern 'TODO'
```

`read` decides the framing itself: a file gets an outline, a directory or
glob gets an inventory, configs and docs get an anchor outline. It skips
junk directories (`node_modules`, `.git`, `target`, ...) so a `read('/')`
doesn't explode.

## Editing code

All edits are symbol-scoped — you name the symbol, runuz swaps its exact
byte range:

```sh
# Create a new file (fails if it already exists)
runuz create --file-path src/main.rs --new-source 'fn main() {}'

# Replace a symbol's whole body
runuz replace --file-path src/main.rs --symbol main \
  --new-source 'fn main() { run(); }'

# Splice a helper right before / after a symbol
runuz insert_before --file-path src/main.rs --symbol main \
  --new-source 'fn helper() {}'
runuz insert_after --file-path src/main.rs --symbol main \
  --new-source 'fn helper() {}'

# Drop a symbol entirely
runuz delete --file-path src/main.rs --symbol helper

# Or replace / delete several symbols in one atomic write
runuz replace --file-path src/main.rs --symbols main,helper \
  --new-source 'fn main() { run(); }'
```

The top-of-file import block is addressable as the synthetic symbol
`imports`. Symbols nest with dots (`Class.method`), and you can walk into
sub-parts of a symbol (`main.body`, `main.loop`) — the full vocabulary is
in [`grammar.md`](grammar.md).

Omit `--symbol` on `replace` to rewrite the whole file. `write` does a
whole-file create or overwrite, auto-routing by extension.

## Editing everything else

For non-code files the scopes are linguistic — you name the smallest
unit and replace it:

```sh
# Swap one word
runuz word --file-path .env DATABASE_URL --replace 'postgres://new/db'

# Swap a phrase — structure-aware: JSON keys/values, env vars,
# markdown headings, TOML sections
runuz phrase --file-path README.md runuz --replace runuz2

# Sentence = the single line holding the text
runuz sentence --file-path notes.txt 'fix the bug' --replace 'done'

# Paragraph = the whole blank-line block
runuz paragraph --file-path notes.txt 'old paragraph' --replace 'new'
```

Omit `--replace` to delete the resolved scope. JSON files are re-validated
as JSON after the edit; a broken result is rejected.

## Machine-readable output

Add `--json` anywhere and runuz prints `{is_error, output, title,
metadata}` instead of text.

## Languages

AST-backed today: `rs`, `py`/`pyi`, `go`, `js`/`jsx`/`mjs`/`cjs`,
`ts`, `tsx`. Everything else is handled by the structure-aware
non-code path. `bash` is intentionally not part of the standalone CLI.

## Where did this come from?

runuz was extracted from hum's `humfs` forager hive — the `humfs/`
directory in this repo is the follow-up remote hive that shells out to
this CLI. The standalone form here is the installable-anywhere version
of the same tool surface.

## License

MIT. See `LICENSE`.
