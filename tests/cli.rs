//! Black-box integration tests for the runuz CLI binary.
//!
//! Each test invokes the compiled `runuz` binary as a subprocess,
//! passing CLI arguments and asserting on stdout/stderr/exit code.

use std::process::Command;

fn runuz(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_runuz"))
        .args(args)
        .output()
        .expect("failed to run runuz binary");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn tmp_file(content: &str, ext: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut f = tempfile::Builder::new()
        .suffix(&format!(".{ext}"))
        .tempfile()
        .expect("tempfile");
    f.write_all(content.as_bytes()).expect("write");
    f.flush().ok();
    f
}

// ── read ──────────────────────────────────────────────────────────────

#[test]
fn read_rust_file_shows_outline() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read should succeed");
    assert!(stdout.contains("alpha"), "should find alpha: {stdout}");
    assert!(stdout.contains("beta"), "should find beta: {stdout}");
}

#[test]
fn read_symbol_scoped() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha"]);
    assert!(ok, "read symbol should succeed");
    assert!(stdout.contains("alpha"), "should contain alpha: {stdout}");
    assert!(!stdout.contains("beta"), "should not contain beta: {stdout}");
}

#[test]
fn read_pattern_search() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--pattern", "beta"]);
    assert!(ok, "read pattern should succeed");
    assert!(stdout.contains("beta"), "should find beta: {stdout}");
}

#[test]
fn read_query_fuzzy() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--query", "alph"]);
    assert!(ok, "read query should succeed");
    assert!(stdout.contains("alpha"), "should find alpha: {stdout}");
}

#[test]
fn read_missing_file_errors() {
    let (ok, _, stderr) = runuz(&["read", "--file-path", "/nonexistent/file.rs"]);
    assert!(!ok, "read missing file should fail");
    assert!(stderr.contains("No files resolved") || stderr.contains("not found"), "should mention missing: {stderr}");
}

#[test]
fn read_json_output() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--json"]);
    assert!(ok, "read json should succeed");
    assert!(stdout.contains("\"is_error\""), "should be JSON: {stdout}");
    assert!(stdout.contains("\"output\""), "should have output field: {stdout}");
}

#[test]
fn read_outline_shows_callees() {
    let f = tmp_file("fn helper() {}\nfn caller() { helper(); helper(); }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read should succeed");
    assert!(stdout.contains("fn caller"), "caller missing: {stdout}");
    let caller_line = stdout.lines().find(|l| l.contains("fn caller")).unwrap();
    assert!(caller_line.contains("→ helper"), "callee suffix missing: {caller_line}");
    let helper_line = stdout.lines().find(|l| l.contains("fn helper")).unwrap();
    assert!(!helper_line.contains('→'), "helper should be silent: {helper_line}");
}

#[test]
fn read_symbol_shows_calls_line() {
    let f = tmp_file("fn helper() {}\nfn caller() { helper(); }\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--symbol", "caller"]);
    assert!(ok, "read symbol should succeed");
    assert!(stdout.contains("calls: helper"), "calls line missing: {stdout}");
    assert!(stdout.contains("fn caller()"), "code missing: {stdout}");
}

#[test]
fn read_outline_hides_macros_from_callees() {
    let f = tmp_file("fn caller() { println!(\"x\"); helper(); }\nfn helper() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read should succeed");
    let caller_line = stdout.lines().find(|l| l.contains("fn caller")).unwrap();
    assert!(caller_line.contains("→ helper"), "helper missing: {caller_line}");
    assert!(!caller_line.contains("println"), "macro leaked: {caller_line}");
}

// ── create ────────────────────────────────────────────────────────────

#[test]
fn create_writes_file() {
    let f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
    let path = f.path().to_path_buf();
    std::fs::remove_file(&path).ok();
    let (ok, stdout, _) = runuz(&["create", "--file-path", path.to_str().unwrap(), "--new-source", "fn main() {}\n"]);
    assert!(ok, "create should succeed: {stdout}");
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "fn main() {}\n");
}

