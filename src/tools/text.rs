use std::path::Path;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TextScope {
    Token,
    Slot,
    Statement,
    Block,
}

impl TextScope {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            TextScope::Token => "token",
            TextScope::Slot => "slot",
            TextScope::Statement => "statement",
            TextScope::Block => "block",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<(TextScope, &str)> {
        let (rung, rest) = s.split_once(' ')?;
        let scope = match rung {
            "token" => TextScope::Token,
            "slot" => TextScope::Slot,
            "statement" => TextScope::Statement,
            "block" => TextScope::Block,
            _ => return None,
        };
        Some((scope, rest))
    }
}

pub(crate) fn resolve(source: &str, path: &Path, scope: TextScope, text: &str) -> Option<(usize, usize)> {
    match scope {
        TextScope::Token => resolve_token(source, text),
        TextScope::Slot => resolve_slot(source, text, path),
        TextScope::Statement => resolve_statement(source, text),
        TextScope::Block => resolve_block(source, text),
    }
    .map(|m| (m.start, m.end))
}

struct Match {
    start: usize,
    end: usize,
}

fn resolve_token(source: &str, token: &str) -> Option<Match> {
    let re = Regex::new(&format!(r"\b{}\b", regex::escape(token))).ok()?;
    let m = re.find(source)?;
    Some(Match { start: m.start(), end: m.end() })
}

fn resolve_slot(source: &str, slot: &str, path: &Path) -> Option<Match> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let by_format = match ext.as_str() {
        "json" | "jsonc" => slot_json(source, slot),
        "yaml" | "yml"   => slot_yaml(source, slot),
        "toml"           => slot_toml(source, slot),
        "env"            => slot_env(source, slot),
        "md" | "markdown" => slot_markdown(source, slot),
        _ => None,
    };
    if by_format.is_some() { return by_format; }
    let base = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if base.starts_with(".env") {
        if let Some(m) = slot_env(source, slot) { return Some(m); }
    }
    let idx = source.find(slot)?;
    Some(Match { start: idx, end: idx + slot.len() })
}

fn resolve_statement(source: &str, statement: &str) -> Option<Match> {
    let idx = source.find(statement)?;
    let line_start = source[..idx].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end_rel = source[idx..].find('\n').map(|p| idx + p + 1).unwrap_or(source.len());
    Some(Match { start: line_start, end: line_end_rel })
}

fn resolve_block(source: &str, block: &str) -> Option<Match> {
    let idx = source.find(block)?;
    let start = find_block_start(source, idx);
    let end = find_block_end(source, idx);
    Some(Match { start, end })
}

fn find_block_start(source: &str, idx: usize) -> usize {
    let mut start = source[..idx].rfind("\n\n").map(|p| p + 2).unwrap_or(0);
    while start < idx && source.as_bytes().get(start).map_or(false, |b| *b == b' ' || *b == b'\t') {
        start += 1;
    }
    if start > idx { idx } else { start }
}

fn find_block_end(source: &str, idx: usize) -> usize {
    source[idx..].find("\n\n")
        .map(|p| idx + p + 1)
        .unwrap_or(source.len())
}

fn slot_json(source: &str, slot: &str) -> Option<Match> {
    let value_pattern = format!("\"{}\"", slot);
    let mut search_start = 0;
    while let Some(idx) = source[search_start..].find(&value_pattern) {
        let abs_idx = search_start + idx;
        let after = abs_idx + value_pattern.len();
        let is_key = source[after..].trim_start().starts_with(':');
        if !is_key {
            return Some(Match { start: abs_idx, end: abs_idx + value_pattern.len() });
        }
        search_start = abs_idx + 1;
    }
    let segs: Vec<&str> = slot.split('.').collect();
    let mut cursor = 0;
    let mut last_value_range: Option<(usize, usize)> = None;
    for seg in segs {
        let key_pattern = format!("\"{}\"", seg);
        let key_idx = source[cursor..].find(&key_pattern).map(|p| cursor + p)?;
        let after_key = key_idx + key_pattern.len();
        let colon_off = source[after_key..].find(':')?;
        let mut value_start = after_key + colon_off + 1;
        while value_start < source.len() && source.as_bytes()[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        let value_end = scan_json_value(source, value_start)?;
        cursor = value_start;
        last_value_range = Some((value_start, value_end));
    }
    last_value_range.map(|(s, e)| Match { start: s, end: e })
}

fn scan_json_value(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let first = *bytes.get(start)?;
    match first {
        b'"' => {
            let mut i = start + 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' { i += 2; continue; }
                if bytes[i] == b'"' { return Some(i + 1); }
                i += 1;
            }
            None
        }
        b'{' | b'[' => {
            let close = if first == b'{' { b'}' } else { b']' };
            let mut depth = 1;
            let mut i = start + 1;
            let mut in_str = false;
            while i < bytes.len() {
                let b = bytes[i];
                if in_str {
                    if b == b'\\' { i += 2; continue; }
                    if b == b'"' { in_str = false; }
                } else {
                    if b == b'"' { in_str = true; }
                    else if b == first { depth += 1; }
                    else if b == close {
                        depth -= 1;
                        if depth == 0 { return Some(i + 1); }
                    }
                }
                i += 1;
            }
            None
        }
        _ => {
            let mut i = start;
            while i < bytes.len() {
                let b = bytes[i];
                if b == b',' || b.is_ascii_whitespace() || b == b'}' || b == b']' { break; }
                i += 1;
            }
            if i == start { None } else { Some(i) }
        }
    }
}

