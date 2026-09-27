---
title: "runuz-hive"
description: "runuz's formal remote forager hive for hum — mirrors the runuz CLI tool surface (runuz_<subcommand>) on-the-fly and translates chi:tool-call ↔ runuz CLI invocations"
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

## Zero-maintenance: the CLI is the source of truth

The hive hardcodes **no tool list**. On startup it spawns
`runuz tools --json` — which the CLI emits as one `ToolDef` per CLI
subcommand, named `runuz_<subcommand>` — and advertises those defs
verbatim. Dispatch is fully generic: `toolName` `runuz_<sub>` maps to
`runuz <sub> --<kebab-key> <val>` for each schema key present, with the
scope tools (`runuz_word` / `runuz_phrase` / `runuz_sentence` /
`runuz_paragraph`) passing their scope value as the positional.

So as the `runuz` CLI surface grows, **no hive change is ever needed**:
add a subcommand, and the hive picks it up automatically. The CLI is the
single source of truth; the hive is a pure mirror.

## Advertised tools

Today the CLI reports eleven `runuz_<sub>` tools — one per subcommand:

- **`runuz_read`** — filesystem analysis: discover, study, search.
- **`runuz_create`** / **`runuz_replace`** / **`runuz_insert_before`** /
  **`runuz_insert_after`** / **`runuz_delete`** — AST-grounded code
  authoring (top-of-file `imports` symbol; sub-symbol walks compose with
  dots; every write re-parsed).
- **`runuz_write`** — whole-file write, auto-routed by extension.
- **`runuz_word`** / **`runuz_phrase`** / **`runuz_sentence`** /
  **`runuz_paragraph`** — linguistic-scope edits for non-code.

Each maps 1:1 onto a `runuz` CLI subcommand plus `--json`, and the
`--json` result is parsed back into a `hum_mcp` `ToolResult`.

## Architecture

```
humd ─chi:tool-call(runuz_read)─► runuz-hive ─runuz read --json─► runuz CLI ─fs op─► disk
   ◄─chi:tool-result──────────────────────────────────────────────────◄
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
blocks hum's own fs hive uses. No daemon-tree imports.

It advertises its own kind (`hive: "runuz"`) and a canonical persisted
`fbee_<hex>` hid derived from `$XDG_STATE_HOME/hum/bees/runuz.key`, so
humd dedupes it across reconnects instead of leaking manifests.

## Running it

Ship an `Orchfile` (SERVICE runuz, RUN `~/.local/bin/runuz-hive`,
RESTART always) and `hum hive install <target>` builds the binary and
registers the bee with orchd.
