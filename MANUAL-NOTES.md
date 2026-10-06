# runuz manual notes

Philosophy and meta-knowledge that used to live as comments inside code lands here.
Feeds the man page (MANUAL.md).

## Symbol extraction (ast.rs)

- One parser registry keyed by file extension. Each language ships a tree-sitter
  Language, a Query over named-symbol patterns (functions, classes/structs,
  methods, top-level bindings, types, enums), and a capture-name to SymbolKind
  mapping.
- Extraction parses once, runs the query, and walks captures into a flat
  Vec<Symbol> sorted by byte range. The read outline is fed by this; symbol-scoped
  writes consume the same list, so read and edit always agree.
- Capture convention: `@def` captures the enclosing definition node (its byte
  range is the unit we edit), `@name` captures the identifier. Other capture
  suffixes (.fn / .class / ...) act as the kind tag.
- Anonymous or unnamed captures never surface. Only named structural definitions
  are kept; an `Other` tag is dropped rather than leaked into outlines, ambiguity
  lists, or pattern annotations.
- Queries capture symbols at any nesting depth (top-level plus nested level:
  methods inside classes and impls). Control-flow internals are NOT symbols:
  closures, if/match/loop bodies, arrow functions, decorators, and function
  literals never appear because their structure is reachable through
  sub-symbol walks (body/when/loop/...) and the outline must stay quiet.
- TSX reuses the TypeScript symbol surface (same query, same capture tags).
- Symbol data model: the byte range is half-open [start_byte, end_byte); the row
  range is 1-based inclusive on both ends. Imports is a synthetic block spanning
  the file's import region; Field is a struct field (Rust only).

## Address resolution (ast.rs resolve_path + subwalk.rs)

- A symbol path is segments joined by dots: "foo", "Class.method",
  "alpha.when.otherwise", "alpha.loop#2.body".
- Named segments resolve by containment of the previously matched symbol's byte
  range. A bare name that matches several symbols is ambiguous and fails loudly,
  so a delete/replace never silently hits the wrong one. Nested paths
  disambiguate by containment (a struct and its impl may both be named `Type`),
  and #N picks an occurrence.
- Alias segments (body/when/otherwise/loop/try/return/call) walk the resulting
  AST node via subwalk (subwalk.rs): 7-word vocabulary composed with dots,
  disambiguated with #N. `body` is the inside-block of any compound (function
  body, then-branch, loop body, try block) via the `body`/`consequence` named
  field, falling back to the first block-typed child for the language. `when`
  is an if (if_statement / if_expression). `otherwise` is the alternate
  branch: `else` via the `alternative` field, or a catch-clause typed
  descendant. `loop` is for / while / loop / do. `try` matches try constructs
  (unsupported on Rust: tree-sitter-rust has no top-level try_expression; the
  `?`-postfix is a unary op without a recognizable scope). `return` and `call`
  match their statements/expressions.
- The walk is document-order. #N counts distinct siblings, not nested matches
  inside earlier matches: a call inside a call is `call#1.call`, not `call#2`
  of the enclosing scope.

## Syntax gate (ast.rs)

- validate_syntax returns a one-line error with a position for the first ERROR /
  missing node.
- validate_edited rejects an edit only if it introduced or overlapped a syntax
  error. Errors are matched by byte range and by text, so a pre-existing error
  elsewhere (a Rust 2024 `safe fn` a parser does not know yet) does not block an
  edit that touches clean regions; deleting/inserting bytes above an error would
  otherwise move its offset.
- read annotates regex hits with the smallest symbol containing the byte, so a
  pattern hit names the function/class it sits inside.

## Multi-symbol ops (tools/code.rs)

- The `--symbols A,B` list is resolved against the SAME source, so resolved
  ranges cannot drift relative to each other. Any missing name aborts the whole
  edit: no partial application.
- Replace and delete splice every range in one pass and one write.
  Overlapping or adjacent ranges coalesce into one combined span, so a
  contiguous run still yields a single replacement.
- Delete extends each range to its full line (line start, field terminator) and
  only eats a trailing newline when the line it leaves behind is surely blank.
- The synthetic `imports` symbol groups the leading contiguous run of top-level
  import nodes into one byte range. Write ops resolve it from
  `synthesize_imports`; the read tool resolves it through a crate-visible alias
  that calls the same function, so read and write always agree on its range.
  Node kinds by language: use_declaration and extern_crate_declaration (rust),
  import_statement (py/js/ts), import_from_statement (py),
  import_declaration (go/js/ts), import_spec, require_statement,
  preproc_include (c/cpp).

## Tool surface (tools/surface.rs)

- SURFACE is the single source of truth the runuz-hive mirrors on the fly.
  `runuz tools --json` emits one ToolDef per CLI subcommand, named
  runuz_<subcommand>. The hive advertises these verbatim and dispatches
  generically as `runuz <sub> --<key> <val>`, so growing the CLI never requires
  a hive change: add a subcommand plus a line in SURFACE and the hive picks it
  up automatically.
- Schema property keys are snake_case in the surface; they become kebab-case
  flags (--file_path becomes --file-path) at the CLI.

## Integration tests (tests/cli.rs)

- Black-box: each test invokes the compiled runuz binary as a subprocess (via
  the CARGO_BIN_EXE_runuz env var), passing CLI arguments and asserting on
  stdout/stderr/exit code.