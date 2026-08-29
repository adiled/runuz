//! Command-line parsing for the runuz binary — a small hand-rolled
//! parser (no clap dependency; keeps the standalone crate lean).
//!
//! The four linguistic scopes are TOP-LEVEL subcommands:
//!   runuz word <scope-text> --file-path <path> [--replace <text>]
//!   runuz phrase <scope-text> ...   sentence ...   paragraph ...
//! plus `read` for reading/outlining. Scope-as-top-level makes the
//! four scopes discoverable and impossible to forget.
//!
//! The scope text is the FIRST positional argument after the scope
//! subcommand; `--file-path` is required; `--replace` is optional
//! (omit to delete the scope).

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Args {
    pub cmd: Command,
    pub json: bool,
}

#[derive(Debug)]
pub enum Command {
    Read(ReadArgs),
    DoCode(DoCodeArgs),
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
pub struct DoCodeArgs {
    pub file_path: String,
    pub operation: String,
    pub symbol: Option<String>,
    pub new_source: Option<String>,
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

        // Remaining tokens: positional args come first (`rest`), then
        // `--key value` pairs (with `--json` allowed anywhere).
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
            "do_code" | "do-code" => Command::DoCode(do_code_args(&pairs)?),
            "word" | "phrase" | "sentence" | "paragraph" =>
                Command::DoNonCode(do_nocode_args(&pairs, &rest, &sub)?),
            other => bail!("unknown subcommand {other:?} — use read, do_code, word, phrase, sentence, or paragraph"),
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

fn do_code_args(pairs: &[(String, String)]) -> Result<DoCodeArgs> {
    Ok(DoCodeArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        operation: opt(pairs, "operation").unwrap_or_else(|| "replace".into()),
        symbol: opt(pairs, "symbol"),
        new_source: opt(pairs, "new-source"),
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
  runuz do_code --file-path <path> [--operation op] [--symbol S] [--new-source T]
  runuz word <scope-text> --file-path <path> [--replace T]
  runuz phrase <scope-text> --file-path <path> [--replace T]
  runuz sentence <scope-text> --file-path <path> [--replace T]
  runuz paragraph <scope-text> --file-path <path> [--replace T]

The four linguistic scopes are TOP-LEVEL subcommands:
  word      single-token swap, format-agnostic
  phrase    structural name (JSON/YAML key, env var, markdown heading,
            TOML section) OR exact text — in-place, no line munging
  sentence  smallest independent unit (whole containing line)
  paragraph full blank-line block

Omit --replace to delete the resolved scope. --json anywhere for
machine-readable output.
"#);
}