#[test]
fn create_refuses_existing_file() {
    let f = tmp_file("fn existing() {}\n", "rs");
    let (ok, _, stderr) = runuz(&["create", "--file-path", f.path().to_str().unwrap(), "--new-source", "fn new() {}\n"]);
    assert!(!ok, "create should refuse existing file");
    assert!(stderr.contains("already exists") || stderr.contains("already exists"), "should mention existing: {stderr}");
}

#[test]
fn create_rejects_bad_syntax() {
    let f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
    let path = f.path().to_path_buf();
    std::fs::remove_file(&path).ok();
    let (ok, _, stderr) = runuz(&["create", "--file-path", path.to_str().unwrap(), "--new-source", "fn x( { ;;"]);
    assert!(!ok, "create should reject bad syntax");
    assert!(stderr.contains("syntax error") || stderr.contains("rejected"), "should mention syntax: {stderr}");
}

// ── replace ───────────────────────────────────────────────────────────

#[test]
fn replace_symbol_scoped() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "fn alpha() -> u32 { 99 }"]);
    assert!(ok, "replace should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("99"), "should replace alpha: {content}");
    assert!(content.contains("fn beta"), "should keep beta: {content}");
}

#[test]
fn replace_whole_file_when_no_symbol() {
    let f = tmp_file("fn old() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--new-source", "fn brand_new() -> u32 { 42 }\n"]);
    assert!(ok, "replace whole file should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("brand_new"), "should have new content: {content}");
    assert!(!content.contains("old"), "should not have old content: {content}");
}

#[test]
fn replace_rejects_broken_result() {
    let f = tmp_file("fn alpha() {}\n", "rs");
    let (ok, _, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "fn alpha( { ;;"]);
    assert!(!ok, "replace should reject broken result");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("fn alpha() {}"), "original should be untouched: {content}");
}

#[test]
fn replace_multi_symbols() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\nfn gamma() -> u32 { 3 }\n", "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbols", "alpha,beta", "--new-source", "fn combined() -> u32 { 99 }"]);
    assert!(ok, "multi replace should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("combined"), "should have combined: {content}");
    assert!(!content.contains("fn alpha"), "alpha gone: {content}");
    assert!(!content.contains("fn beta"), "beta gone: {content}");
    assert!(content.contains("fn gamma"), "gamma kept: {content}");
}

