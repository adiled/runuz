//! Command-line parsing for the runuz binary — a small hand-rolled
//! parser (no clap dependency; keeps the standalone crate lean).

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
    pub word: Option<String>,
    pub phrase: Option<String>,
    pub sentence: Option<String>,
    pub paragraph: Option<String>,
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

        // Remaining tokens: `--key value` pairs (with `--json` allowed
        // anywhere).
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
            "do_nocode" | "do-nocode" => Command::DoNonCode(do_noncode_args(&pairs)?),
            other => bail!("unknown subcommand {other:?} — use read, do_code, or do_nocode"),
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

fn do_noncode_args(pairs: &[(String, String)]) -> Result<DoNonCodeArgs> {
    Ok(DoNonCodeArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        word: opt(pairs, "word"),
        phrase: opt(pairs, "phrase"),
        sentence: opt(pairs, "sentence"),
        paragraph: opt(pairs, "paragraph"),
        replace: opt(pairs, "replace"),
    })
}

fn help() {
    print!(
        "runuz — standalone filesystem tool: do_code / do_nocode / do_read\n\n\
         USAGE:\n\
         \x20 runuz [--json] read --file-path <path> [--symbol S] [--query Q] [--pattern RE]\n\
         \x20 runuz [--json] do_code --file-path <path> [--operation op] [--symbol S] [--new-source TEXT]\n\
         \x20 runuz [--json] do_nocode --file-path <path> [--word W | --phrase P | --sentence S | --paragraph P] [--replace TEXT]\n\n\
         operations (do_code): create | replace | insert_before | insert_after | delete\n"
    );
}