fn slot_yaml(source: &str, slot: &str) -> Option<Match> {
    let re = Regex::new(&format!(r"(?m)^([ \t]*){}[ \t]*:[ \t]*", regex::escape(slot))).ok()?;
    let m = re.find(source)?;
    let key_indent_len = m.as_str().chars().take_while(|c| c.is_whitespace()).count();
    let value_start = m.end();
    let line_end = source[value_start..].find('\n').map(|p| value_start + p).unwrap_or(source.len());
    let inline = source[value_start..line_end].trim();
    if !inline.is_empty() {
        return Some(Match { start: value_start, end: line_end });
    }
    let mut end = line_end + 1;
    while end < source.len() {
        let next_line_end = source[end..].find('\n').map(|p| end + p).unwrap_or(source.len());
        let line = &source[end..next_line_end];
        let line_indent_len = line.chars().take_while(|c| c.is_whitespace()).count();
        if line.trim().is_empty() {
            end = next_line_end + 1;
            continue;
        }
        if line_indent_len <= key_indent_len { break; }
        end = next_line_end + 1;
    }
    Some(Match { start: line_end + 1, end })
}

fn slot_toml(source: &str, slot: &str) -> Option<Match> {
    if slot.starts_with('[') && slot.ends_with(']') {
        let re = Regex::new(&format!(r"(?m)^{}\s*$", regex::escape(slot))).ok()?;
        let m = re.find(source)?;
        let body_start = source[m.end()..].find('\n').map(|p| m.end() + p + 1).unwrap_or(source.len());
        let next_section = Regex::new(r"(?m)^\[").ok()?;
        let after_body = next_section.find(&source[body_start..]).map(|n| body_start + n.start()).unwrap_or(source.len());
        return Some(Match { start: body_start, end: after_body });
    }
    let re = Regex::new(&format!(r"(?m)^([ \t]*){}\s*=\s*", regex::escape(slot))).ok()?;
    let m = re.find(source)?;
    let value_start = m.end();
    let line_end = source[value_start..].find('\n').map(|p| value_start + p).unwrap_or(source.len());
    Some(Match { start: value_start, end: line_end })
}

fn slot_env(source: &str, slot: &str) -> Option<Match> {
    let re = Regex::new(&format!(r"(?m)^(?:export\s+)?{}\s*=", regex::escape(slot))).ok()?;
    let m = re.find(source)?;
    let value_start = m.end();
    let line_end = source[value_start..].find('\n').map(|p| value_start + p).unwrap_or(source.len());
    Some(Match { start: value_start, end: line_end })
}

fn slot_markdown(source: &str, slot: &str) -> Option<Match> {
    let re = Regex::new(&format!(r"(?m)^{}\s*$", regex::escape(slot))).ok()?;
    let m = re.find(source)?;
    let line_end = source[m.end()..].find('\n').map(|p| m.end() + p + 1).unwrap_or(source.len());
    Some(Match { start: m.start(), end: line_end })
}

pub(crate) fn validate_structure(path: &Path, before: &str, after: &str) -> Result<(), String> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "json" | "jsonc" => validate_json(before, after),
        _ => Ok(()),
    }
}

fn validate_json(before: &str, after: &str) -> Result<(), String> {
    if serde_json::from_str::<serde_json::Value>(before).is_err() { return Ok(()); }
    match serde_json::from_str::<serde_json::Value>(after) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("result is invalid JSON: {e}")),
    }
}

