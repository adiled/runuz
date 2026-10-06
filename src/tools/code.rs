use std::path::{Path, PathBuf};

use crate::io::{atomic_write, ok, splice};
use crate::tools::text as text_scope;
use crate::{ToolDef, ToolResult};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ast::{self, LangSpec, Symbol, SymbolKind};

#[derive(Deserialize)]
struct Args {
    file_path: String,
    #[serde(default = "default_op")]
    operation: String,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    symbols: Option<String>,
    #[serde(default)]
    new_source: Option<String>,
    #[serde(default)]
    pub new_name: Option<String>,
}

fn default_op() -> String { "replace".into() }

#[allow(dead_code)]
pub(crate) fn def() -> ToolDef {
    ToolDef {
        name: "runuz_code".into(),
        description: "Author files: create | replace | insert_before | insert_after | delete | rename. Every unit of the file is addressable by --symbol. A name (fn/Class.method/imports, dot-nested sub-walks body/when/otherwise/loop/try/return/call, #N disambiguation) or a shape rung ('token <t>' | 'slot <s>' | 'statement <s>' | 'block <s>'). Slot is format-aware (JSON key-path, YAML/TOML/env key, markdown heading) with exact-substring fallback. A plain name on a non-code file finds its exact text. No symbol means a whole-file replace. Every code write is re-parsed; a syntax-error result aborts the write. Text writes validate structure (JSON stays JSON).".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path":  { "type": "string", "description": "Absolute path to the file." },
                "operation":  { "type": "string", "description": "One of: create, replace, insert_before, insert_after, delete, rename. Default: replace." },
                "symbol":     { "type": "string", "description": "Address of the unit to edit: a symbol NAME (code files; dot-separated 'Class.method', sub-walks 'alpha.body', 'alpha.when.otherwise#2', synthetic 'imports') or a shape rung 'token <t>' / 'slot <s>' / 'statement <s>' / 'block <s>' (rungs resolve by shape on any file; slot is format-aware and falls back to exact substring). On non-code files, a bare name resolves as its exact text." },
                "symbols":    { "type": "string", "description": "Comma-separated list of symbol names for one atomic replace/delete, contiguous or not. Any missing name aborts with no partial edit." },
                "new_source": { "type": "string", "description": "The new source. Required for create/replace/insert. Empty string deletes the addressed range." },
                "new_name":   { "type": "string", "description": "Rename target (operation rename)." },
            },
            "required": ["file_path"],
        }),
    }
}

pub async fn run(args: Value) -> ToolResult {
    let args: Args = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolResult::error(format!("invalid args: {e}")),
    };
    let path = PathBuf::from(&args.file_path);

    if args.operation == "create" {
        return op_create(&path, ast::detect_language(&path), args.new_source.as_deref());
    }

    let lang = ast::detect_language(&path);
    let address = args.symbol.as_deref().map(text_scope::classify);

    match address {
        Some(text_scope::Address::Shape(scope, text)) => {
            shape_op(&path, lang, &args.operation, scope, text, args.new_source.as_deref())
        }
        Some(text_scope::Address::Name(name)) => {
            name_op(&path, lang, &args.operation, Some(name), args.symbols.as_deref(),
                    args.new_source.as_deref(), args.new_name.as_deref())
        }
        None => name_op(&path, lang, &args.operation, None, args.symbols.as_deref(),
                        args.new_source.as_deref(), args.new_name.as_deref()),
    }
}

fn shape_op(
    path: &Path, lang: Option<LangSpec>, operation: &str, scope: text_scope::TextScope, text: &str,
    new_source: Option<&str>,
) -> ToolResult {
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    let (start, end) = match text_scope::resolve(&original, path, scope, text) {
        Some(r) => r,
        None => return ToolResult::error(format!(
            "{} '{text}' not found in {}. Read the file first to see its content.",
            scope.tag(), path.display()
        )),
    };
    text_op(path, lang, operation, scope.tag(), text, start, end, new_source)
}

