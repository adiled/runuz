---
title: "runuz-hive"
description: "runuz's formal remote forager hive for hum — advertises the runuz filesystem tool surface (humfs_read, humfs_do_code, humfs_do_noncode) and translates chi:tool-call ↔ runuz CLI invocations"
---

# runuz-hive

> _runuz's formal remote forager hive of hum. A thrum-attached forager
> that advertises runuz's filesystem surface and shells out to the
> `runuz` CLI for the actual file ops._

`runuz-hive` is a **forager hive** for hum: a standalone process that
dials humd's thrum socket, handshakes with `bee: ["forager"]`, and
handles `chi:"tool-call"` tones humd routes here by `toolName`. It is
a *remote hive*: the actual filesystem work happens in the installed
`runuz` binary, not in-process — the hive is the thrum bridge.

## Tools it advertises

- **`humfs_read`** — filesystem analysis: discover, study, search.
  Works on any file — code returns a tree-sitter symbol outline;
  configs/docs return an anchor outline; extensionless files return
  content. Path auto-detection: file | directory | glob. Pick at most
  one modifier: `symbol` (exact, dot-nested), `query` (fuzzy on
  symbol names), `pattern` (regex over content).
- **`humfs_do_code`** — AST-grounded code authoring. Operations:
  `create` | `replace` | `insert_before` | `insert_after` | `delete`.
  The top-of-file import block is addressable as the synthetic
  `imports` symbol. Sub-symbol walks compose with dots. Every write is
  re-parsed; a syntax-error result aborts the write. Non-code files
  route to `humfs_do_noncode`.
- **`humfs_do_noncode`** — linguistic-scope edits for non-code. Four
  scopes (pass exactly one): `word` (token swap), `phrase` (structural
  name or exact text), `sentence` (whole line), `paragraph` (full
  block). Omit `replace` to delete the scope; no scope param
  creates/overwrites the whole file. Code files route to
  `humfs_do_code`.

Each tool maps 1:1 onto a `runuz` CLI subcommand (`read`, `create`,
`replace`, `insert_before`, `insert_after`, `delete`, `word`,
`phrase`, `sentence`, `paragraph`) plus `--json`, and the `--json`
result is parsed back into a `hum_mcp` `ToolResult`.

## Architecture

```
humd ─chi:tool-call(humfs_read)─► runuz-hive ─runuz read --json─► runuz CLI ─fs op─► disk
   ◄─chi:tool-result───────────────────────────────────────────────◄
```

The hive is pure transport: it translates `chi:"tool-call"` tones into
`runuz` CLI invocations (locating the binary at `~/.cargo/bin/runuz`,
override via `RUNUZ_BIN`), and ships `chi:"tool-result"` back keyed by
the same `callId`. No file ops happen in-process.

## A formal remote hive of hum

This is a *formal* hive, not a vendored stand-in: it depends on hum's
reusable hive kernel via git addressing — `hum-nest` (which owns
`serve_forager` / `ForagerAdvert` / `ToolDispatcher` / `ToolDef` /
`ToolResult`), `hum-paths`, and `thrum-core` — the same building
blocks hum's own `humfs` hive uses. No daemon-tree imports.

It advertises its own kind (`hive: "runuz"`) and a canonical persisted
`fbee_<hex>` hid derived from `$XDG_STATE_HOME/hum/bees/runuz.key`, so
humd dedupes it across reconnects instead of leaking manifests.

## Running it

Ship an `Orchfile` (SERVICE runuz, RUN `~/.local/bin/runuz-hive`,
RESTART always) and `hum hive install <target>` builds the binary and
registers the bee with orchd.
