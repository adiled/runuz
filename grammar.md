# runuz grammar

The symbol and scope vocabulary that runuz resolves on top of each file's
own structure. This is the reference; the README covers usage.

## Symbol paths

A `--symbol` names a definition in the file. Top-level definitions are
the functions, classes, structs, impls, enums, type aliases, and modules
the language's tree-sitter query captures. Symbols nest with dots:

- `Class.method` — a method inside a class/struct/impl.
- `imports` — the synthetic symbol covering the leading contiguous run of
  `import` / `use` / `include` / `require` nodes at top level.

## Sub-symbol walks

Beyond a named symbol you can walk into its sub-parts with a 7-word
vocabulary, composing with dots. Each word resolves a structural node;
`#N` disambiguates when several match.

| word | what it resolves |
|---|---|
| `body` | the inside-block of any compound (function body, then-branch of an `if`, loop body, `try` block). Resolves the `body`/`consequence` field first, else the first block-typed child. |
| `when` | an `if` (`if_statement` / `if_expression`). |
| `otherwise` | the alternate branch: `else` of an `if` (its `alternative` field), `catch` of a `try`. |
| `loop` | `for` / `while` / `loop` / `do`. |
| `try` | the `try` construct. |
| `return` | a `return` statement. |
| `call` | a function call. |

Examples:

```sh
runuz replace --file-path main.rs --symbol main.body \
  --new-source 'run();'
runuz delete  --file-path main.rs --symbol main.when
runuz replace --file-path main.rs --symbol main.call#2 \
  --new-source 'handle()'
```

The walk is document-order; `#N` counts distinct siblings, not nested
matches inside earlier ones (a call inside a call is `call#1.call`, not
`call#2`).

## Multiple symbols

`--symbols A,B` (replace / delete) resolves each name against the same
file and splices each symbol's own range in one atomic write — contiguous
or not. Any missing name aborts with no partial edit.

## Non-code scopes

For non-code files the `--scope-text` is the *text* to find, and the scope
kind is the subcommand you chose:

| subcommand | resolves |
|---|---|
| `word` | a single token, format-agnostic (word-boundary regex). |
| `phrase` | a structural name OR exact text. Format-aware: JSON keys + values, env vars, markdown headings, TOML sections. Falls back to first exact-substring match. |
| `sentence` | the single line containing the scope text. |
| `paragraph` | the blank-line paragraph (or YAML indentation block) containing the scope. |

Omit `--replace` to delete the resolved scope; no scope parameter
creates / overwrites the whole file.