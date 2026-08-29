//! runuz — the standalone filesystem CLI.
//!
//! `do_code`, `do_nocode`, `do_read` for any project on Earth —
//! AST-grounded via tree-sitter, zero hum dependencies. Each
//! subcommand maps 1:1 onto the tool body, taking the same args the
//! humfs tool surface takes, and prints the tool result (output,
//! plus title/metadata on --json).

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
            let mut args = serde_json::json!({
                "file_path": c.file_path,
            });
            // serde_json::json! can't take optionals cleanly; build by hand.
            if let Some(v) = c.word { args["word"] = serde_json::json!(v); }
            if let Some(v) = c.phrase { args["phrase"] = serde_json::json!(v); }
            if let Some(v) = c.sentence { args["sentence"] = serde_json::json!(v); }
            if let Some(v) = c.paragraph { args["paragraph"] = serde_json::json!(v); }
            if let Some(v) = c.replace { args["replace"] = serde_json::json!(v); }
            let res = runuz::tools::do_noncode(args).await;
            print_result(&res, json);
        }
    }
    Ok(if json { ExitCode::SUCCESS } else { ExitCode::SUCCESS })
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
