//! runuz — the standalone filesystem CLI.
//!
//! `read`, `do_code`, and the four linguistic scopes (`word`, `phrase`,
//! `sentence`, `paragraph`) for any project on Earth — AST-grounded via
//! tree-sitter, zero hum dependencies. The four scopes are top-level
//! subcommands so they're discoverable and hard to forget.
//!
//!   runuz read --file-path <path> [--symbol S] [--query Q] [--pattern RE]
//!   runuz do_code --file-path <path> [--operation op] [--symbol S] [--new-source T]
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
        cli::Command::DoCode(c) => {
            let res = runuz::tools::do_code(serde_json::json!({
                "file_path": c.file_path,
                "operation": c.operation,
                "symbol": c.symbol,
                "new_source": c.new_source,
            })).await;
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
