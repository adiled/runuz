//! runuz — the standalone filesystem CLI.
//!
//! `read`, the five code operations (`create`, `replace`,
//! `insert_before`, `insert_after`, `delete`), and the four linguistic
//! scopes (`word`, `phrase`, `sentence`, `paragraph`) for any project on
//! Earth — AST-grounded via tree-sitter, zero hum dependencies. All tool
//! operations are top-level subcommands so they're discoverable and hard
//! to forget.
//!
//!   runuz read --file-path <path> [--symbol S] [--query Q] [--pattern RE]
//!   runuz create  --file-path <path> [--new-source T]
//!   runuz replace --file-path <path> [--symbol S] [--new-source T]
//!   runuz insert_before --file-path <path> --symbol S [--new-source T]
//!   runuz insert_after  --file-path <path> --symbol S [--new-source T]
//!   runuz delete --file-path <path> --symbol S
//!   runuz word <scope> --file-path <path> [--replace T]
//!   runuz phrase <scope> --file-path <path> [--replace T]
//!   runuz sentence <scope> --file-path <path> [--replace T]
//!   runuz paragraph <scope> --file-path <path> [--replace T]

mod cli;

use anyhow::Result;
use std::process::ExitCode;

fn main() -> Result<ExitCode> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run())
}

async fn run() -> Result<ExitCode> {
    let args = cli::Args::parse()?;
    let json = args.json;
    match args.cmd {
        cli::Command::Read(c) => {
            let res = runuz::tools::read(serde_json::json!({
                "file_path": c.file_path,
                "symbol": c.symbol,
                "query": c.query,
                "pattern": c.pattern,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Create(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "create",
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Replace(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "replace",
                "symbol": c.symbol,
                "symbols": c.symbols,
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Insert(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": if c.anchor == "before" { "insert_before" } else { "insert_after" },
                "symbol": c.symbol,
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Delete(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "delete",
                "symbol": c.symbol,
                "symbols": c.symbols,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Tools => {
            let defs = runuz::tools::surface::advertised_defs();
            if json {
                println!("{}", serde_json::to_string_pretty(
                    &runuz::tools::surface::surface_json()).unwrap());
            } else {
                println!("runuz tools — {} advertised tools", defs.len());
                for d in &defs {
                    println!("  {}", d.name);
                }
            }
        }
        cli::Command::DoNonCode(c) => {
            let mut args = serde_json::json!({
                "file_path": c.file_path,
            });
            args[c.scope.clone()] = serde_json::json!(c.scope_text);
            if let Some(v) = c.replace { args["replace"] = serde_json::json!(v); }
            let res = runuz::tools::text(args).await;
            print_result(&res, json);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_result(res: &runuz::ToolResult, json: bool) {
    if json {
        let v = serde_json::json!({
            "is_error": res.is_error,
            "output": res.output,
            "title": res.title,
            "metadata": res.metadata,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        if let Some(t) = &res.title {
            println!("{t}");
        }
        println!("{}", res.output);
    }
}