#[test]
fn replace_multi_rejects_missing_symbol() {
    let f = tmp_file("fn alpha() {}\nfn beta() {}\n", "rs");
    let (ok, _, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbols", "alpha,nonexistent", "--new-source", "fn x() {}\n"]);
    assert!(!ok, "should reject missing symbol");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(content, "fn alpha() {}\nfn beta() {}\n", "file untouched on partial failure");
}

#[test]
fn replace_allows_preexisting_errors() {
    // File with a pre-existing syntax error in an unrelated region
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta( { ;; }\n", "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "fn alpha() -> u32 { 99 }"]);
    assert!(ok, "replace should succeed despite pre-existing error: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("99"), "alpha replaced: {content}");
}

#[test]
fn replace_includes_attributes_in_range() {
    let f = tmp_file("#[derive(Debug)]\nfn alpha() -> u32 { 1 }\n", "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "#[derive(Debug)]\nfn alpha() -> u32 { 99 }\n"]);
    assert!(ok, "replace with attributes should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("99"), "should replace: {content}");
    assert!(content.contains("derive"), "should keep derive: {content}");
}

// ── insert_before / insert_after ─────────────────────────────────────

#[test]
fn insert_after_anchor() {
    let f = tmp_file("fn alpha() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["insert_after", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "fn beta() {}\n"]);
    assert!(ok, "insert_after should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("fn alpha"), "alpha kept: {content}");
    assert!(content.contains("fn beta"), "beta added: {content}");
}

#[test]
fn insert_before_anchor() {
    let f = tmp_file("fn beta() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["insert_before", "--file-path", f.path().to_str().unwrap(), "--symbol", "beta", "--new-source", "fn alpha() {}\n"]);
    assert!(ok, "insert_before should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("fn alpha"), "alpha added: {content}");
    assert!(content.contains("fn beta"), "beta kept: {content}");
}

// ── delete ────────────────────────────────────────────────────────────

#[test]
fn delete_symbol() {
    let f = tmp_file("fn alpha() {}\nfn beta() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha"]);
    assert!(ok, "delete should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(!content.contains("fn alpha"), "alpha gone: {content}");
    assert!(content.contains("fn beta"), "beta kept: {content}");
}

#[test]
fn delete_multi_symbols() {
    let f = tmp_file("fn alpha() {}\nfn beta() {}\nfn gamma() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbols", "alpha,beta"]);
    assert!(ok, "multi delete should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(!content.contains("fn alpha"), "alpha gone: {content}");
    assert!(!content.contains("fn beta"), "beta gone: {content}");
    assert!(content.contains("fn gamma"), "gamma kept: {content}");
}

#[test]
fn delete_ignores_unrelated_preexisting_error() {
    let f = tmp_file("pub const PRODUCT_NAME: &str = \"x\";\n\nunsafe extern \"C\" {\n    #[link_name = \"getuid\"]\n    safe fn getuid() -> u32;\n}\n", "rs");
    let (ok, stdout, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbol", "PRODUCT_NAME"]);
    assert!(ok, "delete should succeed despite pre-existing error: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(!content.contains("PRODUCT_NAME"), "PRODUCT_NAME gone: {content}");
}

// ── word / phrase / sentence / paragraph ─────────────────────────────

#[test]
fn word_swap() {
    let f = tmp_file("hello world\n", "txt");
    let (ok, stdout, _) = runuz(&["word", "world", "--file-path", f.path().to_str().unwrap(), "--replace", "universe"]);
    assert!(ok, "word swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("universe"), "should swap word: {content}");
}

#[test]
fn phrase_swap_in_markdown() {
    let f = tmp_file("# Old Title\n\nSome content.\n", "md");
    let (ok, stdout, _) = runuz(&["phrase", "Old Title", "--file-path", f.path().to_str().unwrap(), "--replace", "New Title"]);
    assert!(ok, "phrase swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("New Title"), "should swap phrase: {content}");
}

#[test]
fn sentence_swap() {
    let f = tmp_file("First line.\nSecond line.\n", "txt");
    let (ok, stdout, _) = runuz(&["sentence", "Second line.", "--file-path", f.path().to_str().unwrap(), "--replace", "Replaced line."]);
    assert!(ok, "sentence swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("Replaced line."), "should swap sentence: {content}");
}

#[test]
fn paragraph_delete() {
    let f = tmp_file("First paragraph.\n\nSecond paragraph.\n", "txt");
    let (ok, stdout, _) = runuz(&["paragraph", "First paragraph.", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "paragraph delete should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(!content.contains("First paragraph"), "first gone: {content}");
    assert!(content.contains("Second paragraph"), "second kept: {content}");
}

// ── tools ─────────────────────────────────────────────────────────────

#[test]
fn tools_lists_surface() {
    let (ok, stdout, _) = runuz(&["tools"]);
    assert!(ok, "tools should succeed");
    assert!(stdout.contains("runuz_read"), "should list read: {stdout}");
    assert!(stdout.contains("runuz_create"), "should list create: {stdout}");
    assert!(stdout.contains("runuz_replace"), "should list replace: {stdout}");
    assert!(!stdout.contains("runuz_write"), "should NOT list write: {stdout}");
}

#[test]
fn tools_json_output() {
    let (ok, stdout, _) = runuz(&["tools", "--json"]);
    assert!(ok, "tools json should succeed");
    assert!(stdout.contains("\"name\""), "should be JSON: {stdout}");
}

// ── error handling ────────────────────────────────────────────────────

#[test]
fn unknown_subcommand_errors() {
    let (ok, _, stderr) = runuz(&["frobnicate"]);
    assert!(!ok, "unknown subcommand should fail");
    assert!(stderr.contains("unknown") || stderr.contains("missing"), "should mention error: {stderr}");
}

#[test]
fn missing_file_path_errors() {
    let (ok, _, stderr) = runuz(&["read"]);
    assert!(!ok, "missing file-path should fail");
    assert!(stderr.contains("file-path") || stderr.contains("required"), "should mention missing: {stderr}");
}

#[test]
fn non_code_file_rejected_by_code_ops() {
    let f = tmp_file("# hello\n", "md");
    let (ok, _, stderr) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--new-source", "# bye\n"]);
    assert!(!ok, "replace on non-code should fail");
    assert!(stderr.contains("runuz_text") || stderr.contains("code files only"), "should mention routing: {stderr}");
}

#[test]
fn code_file_rejected_by_text_ops() {
    let f = tmp_file("fn alpha() {}\n", "rs");
    let (ok, _, stderr) = runuz(&["word", "alpha", "--file-path", f.path().to_str().unwrap(), "--replace", "beta"]);
    assert!(!ok, "word on code file should fail");
    assert!(stderr.contains("runuz_code") || stderr.contains("refuses"), "should mention routing: {stderr}");
}

// ── json output mode ──────────────────────────────────────────────────

#[test]
fn json_output_on_success() {
    let f = tmp_file("fn alpha() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--json"]);
    assert!(ok, "json read should succeed");
    assert!(stdout.contains("\"is_error\": false"), "should be JSON success: {stdout}");
}

#[test]
fn json_output_on_error() {
    let (ok, _, stderr) = runuz(&["read", "--file-path", "/nonexistent.rs", "--json"]);
    assert!(!ok, "json read missing should fail");
    assert!(stderr.contains("\"is_error\": true"), "should be JSON error: {stderr}");
}

// ── real-world file content ───────────────────────────────────────────

fn real_rust_file() -> String {
    r#"use std::collections::HashMap;
use std::io::{self, Write};

/// A user record with metadata.
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
}

impl User {
    pub fn new(id: u64, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            roles: Vec::new(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn add_role(&mut self, role: &str) {
        if !self.has_role(role) {
            self.roles.push(role.to_string());
        }
    }
}

pub fn process_users(users: &[User]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for user in users {
        for role in &user.roles {
            *counts.entry(role.clone()).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Alice", "alice@example.com");
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");
    }
}
"#.to_string()
}

#[test]
fn read_real_rust_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read real rust file should succeed");
    assert!(stdout.contains("User"), "should find User struct: {stdout}");
    assert!(stdout.contains("process_users"), "should find process_users: {stdout}");
    assert!(stdout.contains("has_role"), "should find has_role method: {stdout}");
}

#[test]
fn read_nested_symbol_in_real_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap(), "--symbol", "User.new"]);
    assert!(ok, "read User.new should succeed: {stdout}");
    assert!(stdout.contains("new"), "should find new method: {stdout}");
}

#[test]
fn replace_method_in_real_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "has_role", "--new-source", "pub fn has_role(&self, role: &str) -> bool {\n        self.roles.contains(&role.to_string())\n    }"]);
    assert!(ok, "replace has_role should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("contains"), "should use contains: {content}");
    assert!(content.contains("add_role"), "should keep add_role: {content}");
}

#[test]
fn delete_method_in_real_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbol", "add_role"]);
    assert!(ok, "delete add_role should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(!content.contains("add_role"), "add_role gone: {content}");
    assert!(content.contains("has_role"), "has_role kept: {content}");
}

#[test]
fn insert_after_method_in_real_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["insert_after", "--file-path", f.path().to_str().unwrap(), "--symbol", "has_role", "--new-source", "pub fn remove_role(&mut self, role: &str) {\n        self.roles.retain(|r| r != role);\n    }"]);
    assert!(ok, "insert_after should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("remove_role"), "remove_role added: {content}");
    assert!(content.contains("has_role"), "has_role kept: {content}");
}

#[test]
fn replace_imports_in_real_file() {
    let f = tmp_file(&real_rust_file(), "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "imports", "--new-source", "use std::collections::HashMap;\nuse std::path::PathBuf;"]);
    assert!(ok, "replace imports should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("PathBuf"), "should have PathBuf: {content}");
    assert!(!content.contains("std::io"), "old std::io import gone: {content}");
}

// ── edge cases ────────────────────────────────────────────────────────

#[test]
fn empty_file_read() {
    let f = tmp_file("", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read empty file should succeed");
    assert!(stdout.contains("0 bytes") || stdout.contains("0 lines"), "should show empty: {stdout}");
}

#[test]
fn whitespace_only_file() {
    let f = tmp_file("   \n\n   \n", "txt");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read whitespace file should succeed");
}

#[test]
fn file_with_bom() {
    let f = tmp_file("\u{feff}fn alpha() {}\n", "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read BOM file should succeed");
}

#[test]
fn large_file_read() {
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("fn func_{}() -> u32 {{ {} }}\n", i, i));
    }
    let f = tmp_file(&content, "rs");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read large file should succeed");
    assert!(stdout.contains("func_0"), "should find func_0: {stdout}");
    assert!(stdout.contains("func_999"), "should find func_999: {stdout}");
}

#[test]
fn large_file_replace() {
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!("fn func_{}() -> u32 {{ {} }}\n", i, i));
    }
    let f = tmp_file(&content, "rs");
    let (ok, stdout, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "func_50", "--new-source", "fn func_50() -> u32 { 999 }"]);
    assert!(ok, "replace in large file should succeed: {stdout}");
    let updated = std::fs::read_to_string(f.path()).unwrap();
    assert!(updated.contains("999"), "should replace func_50: {updated}");
    assert!(updated.contains("func_0"), "should keep func_0: {updated}");
    assert!(updated.contains("func_99"), "should keep func_99: {updated}");
}

// ── multi-language support ────────────────────────────────────────────

#[test]
fn python_file_operations() {
    let f = tmp_file("def hello():\n    print('hello')\n\nclass Greeter:\n    def greet(self):\n        return 'hi'\n", "py");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read python should succeed");
    assert!(stdout.contains("hello"), "should find hello: {stdout}");
    assert!(stdout.contains("Greeter"), "should find Greeter: {stdout}");
}

#[test]
fn typescript_file_operations() {
    let f = tmp_file("function hello(): string {\n  return 'hello';\n}\n\nclass Greeter {\n  greet(): string {\n    return 'hi';\n  }\n}\n", "ts");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read typescript should succeed");
    assert!(stdout.contains("hello"), "should find hello: {stdout}");
    assert!(stdout.contains("Greeter"), "should find Greeter: {stdout}");
}

#[test]
fn go_file_operations() {
    let f = tmp_file("package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"hello\")\n}\n", "go");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read go should succeed");
    assert!(stdout.contains("main"), "should find main: {stdout}");
}

#[test]
fn javascript_file_operations() {
    let f = tmp_file("function hello() {\n  return 'hello';\n}\n\nclass Greeter {\n  greet() {\n    return 'hi';\n  }\n}\n", "js");
    let (ok, stdout, _) = runuz(&["read", "--file-path", f.path().to_str().unwrap()]);
    assert!(ok, "read javascript should succeed");
    assert!(stdout.contains("hello"), "should find hello: {stdout}");
    assert!(stdout.contains("Greeter"), "should find Greeter: {stdout}");
}

// ── non-code file operations ──────────────────────────────────────────

#[test]
fn json_file_phrase_swap() {
    let f = tmp_file("{\"name\": \"old_name\", \"version\": \"1.0.0\"}\n", "json");
    let (ok, stdout, _) = runuz(&["phrase", "old_name", "--file-path", f.path().to_str().unwrap(), "--replace", "\"new_name\""]);
    assert!(ok, "json phrase swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("new_name"), "should swap name: {content}");
    assert!(content.contains("1.0.0"), "should keep version: {content}");
}

#[test]
fn toml_file_phrase_swap() {
    let f = tmp_file("[package]\nname = \"old_name\"\nversion = \"1.0.0\"\n", "toml");
    let (ok, stdout, _) = runuz(&["phrase", "old_name", "--file-path", f.path().to_str().unwrap(), "--replace", "new_name"]);
    assert!(ok, "toml phrase swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("new_name"), "should swap name: {content}");
}

#[test]
fn env_file_word_swap() {
    let f = tmp_file("DB_HOST=localhost\nDB_PORT=5432\n", "env");
    let (ok, stdout, _) = runuz(&["word", "localhost", "--file-path", f.path().to_str().unwrap(), "--replace", "production-db"]);
    assert!(ok, "env word swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("production-db"), "should swap host: {content}");
    assert!(content.contains("DB_PORT"), "should keep port: {content}");
}

#[test]
fn markdown_heading_phrase_swap() {
    let f = tmp_file("# Old Title\n\n## Section 1\n\nContent here.\n", "md");
    let (ok, stdout, _) = runuz(&["phrase", "Old Title", "--file-path", f.path().to_str().unwrap(), "--replace", "New Title"]);
    assert!(ok, "markdown phrase swap should succeed: {stdout}");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert!(content.contains("New Title"), "should swap title: {content}");
    assert!(content.contains("Section 1"), "should keep section: {content}");
}

// ── error recovery ────────────────────────────────────────────────────

#[test]
fn replace_preserves_file_on_syntax_error() {
    let f = tmp_file("fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n", "rs");
    let original = std::fs::read_to_string(f.path()).unwrap();
    let (ok, _, _) = runuz(&["replace", "--file-path", f.path().to_str().unwrap(), "--symbol", "alpha", "--new-source", "fn alpha( { ;;"]);
    assert!(!ok, "replace should fail");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(content, original, "file should be untouched after failed replace");
}

#[test]
fn delete_preserves_file_on_missing_symbol() {
    let f = tmp_file("fn alpha() {}\n", "rs");
    let original = std::fs::read_to_string(f.path()).unwrap();
    let (ok, _, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbol", "nonexistent"]);
    assert!(!ok, "delete should fail");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(content, original, "file should be untouched after failed delete");
}

#[test]
fn multi_delete_atomic_rollback() {
    let f = tmp_file("fn alpha() {}\nfn beta() {}\nfn gamma() {}\n", "rs");
    let original = std::fs::read_to_string(f.path()).unwrap();
    let (ok, _, _) = runuz(&["delete", "--file-path", f.path().to_str().unwrap(), "--symbols", "alpha,nonexistent"]);
    assert!(!ok, "multi delete should fail");
    let content = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(content, original, "file should be untouched after partial failure");
}

// ── glob and directory operations ─────────────────────────────────────

#[test]
fn read_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn beta() {}\n").unwrap();
    let (ok, stdout, _) = runuz(&["read", "--file-path", dir.path().to_str().unwrap()]);
    assert!(ok, "read directory should succeed");
    assert!(stdout.contains("a.rs") || stdout.contains("alpha"), "should find a.rs: {stdout}");
}

#[test]
fn read_glob() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn beta() {}\n").unwrap();
    let pattern = format!("{}/**/*.rs", dir.path().display());
    let (ok, stdout, _) = runuz(&["read", "--file-path", &pattern]);
    assert!(ok, "read glob should succeed");
    // Multi-file glob returns inventory view
    assert!(stdout.contains("a.rs") || stdout.contains("b.rs"), "should list files: {stdout}");
}

#[test]
fn read_pattern_across_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn alpha() -> u32 { 1 }\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn beta() -> u32 { 2 }\n").unwrap();
    let pattern = format!("{}/**/*.rs", dir.path().display());
    let (ok, stdout, _) = runuz(&["read", "--file-path", &pattern, "--pattern", "fn.*"]);
    assert!(ok, "read pattern should succeed");
    assert!(stdout.contains("alpha") || stdout.contains("beta"), "should find matches: {stdout}");
}
