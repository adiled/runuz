---
title: "runuz-hive"
description: "runuz's formal remote forager hive for hum. Mirrors the runuz CLI tool surface (runuz_<subcommand>) on the fly and translates chi:tool-call tones into runuz CLI invocations"
---

# runuz-hive

`runuz-hive` is a forager hive for hum. It dials humd's thrum socket,
handshakes with `bee: ["forager"]`, and handles the `chi:"tool-call"`
tones humd routes here by `toolName`. It is a remote hive: the actual
filesystem work happens in the installed `runuz` binary. The hive is the
thrum bridge, and it does no file work in-process.

## The CLI is the source of truth

The hive hardcodes no tool list. On startup it runs `runuz tools --json`,
which emits one `ToolDef` per CLI subcommand, named `runuz_<subcommand>`,
and it advertises those defs verbatim. Dispatch is generic: `toolName`
`runuz_<sub>` maps to `runuz <sub> --<kebab-key> <val>` for each schema
key present, with the scope tools (`runuz_word`, `runuz_phrase`,
`runuz_sentence`, `runuz_paragraph`) passing their scope value as the
positional.

So when the `runuz` CLI grows, no hive change is ever needed: add a
subcommand and the hive picks it up automatically. The CLI is the single
source of truth; the hive is a pure mirror.

## Advertised tools

The CLI reports one `runuz_<sub>` tool per subcommand:

- `runuz_read` for filesystem analysis: discover, study, search.
- `runuz_create`, `runuz_replace`, `runuz_insert_before`,
  `runuz_insert_after`, `runuz_delete` for AST-grounded code authoring
  (top-of-file `imports` symbol; sub-symbol walks compose with dots;
  every write is re-parsed).
- `runuz_write` for whole-file writes, auto-routed by extension.
- `runuz_word`, `runuz_phrase`, `runuz_sentence`, `runuz_paragraph` for
  linguistic-scope edits on non-code files.

Each maps 1:1 onto a `runuz` CLI subcommand plus `--json`, and the
`--json` result is parsed back into a `hum_mcp` `ToolResult`.

## Architecture

```
humd --chi:tool-call(runuz_read)--> runuz-hive --runuz read --json--> runuz CLI --fs op--> disk
   <--chi:tool-result---------------------------------------------------------------
```

The hive is pure transport: it translates `chi:"tool-call"` tones into
`runuz` CLI invocations (locating the binary at `~/.cargo/bin/runuz`,
overridable via `RUNUZ_BIN`), and ships `chi:"tool-result"` back keyed
by the same `callId`. No file operations happen in-process.

## A formal remote hive

This is a formal hive. It depends on hum's reusable hive kernel via git
addressing: `hum-nest` (which owns `serve_forager`, `ForagerAdvert`,
`ToolDispatcher`, `ToolDef`, `ToolResult`), `hum-paths`, and
`thrum-core`. Those are the same building blocks hum's own fs hive uses,
imported directly rather than from a daemon tree.

It advertises its own kind (`hive: "runuz"`) and a canonical persisted
`fbee_<hex>` hid derived from `$XDG_STATE_HOME/hum/bees/runuz.key`, so
humd dedupes it across reconnects instead of leaking manifests.

## Running it

Ship an `Orchfile` (SERVICE runuz, RUN `~/.local/bin/runuz-hive`,
RESTART always) and run `hum hive install <target>` to build the binary
and register the bee with orchd.