pub(crate) fn window(source: &str, edit_start: usize, edit_end: usize, ctx_lines: usize) -> String {
    let mut ctx_start = edit_start;
    for _ in 0..ctx_lines {
        if ctx_start == 0 { break; }
        ctx_start = source[..ctx_start - 1].rfind('\n').map(|p| p + 1).unwrap_or(0);
    }
    let mut ctx_end = edit_end;
    for _ in 0..ctx_lines {
        match source[ctx_end..].find('\n') {
            Some(p) => ctx_end += p + 1,
            None => { ctx_end = source.len(); break; }
        }
    }
    source.get(ctx_start..ctx_end).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::splice;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    fn tmp(ext: &str) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("runuz-text-{}-{}.{}", std::process::id(), n, ext))
    }

    fn r(src: &str, ext: &str, scope: TextScope, text: &str) -> (usize, usize) {
        let p = tmp(ext);
        fs::write(&p, src).unwrap();
        let m = resolve(src, &p, scope, text).expect("resolve failed");
        let _ = fs::remove_file(&p);
        m
    }

    #[test]
    fn token_resolves_first_word() {
        let s = "host = localhost\nport = 3000\n";
        let m = r(s, "txt", TextScope::Token, "localhost");
        assert_eq!(&s[m.0..m.1], "localhost");
    }

    #[test]
    fn statement_expands_to_line() {
        let s = "first line\nsecond line\n";
        let m = r(s, "txt", TextScope::Statement, "second");
        assert!(s[m.0..m.1].contains("second line"));
        assert!(s[m.0..m.1].ends_with('\n'));
    }

    #[test]
    fn block_is_blank_line_paragraph() {
        let s = "first para\nmore\n\ntarget block\ncontent\n\nthird block\n";
        let m = r(s, "txt", TextScope::Block, "target block");
        let cut = splice(s, m.0, m.1, "");
        assert!(!cut.contains("target block"));
        assert!(!cut.contains("content"));
        assert!(cut.contains("first para"));
        assert!(cut.contains("third block"));
    }

    #[test]
    fn env_slot_resolves_value() {
        let s = "DATABASE_URL=postgres://old/db\nAPI_KEY=secret\n";
        let m = r(s, "env", TextScope::Slot, "DATABASE_URL");
        assert_eq!(&s[m.0..m.1], "postgres://old/db");
        assert_eq!(splice(s, m.0, m.1, "postgres://new/db"),
                   "DATABASE_URL=postgres://new/db\nAPI_KEY=secret\n");
    }

    #[test]
    fn markdown_slot_resolves_heading() {
        let s = "# Title\n\n## Setup\n\nold instructions\n";
        let m = r(s, "md", TextScope::Slot, "## Setup");
        let cut = splice(s, m.0, m.1, "## Installation\n");
        assert!(cut.contains("## Installation"));
        assert!(!cut.contains("## Setup"));
    }

    #[test]
    fn json_slot_swaps_value_and_validates() {
        let p = tmp("json");
        let s = r#"{"name": "old", "port": 3000}"#;
        fs::write(&p, s).unwrap();
        let m = resolve(s, &p, TextScope::Slot, "name").unwrap();
        let updated = splice(s, m.0, m.1, "\"new-name\"");
        assert!(validate_structure(&p, s, &updated).is_ok());
        let parsed: serde_json::Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(parsed["name"], "new-name");
        assert_eq!(parsed["port"], 3000);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn json_slot_rejects_invalid_result() {
        let p = tmp("json");
        let s = r#"{"port": 3000}"#;
        fs::write(&p, s).unwrap();
        let m = resolve(s, &p, TextScope::Slot, "port").unwrap();
        let updated = splice(s, m.0, m.1, "not_a_value");
        assert!(validate_structure(&p, s, &updated).is_err());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn toml_section_resolves_body() {
        let s = "[server]\nport = 3000\n\n[database]\nhost = \"localhost\"\n";
        let m = r(s, "toml", TextScope::Slot, "[server]");
        let cut = splice(s, m.0, m.1, "host = \"0.0.0.0\"\nport = 9090\n");
        assert!(cut.contains("port = 9090"));
        assert!(cut.contains("[database]"));
    }

    #[test]
    fn toml_key_resolves_value() {
        let s = "[server]\nport = 3000\n";
        let m = r(s, "toml", TextScope::Slot, "port");
        assert_eq!(&s[m.0..m.1], "3000");
        assert_eq!(splice(s, m.0, m.1, "9090"), "[server]\nport = 9090\n");
    }
}