fn name_op(
    path: &Path, lang: Option<LangSpec>, operation: &str, name: Option<&str>, symbols: Option<&str>,
    new_source: Option<&str>, new_name: Option<&str>,
) -> ToolResult {
    if operation == "rename" {
        return match name {
            Some(n) => op_rename(path, lang, Some(n), new_name),
            None => ToolResult::error("rename needs --symbol (the name to rename)"),
        };
    }
    if let Some(list) = symbols {
        let l = match lang {
            Some(l) => l,
            None => return ToolResult::error("--symbols (multi) needs a code file. Use a single --symbol on text."),
        };
        return match operation {
            "replace" => op_replace(path, l, name, Some(list), new_source),
            "delete" => op_delete(path, l, name, Some(list)),
            other => ToolResult::error(format!(
                "operation '{other}' does not take --symbols (only replace and delete do)"
            )),
        };
    }
    let name = match name {
        Some(n) => n,
        None => {
            if operation == "replace" {
                return match lang {
                    Some(l) => op_replace(path, l, None, None, new_source),
                    None => whole_file_text_replace(path, new_source),
                };
            }
            return ToolResult::error(format!(
                "{} needs an address: --symbol NAME or --symbol 'rung <text>'.",
                operation
            ));
        }
    };
    match lang {
        Some(l) => match operation {
            "replace" => op_replace(path, l, Some(name), None, new_source),
            "insert_before" => op_insert(path, l, Some(name), new_source, Anchor::Before),
            "insert_after"  => op_insert(path, l, Some(name), new_source, Anchor::After),
            "delete" => op_delete(path, l, Some(name), None),
            "rename" => unreachable!(),
            other => ToolResult::error(format!(
                "unknown operation '{other}' - pick one of: create, replace, insert_before, insert_after, delete, rename"
            )),
        },
        None => {
            let original = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => return ToolResult::error(format!("read failed: {e}")),
            };
            let idx = match original.find(name) {
                Some(i) => i,
                None => return ToolResult::error(format!(
                    "'{name}' not found in {}. Read the file first to see its content.",
                    path.display()
                )),
            };
            text_op(path, None, operation, "symbol", name, idx, idx + name.len(), new_source)
        }
    }
}

fn text_op(
    path: &Path, lang: Option<LangSpec>, operation: &str, tag: &str, label: &str,
    start: usize, end: usize, new_source: Option<&str>,
) -> ToolResult {
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    let (updated, action) = match operation {
        "replace" => {
            let src = match new_source {
                Some(s) => s,
                None => return ToolResult::error("replace needs new_source"),
            };
            (splice(&original, start, end, src), if src.is_empty() { "Deleted" } else { "Replaced" })
        }
        "insert_before" => {
            let src = match new_source {
                Some(s) => s,
                None => return ToolResult::error("insert needs new_source"),
            };
            (splice(&original, start, start, src), "Inserted")
        }
        "insert_after" => {
            let src = match new_source {
                Some(s) => s,
                None => return ToolResult::error("insert needs new_source"),
            };
            (splice(&original, end, end, src), "Inserted")
        }
        "delete" => (splice(&original, start, end, ""), "Deleted"),
        other => return ToolResult::error(format!(
            "operation '{other}' cannot be addressed. Use create/rename for files, or replace/insert_before/insert_after/delete for units"
        )),
    };
    match lang {
        Some(l) => {
            if let Err(msg) = ast::validate_edited(&original, &updated, l, &[(start, end)]) {
                return ToolResult::error(format!(
                    "rejected - result has {msg}; original left untouched"
                ));
            }
        }
        None => {
            if let Err(msg) = text_scope::validate_structure(path, &original, &updated) {
                return ToolResult::error(format!(
                    "edit would corrupt {} - {msg}. File NOT modified; fix your replacement and try again.",
                    path.display()
                ));
            }
        }
    }
    if let Err(e) = atomic_write(path, &updated) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    ok(
        format!("{action} {tag} '{label}' in {} ({} bytes)", path.display(), updated.len()),
        path,
    )
}

fn op_create(path: &Path, lang: Option<LangSpec>, new_source: Option<&str>) -> ToolResult {
    let src = match new_source {
        Some(s) => s,
        None => return ToolResult::error("create needs new_source"),
    };
    if path.exists() {
        return ToolResult::error(format!(
            "{} already exists. Use operation replace to modify, or insert_before / insert_after to add adjacent to an existing symbol.",
            path.display()
        ));
    }
    if let Some(l) = lang {
        if let Err(msg) = ast::validate_syntax(src, l) {
            return ToolResult::error(format!("rejected - new_source has {msg}"));
        }
    }
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::error(format!("mkdir -p failed: {e}"));
            }
        }
    }
    if let Err(e) = atomic_write(path, src) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    ok(format!("Created {} ({} bytes)", path.display(), src.len()), path)
}

