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
    let mut had_error = false;
    match args.cmd {
        cli::Command::Read(c) => {
            let res = runuz::tools::read(serde_json::json!({
                "file_path": c.file_path,
                "symbol": c.symbol,
                "scope": c.scope,
                "query": c.query,
                "pattern": c.pattern,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Create(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "create",
                "new_source": c.new_source,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Replace(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "replace",
                "symbol": c.symbol,
                "symbols": c.symbols,
                "scope": c.scope,
                "new_source": c.new_source,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Insert(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": if c.anchor == "before" { "insert_before" } else { "insert_after" },
                "symbol": c.symbol,
                "scope": c.scope,
                "new_source": c.new_source,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Delete(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "delete",
                "symbol": c.symbol,
                "symbols": c.symbols,
                "scope": c.scope,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Rename(c) => {
            let res = runuz::tools::code(serde_json::json!({
                "file_path": c.file_path,
                "operation": "rename",
                "symbol": c.symbol,
                "new_name": c.new_name,
            })).await;
            if res.is_error { had_error = true; }
            print_result(&res, json);
        }
        cli::Command::Tools => {
            let defs = runuz::tools::surface::advertised_defs();
            if json {
                println!("{}", serde_json::to_string_pretty(
                    &runuz::tools::surface::surface_json()).unwrap());
            } else {
                println!("runuz tools - {} advertised tools", defs.len());
                for d in &defs {
                    println!("  {}", d.name);
                }
            }
        }
    }
    if had_error {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn print_result(res: &runuz::ToolResult, json: bool) {
    if json {
        let v = serde_json::json!({
            "is_error": res.is_error,
            "output": res.output,
            "title": res.title,
            "metadata": res.metadata,
        });
        if res.is_error {
            eprintln!("{}", serde_json::to_string_pretty(&v).unwrap());
        } else {
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
    } else {
        if let Some(t) = &res.title {
            if res.is_error {
                eprintln!("{t}");
            } else {
                println!("{t}");
            }
        }
        if res.is_error {
            eprintln!("{}", res.output);
        } else {
            println!("{}", res.output);
        }
    }
}