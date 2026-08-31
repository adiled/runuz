//! Command-line parsing for the runuz binary — a small hand-rolled
//! parser (no clap dependency; keeps the standalone crate lean).
//!
//! The tool *operations* are TOP-LEVEL subcommands so they're
//! discoverable and impossible to forget:
//!
//!   code ops:  runuz create | replace | insert_before | insert_after | delete
//!   text scopes: runuz word | phrase | sentence | paragraph
//!   reading:  runuz read
//!
//! Each op takes its target as the FIRST positional arg after the
//! subcommand (where applicable), then `--file-path` (required) and
//! op-specific flags.

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Args {
    pub cmd: Command,
    pub json: bool,
}

#[derive(Debug)]
pub enum Command {
    Read(ReadArgs),
    Create(CreateArgs),
    Replace(ReplaceArgs),
    Insert(InsertArgs),
    Delete(DeleteArgs),
    Write(WriteArgs),
    DoNonCode(DoNonCodeArgs),
}

#[derive(Debug)]
pub struct ReadArgs {
    pub file_path: String,
    pub symbol: Option<String>,
    pub query: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug)]
pub struct CreateArgs {
    pub file_path: String,
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct ReplaceArgs {
    pub file_path: String,
    pub symbol: Option<String>,     // None => whole-file rewrite
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct InsertArgs {
    pub file_path: String,
    pub anchor: String,             // before | after
    pub symbol: String,             // the anchor symbol
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct DeleteArgs {
    pub file_path: String,
    pub symbol: String,
}

#[derive(Debug)]
pub struct WriteArgs {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug)]
pub struct DoNonCodeArgs {
    pub file_path: String,
    pub scope: String,          // word | phrase | sentence | paragraph
    pub scope_text: String,     // positional scope parameter value
    pub replace: Option<String>,
}

impl Args {
    pub fn parse() -> Result<Args> {
        let mut it = std::env::args().skip(1).peekable();
        let mut json = false;
        let mut sub: Option<String> = None;

        while let Some(a) = it.next() {
            match a.as_str() {
                "--json" => json = true,
                "-h" | "--help" => { help(); std::process::exit(0); }
                s if s.starts_with('-') => bail!("unknown flag {s:?}"),
                s => {
                    sub = Some(s.to_string());
                    break;
                }
            }
        }
        let sub = sub.ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;

        let mut pairs: Vec<(String, String)> = Vec::new();
        let mut rest: Vec<String> = Vec::new();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--json" => json = true,
                "-h" | "--help" => { help(); std::process::exit(0); }
                _ => {
                    if let Some(key) = a.strip_prefix("--") {
                        let val = it.next()
                            .ok_or_else(|| anyhow::anyhow!("flag {a} needs a value"))?;
                        pairs.push((key.to_string(), val));
                    } else {
                        rest.push(a);
                    }
                }
            }
        }

        let cmd = match sub.as_str() {
            "read" => Command::Read(read_args(&pairs)?),
            "create" => Command::Create(create_args(&pairs)?),
            "replace" => Command::Replace(replace_args(&pairs)?),
            "insert_before" => Command::Insert(insert_args(&pairs, "before")?),
            "insert_after"  => Command::Insert(insert_args(&pairs, "after")?),
            "delete" => Command::Delete(delete_args(&pairs)?),
            "write" => Command::Write(write_args(&pairs)?),
            "word" | "phrase" | "sentence" | "paragraph" =>
                Command::DoNonCode(do_nocode_args(&pairs, &rest, &sub)?),
            other => bail!("unknown subcommand {other:?} — use read, create, replace, insert_before, insert_after, delete, write, word, phrase, sentence, or paragraph"),
        };
        Ok(Args { cmd, json })
    }
}

fn req<'a>(pairs: &'a [(String, String)], key: &str) -> Result<&'a str> {
    pairs.iter().find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required --{key}"))
}

fn opt<'a>(pairs: &'a [(String, String)], key: &str) -> Option<String> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn read_args(pairs: &[(String, String)]) -> Result<ReadArgs> {
    Ok(ReadArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol: opt(pairs, "symbol"),
        query: opt(pairs, "query"),
        pattern: opt(pairs, "pattern"),
    })
}

fn create_args(pairs: &[(String, String)]) -> Result<CreateArgs> {
    Ok(CreateArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        new_source: opt(pairs, "new-source"),
    })
}

fn replace_args(pairs: &[(String, String)]) -> Result<ReplaceArgs> {
    Ok(ReplaceArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol: opt(pairs, "symbol"),
        new_source: opt(pairs, "new-source"),
    })
}

fn insert_args(pairs: &[(String, String)], anchor: &str) -> Result<InsertArgs> {
    Ok(InsertArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        anchor: anchor.to_string(),
        symbol: req(pairs, "symbol")?.to_string(),
        new_source: opt(pairs, "new-source"),
    })
}

fn delete_args(pairs: &[(String, String)]) -> Result<DeleteArgs> {
    Ok(DeleteArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol: req(pairs, "symbol")?.to_string(),
    })
}

fn write_args(pairs: &[(String, String)]) -> Result<WriteArgs> {
    Ok(WriteArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        content: req(pairs, "content")?.to_string(),
    })
}

fn do_nocode_args(pairs: &[(String, String)], rest: &[String], scope: &str) -> Result<DoNonCodeArgs> {
    Ok(DoNonCodeArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        scope: scope.to_string(),
        scope_text: rest.first().cloned().unwrap_or_default(),
        replace: opt(pairs, "replace"),
    })
}

fn help() {
    println!(r#"runuz — the standalone filesystem CLI

USAGE:
  runuz read --file-path <path> [--symbol S] [--query Q] [--pattern RE]
  runuz create  --file-path <path> [--new-source T]
  runuz replace --file-path <path> [--symbol S] [--new-source T]
  runuz insert_before --file-path <path> --symbol S [--new-source T]
  runuz insert_after  --file-path <path> --symbol S [--new-source T]
  runuz delete --file-path <path> --symbol S
  runuz write --file-path <path> --content T      (whole-file write;
                routes code files to do_code, non-code to do_nocode)
  runuz word <scope-text> --file-path <path> [--replace T]
  runuz phrase <scope-text> --file-path <path> [--replace T]
  runuz sentence <scope-text> --file-path <path> [--replace T]
  runuz paragraph <scope-text> --file-path <path> [--replace T]

All tool OPERATIONS are TOP-LEVEL subcommands:
  create        new file (fails if it exists)
  replace       symbol-scoped, or whole-file when --symbol omitted
  insert_before / insert_after   splice new_source at the anchor symbol
  delete        drop the anchor symbol's byte range
  write         whole-file create/overwrite, auto-routed by extension
  word / phrase / sentence / paragraph   linguistic scopes (omit
                --replace to delete the resolved scope)

--symbol accepts dot-nested (e.g. 'Class.method'); 'imports' addresses
the synthetic top-of-file import block. --json anywhere for
machine-readable output.
"#);
}