fn whole_file_text_replace(path: &Path, new_source: Option<&str>) -> ToolResult {
    let new = match new_source {
        Some(s) => s.to_string(),
        None => return ToolResult::error("replace needs new_source"),
    };
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    if let Err(msg) = text_scope::validate_structure(path, &original, &new) {
        return ToolResult::error(format!(
            "edit would corrupt {} - {msg}. File NOT modified; fix your replacement and try again.",
            path.display()
        ));
    }
    if let Err(e) = atomic_write(path, &new) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    ok(format!("Replaced whole file in {} ({} bytes)", path.display(), new.len()), path)
}

fn op_replace(
    path: &Path, lang: LangSpec, symbol: Option<&str>, symbols: Option<&str>, new_source: Option<&str>,
) -> ToolResult {
    let new = match new_source {
        Some(s) => s.to_string(),
        None => return ToolResult::error("replace needs new_source"),
    };
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };

    let (updated, edit_ranges) = if let Some(list) = symbols {
        match replace_multi(&original, lang, &list, &new) {
            Err(e) => return ToolResult::error(format!("{e}")),
            Ok((u, ranges)) => (u, ranges),
        }
    } else {
        match symbol {
            None => (new.clone(), vec![(0, original.len())]),
            Some(sym_name) => match locate_symbol(&original, lang, sym_name) {
                Err(e) => return ToolResult::error(format!("{e}")),
                Ok(sym) => (splice(&original, sym.start_byte, sym.end_byte, &new),
                             vec![(sym.start_byte, sym.end_byte)]),
            },
        }
    };

    if let Err(msg) = ast::validate_edited(&original, &updated, lang, &edit_ranges) {
        return ToolResult::error(format!("rejected - result has {msg}; original left untouched"));
    }
    if let Err(e) = atomic_write(path, &updated) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    let scope = symbol.map(|s| format!("symbol {s}")).unwrap_or_else(|| "whole file".into());
    ok(format!("Replaced {} in {} ({} bytes)", scope, path.display(), updated.len()), path)
}

#[derive(Clone, Copy)]
enum Anchor { Before, After }

fn op_insert(
    path: &Path, lang: LangSpec, symbol: Option<&str>, new_source: Option<&str>, anchor: Anchor,
) -> ToolResult {
    let new = match new_source {
        Some(s) => s,
        None => return ToolResult::error("insert needs new_source"),
    };
    let sym_name = match symbol {
        Some(s) => s,
        None => return ToolResult::error("insert needs symbol (the anchor)"),
    };
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    let sym = match locate_symbol(&original, lang, sym_name) {
        Err(e) => return ToolResult::error(format!("{e}")),
        Ok(s) => s,
    };
    let insert_at = match anchor {
        Anchor::Before => line_start_if_indented_alone(&original, sym.start_byte),
        Anchor::After  => sym.end_byte,
    };
    let separator = if new.ends_with('\n') { "\n" } else { "\n\n" };
    let payload = match anchor {
        Anchor::Before => format!("{new}{separator}"),
        Anchor::After  => format!("{separator}{new}"),
    };
    let updated = splice(&original, insert_at, insert_at, &payload);
    let edit_ranges = vec![(insert_at, insert_at)];

    if let Err(msg) = ast::validate_edited(&original, &updated, lang, &edit_ranges) {
        return ToolResult::error(format!("rejected - result has {msg}; original left untouched"));
    }
    if let Err(e) = atomic_write(path, &updated) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    let where_str = match anchor { Anchor::Before => "before", Anchor::After => "after" };
    ok(
        format!("Inserted {} bytes {where_str} '{sym_name}' in {}", new.len(), path.display()),
        path,
    )
}

