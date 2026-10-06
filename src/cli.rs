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
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct InsertArgs {
    pub file_path: String,
    pub anchor: String,
    pub symbol: Option<String>,
    pub new_source: Option<String>,
}

#[derive(Debug)]
pub struct DeleteArgs {
    pub file_path: String,
    pub symbol: Option<String>,
    pub symbols: Option<String>,
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

fn addr_flags(pairs: &[(String, String)]) -> Result<(Option<String>, Option<String>)> {
    let symbol = opt(pairs, "symbol");
    let symbols = opt(pairs, "symbols");
    if symbol.is_some() && symbols.is_some() {
        bail!("give exactly one of --symbol, --symbols");
    }
    Ok((symbol, symbols))
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
    let allowed = ["file-path", "symbol", "symbols", "new-source"];
    for (k, _) in pairs {
        if !allowed.contains(&k.as_str()) {
            bail!("unknown flag --{k} for replace. Allowed: --file-path, --symbol, --symbols, --new-source");
        }
    }
    let (symbol, symbols) = addr_flags(pairs)?;
    Ok(ReplaceArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol,
        symbols,
        new_source: opt(pairs, "new-source"),
    })
}

fn insert_args(pairs: &[(String, String)], anchor: &str) -> Result<InsertArgs> {
    for (k, _) in pairs {
        if !["file-path", "symbol", "new-source"].contains(&k.as_str()) {
            bail!("unknown flag --{k} for insert_{anchor}. Allowed: --file-path, --symbol, --new-source");
        }
    }
    let symbol = opt(pairs, "symbol");
    if symbol.is_none() {
        bail!("insert_{anchor} needs an anchor: --symbol NAME or --symbol 'rung <text>'");
    }
    Ok(InsertArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        anchor: anchor.to_string(),
        symbol,
        new_source: opt(pairs, "new-source"),
    })
}

fn delete_args(pairs: &[(String, String)]) -> Result<DeleteArgs> {
    for (k, _) in pairs {
        if !["file-path", "symbol", "symbols"].contains(&k.as_str()) {
            bail!("unknown flag --{k} for delete. Allowed: --file-path, --symbol, --symbols");
        }
    }
    let (symbol, symbols) = addr_flags(pairs)?;
    if symbol.is_none() && symbols.is_none() {
        bail!("delete needs a target: --symbol S or --symbols A,B");
    }
    Ok(DeleteArgs {
        file_path: req(pairs, "file-path")?.to_string(),
        symbol,
        symbols,
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
  runuz read --file-path <path> [--symbol ADDR] [--query Q] [--pattern RE]
  runuz create  --file-path <path> [--new-source T]
  runuz replace --file-path <path> [--symbol ADDR | --symbols A,B] [--new-source T]
  runuz insert_before --file-path <path> --symbol ADDR [--new-source T]
  runuz insert_after  --file-path <path> --symbol ADDR [--new-source T]
  runuz delete --file-path <path> (--symbol ADDR | --symbols A,B)
  runuz rename --file-path <path> --symbol NAME --new-name Y
  runuz tools [--json]                      (advertised tool surface)

--symbol addresses ANY unit of ANY file, one grammar:

  NAME              the named unit (code): fn/Class.method/imports,
                    sub-walks 'alpha.body', 'alpha.when.otherwise#2'
  'token <t>'       the first word-boundary occurrence of <t>
  'slot <s>'        the named value: JSON key-path, YAML/TOML/env key,
                    markdown heading; falls back to exact substring
  'statement <s>'   the line containing <s>
  'block <s>'       the blank-line paragraph containing <s>

Code files validate against their grammar; text files validate
structure (JSON stays JSON). No address on replace = whole file.
'symbols A,B' = one atomic multi-edit over code symbols.
Empty --new-source deletes the addressed range. --json anywhere
for machine-readable output.
"#);
    if install_man_page() {
        println!("\nnow you can also read man runuz");
    }
}

fn version() {
    println!("runuz {}", env!("CARGO_PKG_VERSION"));
}

static MAN_PAGE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/man/runuz.1"));

fn install_man_page() -> bool {
    if std::env::consts::OS == "windows" {
        return false;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let cargo_home = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{home}/.cargo"));
    if home.is_empty() || cargo_home.is_empty() {
        return false;
    }
    let mut ok = false;
    for dir in [
        format!("{cargo_home}/share/man/man1"),
        format!("{home}/.local/share/man/man1"),
    ] {
        if write_man_page(&dir, true) {
            ok = true;
        }
    }
    for dir in [
        "/opt/homebrew/share/man/man1",
        "/usr/local/share/man/man1",
    ] {
        if std::path::Path::new(dir).is_dir() && write_man_page(dir, false) {
            ok = true;
        }
    }
    ok
}

fn write_man_page(dir: &str, create: bool) -> bool {
    let path = std::path::Path::new(dir).join("runuz.1");
    if let Ok(existing) = std::fs::read(&path) {
        return existing == MAN_PAGE.as_bytes();
    }
    let create_dir = if create {
        std::fs::create_dir_all(dir)
    } else {
        Ok(())
    };
    create_dir
        .and_then(|_| std::fs::write(&path, MAN_PAGE))
        .is_ok()
}