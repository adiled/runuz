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
    Rename(RenameArgs),
    Tools,
}

#[derive(Debug)]
pub struct ReadArgs {
    pub file_path: String,
    pub symbol: Option<String>,
    pub scope: Option<String>,
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
    pub symbol: Option<String>,
    pub symbols: Option<String>,
    pub scope: Option<String>,
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct InsertArgs {
    pub file_path: String,
    pub anchor: String,
    pub symbol: Option<String>,
    pub scope: Option<String>,
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct DeleteArgs {
    pub file_path: String,
    pub symbol: Option<String>,
    pub symbols: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug)]
pub struct RenameArgs {
    pub file_path: String,
    pub symbol: Option<String>,
    pub new_name: Option<String>,
}

impl Args {
    pub fn parse() -> Result<Args> {
        let mut it = std::env::args().skip(1).peekable();
        let mut json = false;
        let mut sub: Option<String> = None;

        while let Some(a) = it.next() {
            match a.as_str() {
                "--json" => json = true,
                "--version" | "-V" => { version(); std::process::exit(0); }
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
        while let Some(a) = it.next() {
            match a.as_str() {
                "--json" => json = true,
                "--version" | "-V" => { version(); std::process::exit(0); }
                "-h" | "--help" => { help(); std::process::exit(0); }
                _ => {
                    if let Some(key) = a.strip_prefix("--") {
                        let val = it.next()
                            .ok_or_else(|| anyhow::anyhow!("flag {a} needs a value"))?;
                        pairs.push((key.to_string(), val));
                    } else {
                        bail!("unexpected positional argument {a:?} - runuz takes flags only");
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
            "rename" => Command::Rename(rename_args(&pairs)?),
            "tools" => Command::Tools,
            other => bail!("unknown subcommand {other:?} - use read, create, replace, insert_before, insert_after, delete, rename, or tools"),
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

fn addr_flags(pairs: &[(String, String)]) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let symbol = opt(pairs, "symbol");
    let symbols = opt(pairs, "symbols");
    let scope = opt(pairs, "scope");
    let seen = [symbol.is_some(), symbols.is_some(), scope.is_some()].into_iter().filter(|b| *b).count();
    if seen > 1 {
        bail!("give exactly one of --symbol, --symbols, --scope");
    }
    Ok((symbol, symbols, scope))
}

fn read_args(pairs: &[(String, String)]) -> Result<ReadArgs> {
    Ok(ReadArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol: opt(pairs, "symbol"),
        scope: opt(pairs, "scope"),
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
    let allowed = ["file-path", "symbol", "symbols", "scope", "new-source"];
    for (k, _) in pairs {
        if !allowed.contains(&k.as_str()) {
            bail!("unknown flag --{k} for replace. Allowed: --file-path, --symbol, --symbols, --scope, --new-source");
        }
    }
    let (symbol, symbols, scope) = addr_flags(pairs)?;
    Ok(ReplaceArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol,
        symbols,
        scope,
        new_source: opt(pairs, "new-source"),
    })
}

fn insert_args(pairs: &[(String, String)], anchor: &str) -> Result<InsertArgs> {
    for (k, _) in pairs {
        if !["file-path", "symbol", "scope", "new-source"].contains(&k.as_str()) {
            bail!("unknown flag --{k} for insert_{anchor}. Allowed: --file-path, --symbol, --scope, --new-source");
        }
    }
    let symbol = opt(pairs, "symbol");
    let scope = opt(pairs, "scope");
    match (&symbol, &scope) {
        (None, None) => bail!("insert_{anchor} needs an anchor: --symbol S or --scope 'rung text'"),
        (Some(_), Some(_)) => bail!("give exactly one of --symbol or --scope"),
        _ => {}
    }
    Ok(InsertArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        anchor: anchor.to_string(),
        symbol,
        scope,
        new_source: opt(pairs, "new-source"),
    })
}

fn delete_args(pairs: &[(String, String)]) -> Result<DeleteArgs> {
    for (k, _) in pairs {
        if !["file-path", "symbol", "symbols", "scope"].contains(&k.as_str()) {
            bail!("unknown flag --{k} for delete. Allowed: --file-path, --symbol, --symbols, --scope");
        }
    }
    let (symbol, symbols, scope) = addr_flags(pairs)?;
    if symbol.is_none() && symbols.is_none() && scope.is_none() {
        bail!("delete needs a target: --symbol S, --symbols A,B, or --scope 'rung text'");
    }
    Ok(DeleteArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol,
        symbols,
        scope,
    })
}

fn rename_args(pairs: &[(String, String)]) -> Result<RenameArgs> {
    Ok(RenameArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol: opt(pairs, "symbol"),
        new_name: opt(pairs, "new-name"),
    })
}

fn help() {
    println!(r#"runuz - the standalone filesystem CLI

USAGE:
  runuz read --file-path <path> [--symbol S] [--scope "rung TEXT"] [--query Q] [--pattern RE]
  runuz create  --file-path <path> [--new-source T]
  runuz replace --file-path <path> (--symbol S | --symbols A,B | --scope "rung TEXT") [--new-source T]
  runuz insert_before --file-path <path> (--symbol S | --scope "rung TEXT") [--new-source T]
  runuz insert_after  --file-path <path> (--symbol S | --scope "rung TEXT") [--new-source T]
  runuz delete --file-path <path> (--symbol S | --symbols A,B | --scope "rung TEXT")
  runuz rename --file-path <path> --symbol S --new-name Y
  runuz tools [--json]                      (advertised tool surface)

One addressing ladder, two flags:
  --symbol <name>   the named unit (code: fn/main/Class.method/imports).
  --scope "<rung> <text>"   the orthographic rungs of any file:
                  token 'a' | slot 'a.b' (JSON/YAML/TOML/env key or
                  markdown heading) | statement '…' (its line) |
                  block '…' (its blank-line paragraph). Text files
                  only; code files address by --symbol.

Operations:
  create        new file (fails if it exists); validates code syntax
  replace       swap the addressed unit's range (omitted address = whole
                file); --new-source "" deletes the range
  insert_before / insert_after   splice new_source at the unit edge
  delete        drop the unit's range (with blank-line hygiene for code)
  rename        rename the symbol's name across the file

--symbol accepts dot-nested (e.g. 'Class.method'); 'imports' addresses
the synthetic top-of-file import block. --json anywhere for
machine-readable output.
"#);
}

fn version() {
    println!("runuz {}", env!("CARGO_PKG_VERSION"));
}