fn op_delete(path: &Path, lang: LangSpec, symbol: Option<&str>, symbols: Option<&str>) -> ToolResult {
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    let (updated, edit_ranges) = if let Some(list) = symbols {
        match delete_multi_span(&original, lang, &list) {
            Err(e) => return ToolResult::error(format!("{e}")),
            Ok((u, ranges)) => (u, ranges),
        }
    } else {
        let sym_name = match symbol {
            Some(s) => s,
            None => return ToolResult::error("delete needs symbol (or --symbols A,B)"),
        };
        let sym = match locate_symbol(&original, lang, sym_name) {
            Err(e) => return ToolResult::error(format!("{e}")),
            Ok(s) => s,
        };
        let start = line_start_if_indented_alone(&original, sym.start_byte);
        let end = maybe_eat_blank_line(&original, extend_field_terminator(&original, sym.end_byte, sym.kind));
        (splice(&original, start, end, ""), vec![(start, end)])
    };

    if let Err(msg) = ast::validate_edited(&original, &updated, lang, &edit_ranges) {
        return ToolResult::error(format!("rejected - result has {msg}; original left untouched"));
    }
    if let Err(e) = atomic_write(path, &updated) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    let what = match (&symbols, &symbol) {
        (Some(list), _) => format!("symbols '{list}'"),
        (None, Some(n)) => format!("symbol '{n}'"),
        (None, None) => "symbol".into(),
    };
    ok(format!("Deleted {what} from {}", path.display()), path)
}

fn op_rename(
    path: &Path, lang: Option<LangSpec>, symbol: Option<&str>, new_name: Option<&str>,
) -> ToolResult {
    let sym_name = match symbol {
        Some(s) => s,
        None => return ToolResult::error("rename needs symbol (the name to rename)"),
    };
    let replacement = match new_name {
        Some(s) => s,
        None => return ToolResult::error("rename needs new_name"),
    };
    let original = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("read failed: {e}")),
    };
    let l = match lang {
        Some(l) => l,
        None => {
            let re = match regex::Regex::new(&format!(r"\b{}\b", regex::escape(sym_name))) {
                Ok(r) => r,
                Err(e) => return ToolResult::error(format!("bad rename target: {e}")),
            };
            if !re.is_match(&original) {
                return ToolResult::error(format!("'{sym_name}' not found in {}", path.display()));
            }
            let updated = re.replace_all(&original, replacement).into_owned();
            if let Err(e) = atomic_write(path, &updated) {
                return ToolResult::error(format!("write failed: {e}"));
            }
            return ok(
                format!("Renamed '{sym_name}' to '{replacement}' in {} (text, word-boundary)", path.display()),
                path,
            );
        }
    };
    let tree = match ast::parse(&original, l) {
        Some(t) => t,
        None => return ToolResult::error("parser unavailable".to_string()),
    };
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut cursor = tree.walk();
    let mut done = false;
    while !done {
        let node = cursor.node();
        if node.kind() == "identifier" || node.kind() == "type_identifier" || node.kind() == "field_identifier" {
            if let Ok(text) = node.utf8_text(original.as_bytes()) {
                if text == sym_name {
                    ranges.push((node.start_byte(), node.end_byte()));
                }
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                done = true;
                break;
            }
        }
    }
    if ranges.is_empty() {
        return ToolResult::error(format!("symbol '{sym_name}' not found in {}", path.display()));
    }
    let mut updated = original.clone();
    for (s, e) in ranges.iter().rev() {
        updated.replace_range(*s..*e, replacement);
    }
    if let Err(msg) = ast::validate_edited(&original, &updated, l, &ranges) {
        return ToolResult::error(format!("rejected - result has {msg}; original left untouched"));
    }
    if let Err(e) = atomic_write(path, &updated) {
        return ToolResult::error(format!("write failed: {e}"));
    }
    ok(format!("Renamed '{sym_name}' to '{replacement}' in {} ({} occurrences)", path.display(), ranges.len()), path)
}

fn locate_symbol(source: &str, lang: LangSpec, name: &str) -> Result<Symbol, String> {
    if name == "imports" {
        return synthesize_imports(source, lang)
            .ok_or_else(|| format!("no import block found in source"));
    }
    let (start_byte, end_byte, start_row, end_row) = ast::resolve_path(source, lang, name)?;
    Ok(Symbol {
        name: name.to_string(),
        kind: SymbolKind::Other,
        start_byte, end_byte, start_row, end_row,
    })
}

/// Resolve several named symbols against the SAME `source` (so no
/// byte drift), and return them ordered by start_byte. Any missing
/// name is an error (no partial resolution).
fn resolve_multi(source: &str, lang: LangSpec, list: &str) -> Result<Vec<Symbol>, String> {
    let mut out = Vec::new();
    for name in list.split(',') {
        let name = name.trim();
        if name.is_empty() { continue; }
        let sym = match locate_symbol(source, lang, name) {
            Err(e) => return Err(format!("{e}. No partial edit applied")),
            Ok(s) => s,
        };
        out.push(sym);
    }
    if out.is_empty() { return Err("symbols list is empty; nothing to edit".into()); }
    out.sort_by_key(|s| s.start_byte);
    Ok(out)
}

