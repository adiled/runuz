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

use std::process::ExitCode;

use anyhow::Result;
use runuz::tools::ToolResult;

mod cli;

fn main() -> Result<ExitCode> {
    // The tool bodies are async (they match the forager contract);
    // build a small runtime so we can await them.
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
            let res = runuz::tools::do_code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "create",
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Replace(c) => {
            let res = runuz::tools::do_code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "replace",
                "symbol": c.symbol,
                "symbols": c.symbols,
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Insert(c) => {
            let res = runuz::tools::do_code(serde_json::json!({
                "file_path": c.file_path,
                "operation": if c.anchor == "before" { "insert_before" } else { "insert_after" },
                "symbol": c.symbol,
                "new_source": c.new_source,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Delete(c) => {
            let res = runuz::tools::do_code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "delete",
                "symbol": c.symbol,
                "symbols": c.symbols,
            })).await;
            print_result(&res, json);
        }
        cli::Command::Write(c) => {
            // Whole-file write, auto-routed by extension:
            //   code file   -> do_code replace (no symbol => whole-file)
            //   non-code    -> do_nocode with no scope => write_whole_file
            let is_code = runuz::ast::is_code_file(std::path::Path::new(&c.file_path));
            let res = if is_code {
                runuz::tools::do_code(serde_json::json!({
                    "file_path": c.file_path,
                    "operation": "replace",
                    "new_source": c.content,
                })).await
            } else {
                runuz::tools::do_noncode(serde_json::json!({
                    "file_path": c.file_path,
                    "replace": c.content,
                })).await
            };
            print_result(&res, json);
        }
        cli::Command::DoNonCode(c) => {
            // The scope subcommand IS the scope parameter: map
            // `word|phrase|sentence|paragraph` onto the tool's scope key.
            let mut args = serde_json::json!({
                "file_path": c.file_path,
            });
            args[c.scope.clone()] = serde_json::json!(c.scope_text);
            if let Some(v) = c.replace { args["replace"] = serde_json::json!(v); }
            let res = runuz::tools::do_noncode(args).await;
            print_result(&res, json);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_result(res: &ToolResult, json: bool) {
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
