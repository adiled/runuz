use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::ast::{LangSpec, Symbol};

fn call_query(lang: LangSpec) -> &'static str {
    match lang {
        LangSpec::Rust       => RUST_CALLS,
        LangSpec::Python     => PYTHON_CALLS,
        LangSpec::Go         => GO_CALLS,
        LangSpec::JavaScript => JS_CALLS,
        LangSpec::TypeScript => JS_CALLS,
        LangSpec::Tsx        => JS_CALLS,
    }
}

const RUST_CALLS: &str = r#"
(call_expression
  function: (identifier) @call.name)

(call_expression
  function: (scoped_identifier
    name: (identifier) @call.name))

(call_expression
  function: (field_expression
    field: (field_identifier) @call.name))
"#;

const PYTHON_CALLS: &str = r#"
(call
  function: (identifier) @call.name)

(call
  function: (attribute
    attribute: (identifier) @call.name))
"#;

const GO_CALLS: &str = r#"
(call_expression
  function: (identifier) @call.name)

(call_expression
  function: (selector_expression
    field: (field_identifier) @call.name))
"#;

const JS_CALLS: &str = r#"
(call_expression
  function: (identifier) @call.name)

(call_expression
  function: (member_expression
    property: (property_identifier) @call.name))
"#;

const NOISE: &[&str] = &[
    "into", "from", "new", "default", "Some", "None", "Ok", "Err",
    "unwrap", "expect", "unwrap_or", "unwrap_or_else", "ok_or_else", "map", "map_or",
    "and_then", "or_else", "filter", "filter_map", "iter", "iter_mut",
    "into_iter", "collect", "to_string", "to_vec", "to_owned", "clone",
    "as_ref", "as_mut", "as_str", "as_slice", "as_bytes", "len",
    "is_empty", "get", "get_mut", "first", "last", "push", "push_str",
    "pop", "insert", "remove", "contains", "starts_with", "ends_with",
    "trim", "trim_start", "trim_end", "split", "splitn", "join",
    "format", "println", "print", "eprintln", "eprint", "dbg", "matches",
    "is_some", "is_none", "is_ok", "is_err", "eq", "ne", "cmp", "next",
    "take", "skip", "count", "enumerate", "sorted", "sort", "sort_by",
    "sort_by_key", "min", "max", "sum", "abs", "line", "lines", "find",
    "position", "range", "str", "int", "float", "bool", "list", "dict",
    "set", "tuple", "super", "open", "isinstance", "hasattr", "getattr",
    "setattr", "append", "cap", "copy", "delete", "len", "make", "panic",
    "recover", "string", "byte", "rune", "float64",
];

pub(crate) fn call_sites(source: &str, lang: LangSpec) -> Vec<(usize, String)> {
    let mut parser = Parser::new();
    if parser.set_language(&lang.tree_sitter_language()).is_err() {
        return vec![];
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };
    let language = lang.tree_sitter_language();
    let q = match Query::new(&language, call_query(lang)) {
        Ok(q) => q,
        Err(_) => return vec![],
    };
    let capture_names: Vec<&str> = (0..q.capture_names().len())
        .map(|i| q.capture_names()[i])
        .collect();

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&q, tree.root_node(), source.as_bytes());
    let mut out: Vec<(usize, String)> = Vec::new();
    while let Some(m) = matches.next() {
        for cap in m.captures {
            if capture_names[cap.index as usize] == "call.name" {
                if let Ok(text) = cap.node.utf8_text(source.as_bytes()) {
                    if !NOISE.contains(&text) {
                        out.push((cap.node.start_byte(), text.to_string()));
                    }
                }
            }
        }
    }
    out
}