/// `replace` across several symbols — contiguous or not. Each symbol's
/// own byte range is replaced with `new_source` (multi-site splice),
/// resolved against the same source, applied in one pass, one write.
/// Overlapping/adjacent ranges coalesce into one combined span, so a
/// contiguous run still yields a single replacement.
fn replace_multi(source: &str, lang: LangSpec, list: &str, new_source: &str) -> Result<(String, Vec<(usize, usize)>), String> {
    let syms = resolve_multi(source, lang, list)?;
    let mut ranges: Vec<(usize, usize)> =
        syms.iter().map(|s| (s.start_byte, s.end_byte)).collect();
    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 { last.1 = last.1.max(e); continue; }
        }
        merged.push((s, e));
    }
    let mut out = source.to_string();
    for &(s, e) in merged.iter().rev() { out = splice(&out, s, e, new_source); }
    Ok((out, merged))
}

/// `delete` across several symbols — contiguous or not. Each symbol's
/// own line-range is dropped (multi-site splice), resolved against the
/// same source, one pass, one write. Overlapping/adjacent ranges
/// coalesce into one combined span. A trailing newline is only eaten
/// when the line it leaves behind is surely blank.
fn delete_multi_span(source: &str, lang: LangSpec, list: &str) -> Result<(String, Vec<(usize, usize)>), String> {
    let syms = resolve_multi(source, lang, list)?;
    let mut ranges: Vec<(usize, usize)> = syms.iter().map(|s| {
        let start = line_start_if_indented_alone(source, s.start_byte);
        let end = maybe_eat_blank_line(source, extend_field_terminator(source, s.end_byte, s.kind));
        (start, end)
    }).collect();
    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 { last.1 = last.1.max(e); continue; }
        }
        merged.push((s, e));
    }
    let mut out = source.to_string();
    for &(s, e) in merged.iter().rev() { out = splice(&out, s, e, ""); }
    Ok((out, merged))
}

/// Synthetic `imports` symbol: the leading contiguous run of
/// import/use/require/include declarations at top level. Walks
/// the tree-sitter tree's first-level children and groups
/// neighbour import nodes into one byte range.
/// crate-visible alias so `read` can resolve the synthetic
/// `imports` symbol exactly like the write ops do.
pub(crate) fn imports_symbol(source: &str, lang: LangSpec) -> Option<(usize, usize, usize, usize)> {
    synthesize_imports(source, lang).map(|s| (s.start_byte, s.end_byte, s.start_row, s.end_row))
}

fn synthesize_imports(source: &str, lang: LangSpec) -> Option<Symbol> {
    let tree = ast::parse(source, lang)?;
    let root = tree.root_node();
    let mut cur = root.walk();
    let mut first_byte: Option<usize> = None;
    let mut last_byte: usize = 0;
    let mut last_row: usize = 1;
    for child in root.children(&mut cur) {
        let kind = child.kind();
        let is_import = matches!(
            kind,
            "use_declaration"          // rust
            | "extern_crate_declaration"
            | "import_statement"       // py, js, ts
            | "import_from_statement"  // py
            | "import_declaration"     // go, js, ts
            | "import_spec"
            | "require_statement"
            | "preproc_include"        // c/cpp
        );
        if is_import {
            if first_byte.is_none() { first_byte = Some(child.start_byte()); }
            last_byte = child.end_byte();
            last_row = child.end_position().row + 1;
        } else if first_byte.is_some() {
            break;
        }
    }
    let start_byte = first_byte?;
    Some(Symbol {
        name: "imports".into(),
        kind: SymbolKind::Imports,
        start_byte,
        end_byte: last_byte,
        start_row: source[..start_byte].lines().count().max(1),
        end_row: last_row,
    })
}

fn line_start_if_indented_alone(source: &str, index: usize) -> usize {
    let mut i = index;
    while i > 0 && &source[i - 1..i] != "\n" { i -= 1; }
    for k in i..index {
        let ch = &source[k..k + 1];
        if ch != " " && ch != "\t" { return index; }
    }
    i
}

fn extend_field_terminator(source: &str, end: usize, _kind: SymbolKind) -> usize {
    let rest = &source[end..];
    let trimmed = rest.trim_start_matches([' ', '\t']);
    if trimmed.starts_with(',') || trimmed.starts_with(';') {
        let after = trimmed[1..].trim_start_matches([' ', '\t']);
        if after.is_empty() || after.starts_with('\n') {
            return end + 1;
        }
    }
    end
}

fn maybe_eat_blank_line(source: &str, end: usize) -> usize {
    let mut j = end;
    while j < source.len() && &source[j..j + 1] != "\n" {
        let ch = &source[j..j + 1];
        if ch != " " && ch != "\t" { return end; }
        j += 1;
    }
    if source.get(end..end + 1) != Some("\n") {
        return end;
    }
    let mut i = end + 1;
    loop {
        let mut k = i;
        while k < source.len() && &source[k..k + 1] != "\n" {
            let ch = &source[k..k + 1];
            if ch != " " && ch != "\t" { return i; }
            k += 1;
        }
        if k >= source.len() { return i; }
        i = k + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicUsize;

    static SEQ: AtomicUsize = AtomicUsize::new(0);
    fn tmp(suffix: &str) -> PathBuf {
        crate::io::tmp(&SEQ, "code", suffix)
    }

    #[tokio::test]
    async fn create_writes_and_validates() {
        let p = tmp("rs");
        let _ = fs::remove_file(&p);
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "create",
            "new_source": "fn main() {}\n",
        })).await;
        assert!(!res.is_error, "create failed: {}", res.output);
        assert_eq!(fs::read_to_string(&p).unwrap(), "fn main() {}\n");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn create_rejects_bad_syntax() {
        let p = tmp("rs");
        let _ = fs::remove_file(&p);
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "create",
            "new_source": "fn x( { ;;",
        })).await;
        assert!(res.is_error, "should have rejected syntax error");
        assert!(!p.exists(), "file should not have been written");
    }

    #[tokio::test]
    async fn replace_symbol_scoped() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "alpha",
            "new_source": "fn alpha() -> u32 { 99 }",
        })).await;
        assert!(!res.is_error, "replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("99"), "didn't replace alpha: {updated}");
        assert!(updated.contains("fn beta"), "lost beta: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn replace_whole_file_when_no_symbol() {
        let p = tmp("rs");
        fs::write(&p, "fn old() {}").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "new_source": "fn brand_new() -> u32 { 42 }",
        })).await;
        assert!(!res.is_error, "whole-file replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("brand_new"));
        assert!(!updated.contains("old"));
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn insert_after_anchor() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "insert_after",
            "symbol": "alpha",
            "new_source": "fn beta() {}",
        })).await;
        assert!(!res.is_error, "insert_after failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("fn alpha"));
        assert!(updated.contains("fn beta"));
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_symbol() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\nfn beta() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "alpha",
        })).await;
        assert!(!res.is_error, "delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(!updated.contains("fn alpha"));
        assert!(updated.contains("fn beta"));
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_multi_symbols_contiguous_run() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\nfn beta() {}\nfn gamma() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbols": "alpha,beta",
        })).await;
        assert!(!res.is_error, "multi delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(!updated.contains("fn alpha"), "alpha left: {updated}");
        assert!(!updated.contains("fn beta"), "beta left: {updated}");
        assert!(updated.contains("fn gamma"), "lost gamma: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_multi_rejects_missing_symbol_atomically() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\nfn beta() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbols": "alpha,does_not_exist",
        })).await;
        assert!(res.is_error, "should reject missing symbol");
        assert_eq!(fs::read_to_string(&p).unwrap(), "fn alpha() {}\nfn beta() {}\n",
            "file must be untouched on partial resolution failure");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn replace_multi_symbols_contiguous_run() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\nfn gamma() -> u32 { 3 }\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbols": "alpha,beta",
            "new_source": "fn combined() -> u32 { 99 }",
        })).await;
        assert!(!res.is_error, "multi replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("fn combined"), "no combined: {updated}");
        assert!(!updated.contains("fn alpha"), "alpha left: {updated}");
        assert!(!updated.contains("fn beta"), "beta left: {updated}");
        assert!(updated.contains("fn gamma"), "lost gamma: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn rename_symbol_skips_comments_and_strings() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\n// alpha in comment\nlet s = \"alpha in string\"\nfn beta() { alpha() }\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "rename",
            "symbol": "alpha",
            "new_name": "renamed_fn",
        })).await;
        assert!(!res.is_error, "rename failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("renamed_fn"), "should rename: {updated}");
        assert!(updated.contains("// alpha in comment"), "should keep comment: {updated}");
        assert!(updated.contains("alpha in string"), "should keep string: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn replace_rejects_when_result_breaks_syntax() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "alpha",
            "new_source": "fn alpha( { ;;",
        })).await;
        assert!(res.is_error, "should have rejected broken result");
        assert!(fs::read_to_string(&p).unwrap().contains("fn alpha() {}"),
            "original should be untouched");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn plain_name_replace_on_text() {
        let p = tmp("md");
        fs::write(&p, "# heading\n\ntext about hello world\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "hello",
            "new_source": "salut",
        })).await;
        assert!(!res.is_error, "plain name on text should work: {}", res.output);
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains("salut"), "swapped: {s}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn create_works_for_non_code() {
        let p = tmp("json");
        let _ = fs::remove_file(&p);
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "create",
            "new_source": "{\"name\": \"x\"}\n",
        })).await;
        assert!(!res.is_error, "create failed: {}", res.output);
        assert_eq!(fs::read_to_string(&p).unwrap(), "{\"name\": \"x\"}\n");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn shape_replace_on_text() {
        let p = tmp("env");
        fs::write(&p, "DB_HOST=localhost\nDB_PORT=5432\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "slot DB_HOST",
            "new_source": "production-db",
        })).await;
        assert!(!res.is_error, "shape replace failed: {}", res.output);
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains("DB_HOST=production-db"), "swapped: {s}");
        assert!(s.contains("DB_PORT"), "kept: {s}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn shape_delete_leaves_rest() {
        let p = tmp("md");
        fs::write(&p, "# Old Title\n\n## Section\n\nContent here.\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "slot ## Section",
        })).await;
        assert!(!res.is_error, "shape delete failed: {}", res.output);
        let s = fs::read_to_string(&p).unwrap();
        assert!(!s.contains("## Section"), "section gone: {s}");
        assert!(s.contains("# Old Title"), "title kept: {s}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn shape_edit_on_code_is_reparsed() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "token alpha",
            "new_source": "beta",
        })).await;
        assert!(!res.is_error, "shape edit on code should work: {}", res.output);
        assert!(fs::read_to_string(&p).unwrap().contains("fn beta"),
            "should have renamed: {}", fs::read_to_string(&p).unwrap());
        let res_bad = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "statement fn beta",
            "new_source": "this is not rust ((",
        })).await;
        assert!(res_bad.is_error, "broken shape edit on code must be rejected");
        assert!(fs::read_to_string(&p).unwrap().contains("fn beta"),
            "file untouched after rejected edit");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn shape_empty_string_deletes() {
        let p = tmp("txt");
        fs::write(&p, "host = localhost\nport = 3000\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "token localhost",
            "new_source": "",
        })).await;
        assert!(!res.is_error, "empty replace failed: {}", res.output);
        let s = fs::read_to_string(&p).unwrap();
        assert!(!s.contains("localhost"), "gone: {s}");
        assert!(s.contains("port"), "kept: {s}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn replace_sub_symbol_body() {
        // Replace the body of `alpha` via the sub-symbol path
        // "alpha.body". Without that, the caller would have to know
        // alpha's exact byte range.
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {\n    let x = 1;\n}\nfn beta() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "alpha.body",
            "new_source": "{ let y = 42; }",
        })).await;
        assert!(!res.is_error, "sub-symbol replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("y = 42"), "didn't replace body: {updated}");
        assert!(updated.contains("fn alpha"), "lost alpha signature: {updated}");
        assert!(updated.contains("fn beta"), "lost beta: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn imports_symbol_replace() {
        let p = tmp("rs");
        fs::write(&p, "use std::fs;\nuse std::io;\n\nfn alpha() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "imports",
            "new_source": "use std::path::PathBuf;\nuse std::collections::HashMap;",
        })).await;
        assert!(!res.is_error, "imports replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("PathBuf"));
        assert!(updated.contains("HashMap"));
        assert!(!updated.contains("std::fs"));
        assert!(updated.contains("fn alpha"));
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_multi_non_contiguous_sites() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() {}\nfn beta() {}\nfn gamma() {}\nfn delta() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbols": "alpha,gamma",
        })).await;
        assert!(!res.is_error, "non-contiguous delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(!updated.contains("fn alpha"), "alpha left: {updated}");
        assert!(updated.contains("fn beta"), "lost beta: {updated}");
        assert!(!updated.contains("fn gamma"), "gamma left: {updated}");
        assert!(updated.contains("fn delta"), "lost delta: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn replace_multi_non_contiguous_sites() {
        let p = tmp("rs");
        fs::write(&p, "fn alpha() -> u32 { 1 }\nfn beta() -> u32 { 2 }\nfn gamma() -> u32 { 3 }\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbols": "alpha,gamma",
            "new_source": "fn x() {}",
        })).await;
        assert!(!res.is_error, "non-contiguous replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("fn x"), "no x: {updated}");
        assert!(!updated.contains("fn alpha"), "alpha left: {updated}");
        assert!(updated.contains("fn beta"), "lost beta: {updated}");
        assert!(!updated.contains("fn gamma"), "gamma left: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_does_not_eat_shared_line() {
        let p = tmp("rs");
        // alpha and beta share one line: `fn alpha() {} fn beta() {}`
        fs::write(&p, "fn alpha() {} fn beta() {}\nfn gamma() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "alpha",
        })).await;
        assert!(!res.is_error, "delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        // beta must survive on its own line — the newline after alpha's
        // end_byte is NOT blank (beta follows), so it must not be eaten.
        assert!(updated.contains("fn beta"), "beta lost: {updated}");
        assert!(updated.contains("fn gamma"), "gamma lost: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_eats_surely_blank_line() {
        let p = tmp("rs");
        // alpha alone on its line, followed by a blank line then beta.
        fs::write(&p, "fn alpha() {}\n\nfn beta() {}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "alpha",
        })).await;
        assert!(!res.is_error, "delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        // the blank line between alpha and beta should be eaten too,
        // leaving beta at the top with no dangling blank line.
        assert!(!updated.starts_with("\n"), "dangling blank line: {updated:?}");
        assert!(updated.contains("fn beta"), "beta lost: {updated}");
        let _ = fs::remove_file(&p);
    }

    // Issue #1: pre-existing parse error (Rust 2024 `safe fn`) elsewhere
    // in the file must NOT block an edit to a clean, unrelated region.
    #[tokio::test]
    async fn delete_ignores_unrelated_preexisting_error() {
        let p = tmp("rs");
        fs::write(&p, "pub const PRODUCT_NAME: &str = \"x\";\n\nunsafe extern \"C\" {\n    #[link_name = \"getuid\"]\n    safe fn getuid() -> u32;\n}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "PRODUCT_NAME",
        })).await;
        assert!(!res.is_error, "should delete PRODUCT_NAME despite safe fn: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(!updated.contains("PRODUCT_NAME"), "PRODUCT_NAME left: {updated}");
        let _ = fs::remove_file(&p);
    }

    // Issue #2: struct fields are addressable symbols — read/replace/delete.
    #[tokio::test]
    async fn replace_struct_field_symbol() {
        let p = tmp("rs");
        fs::write(&p, "struct Record {\n    ts: u64,\n    reference: Option<String>,\n}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "replace",
            "symbol": "Record.reference",
            "new_source": "reference: Option<u8>",
        })).await;
        assert!(!res.is_error, "field replace failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(updated.contains("reference: Option<u8>"), "not replaced: {updated}");
        assert!(updated.contains("ts: u64"), "lost sibling: {updated}");
        let _ = fs::remove_file(&p);
    }

    #[tokio::test]
    async fn delete_struct_field_does_not_dangle_comma() {
        let p = tmp("rs");
        fs::write(&p, "struct Record {\n    ts: u64,\n    c_us: u64,\n    reference: Option<String>,\n}\n").unwrap();
        let res = run(json!({
            "file_path": p.display().to_string(),
            "operation": "delete",
            "symbol": "Record.ts",
        })).await;
        assert!(!res.is_error, "field delete failed: {}", res.output);
        let updated = fs::read_to_string(&p).unwrap();
        assert!(!updated.contains("ts: u64"), "ts left: {updated}");
        assert!(updated.contains("c_us: u64"), "lost c_us: {updated}");
        assert!(updated.contains("reference: Option<String>"), "lost reference: {updated}");
        let _ = fs::remove_file(&p);
    }
}