pub(crate) fn callees_per_symbol(
    symbols: &[Symbol],
    source: &str,
    lang: LangSpec,
) -> Vec<Vec<String>> {
    let sites = call_sites(source, lang);
    let mut out: Vec<Vec<String>> = vec![Vec::new(); symbols.len()];
    for (offset, name) in &sites {
        let mut best: Option<usize> = None;
        for (i, s) in symbols.iter().enumerate() {
            if s.start_byte <= *offset && *offset < s.end_byte {
                best = match best {
                    Some(b)
                        if (symbols[b].end_byte - symbols[b].start_byte)
                            <= (s.end_byte - s.start_byte) =>
                    {
                        Some(b)
                    }
                    _ => Some(i),
                };
            }
        }
        if let Some(i) = best {
            if !out[i].contains(name) {
                out[i].push(name.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;

    #[test]
    fn rust_direct_calls() {
        let src = "fn a() { b(); c(); }\nfn b() {}\nfn c() {}\n";
        let sites = call_sites(src, LangSpec::Rust);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"b"), "b missing: {names:?}");
        assert!(names.contains(&"c"), "c missing: {names:?}");
    }

    #[test]
    fn rust_method_and_scoped_calls() {
        let src = "fn a() { self.f(); std::mem::swap(&mut x, &mut y); Foo::new(); }\n";
        let sites = call_sites(src, LangSpec::Rust);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"f"), "method f missing: {names:?}");
        assert!(names.contains(&"swap"), "scoped swap missing: {names:?}");
        assert!(!names.contains(&"new"), "constructor glue leaked: {names:?}");
    }

    #[test]
    fn rust_macros_excluded() {
        let src = "fn a() { println!(\"hi\"); assert_eq!(1, 1); real(); }\n";
        let sites = call_sites(src, LangSpec::Rust);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"real"), "real missing: {names:?}");
        assert!(!names.contains(&"println"), "macro leaked: {names:?}");
        assert!(!names.contains(&"assert_eq"), "macro leaked: {names:?}");
    }

    #[test]
    fn std_glue_noise_filtered() {
        let src = "fn a() { let x: Vec<i32> = vec![1,2,3].into_iter().filter(|n| n > 0).map(|n| n + 1).collect();\n    engine.pump(); }\n";
        let sites = call_sites(src, LangSpec::Rust);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"pump"), "pump missing: {names:?}");
        assert!(!names.contains(&"into_iter"), "glue leaked: {names:?}");
        assert!(!names.contains(&"filter"), "glue leaked: {names:?}");
        assert!(!names.contains(&"map"), "glue leaked: {names:?}");
        assert!(!names.contains(&"collect"), "glue leaked: {names:?}");
    }

    #[test]
    fn strings_and_comments_are_not_calls() {
        let src = "fn a() { let s = \"ghost()\"; // phantom()\n real(); }\n";
        let sites = call_sites(src, LangSpec::Rust);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"real"), "real missing: {names:?}");
        assert!(!names.contains(&"ghost"), "string leaked: {names:?}");
        assert!(!names.contains(&"phantom"), "comment leaked: {names:?}");
    }

    #[test]
    fn attribution_smallest_enclosing_and_dedupe() {
        let src = "fn outer() { helper(); inner(); }\nfn inner() { helper(); }\nfn helper() {}\n";
        let syms = ast::file_symbols(src, LangSpec::Rust);
        let callees = callees_per_symbol(&syms, src, LangSpec::Rust);
        let by_name = |n: &str| -> Vec<String> {
            syms.iter().zip(&callees)
                .find(|(s, _)| s.name == n)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("symbol {n} missing"))
        };
        assert_eq!(by_name("outer"), vec!["helper", "inner"]);
        assert_eq!(by_name("inner"), vec!["helper"]);
        assert!(by_name("helper").is_empty());
    }

    #[test]
    fn python_calls() {
        let src = "import os\ndef a():\n    os.path.abspath('x')\n    helper()\n\ndef helper():\n    pass\n";
        let sites = call_sites(src, LangSpec::Python);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"abspath"), "abspath missing: {names:?}");
        assert!(names.contains(&"helper"), "helper missing: {names:?}");
    }

    #[test]
    fn go_calls() {
        let src = "package main\nimport \"fmt\"\nfunc main() {\n\tfmt.Println(\"hi\")\n\thelper()\n}\nfunc helper() {}\n";
        let sites = call_sites(src, LangSpec::Go);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"Println"), "Println missing: {names:?}");
        assert!(names.contains(&"helper"), "helper missing: {names:?}");
    }

    #[test]
    fn js_calls() {
        let src = "function a() { helper(); obj.method(); }\nfunction helper() {}\n";
        let sites = call_sites(src, LangSpec::JavaScript);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"helper"), "helper missing: {names:?}");
        assert!(names.contains(&"method"), "method missing: {names:?}");
    }

    #[test]
    fn typescript_calls() {
        let src = "function a(): void { helper(); }\nfunction helper(): void {}\n";
        let sites = call_sites(src, LangSpec::TypeScript);
        let names: Vec<&str> = sites.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"helper"), "helper missing: {names:?}");
    }
}
