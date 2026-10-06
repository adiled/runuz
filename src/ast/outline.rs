use crate::ast::{Symbol, SymbolKind};

pub(crate) fn format_symbols_with_callees(
    symbols: &[Symbol],
    callees: Option<&[Vec<String>]>,
) -> String {
    if symbols.is_empty() {
        return "(no symbols detected)".into();
    }
    let mut out = String::new();
    let mut stack: Vec<usize> = Vec::new();
    for (i, sym) in symbols.iter().enumerate() {
        if sym.kind == SymbolKind::Other { continue; }
        while let Some(&top) = stack.last() {
            if sym.start_byte < symbols[top].end_byte {
                break;
            }
            stack.pop();
        }
        let depth = stack.len();
        let indent: String = "  ".repeat(depth);
        out.push_str(&format!(
            "{indent}{tag} {name} L{start}-L{end}",
            tag = sym.kind.tag(),
            name = sym.name,
            start = sym.start_row,
            end = sym.end_row,
        ));
        if let Some(cs) = callees.and_then(|c| c.get(i)) {
            if !cs.is_empty() {
                let shown: Vec<&str> = cs.iter().map(|s| s.as_str()).collect();
                out.push_str(&format!(" → {}", shown.join(", ")));
            }
        }
        out.push('\n');
        stack.push(i);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SymbolKind;

    #[test]
    fn nested_indents() {
        let syms = vec![
            Symbol { name: "Outer".into(), kind: SymbolKind::Class, start_byte: 0,  end_byte: 100, start_row: 1,  end_row: 10 },
            Symbol { name: "inner".into(), kind: SymbolKind::Method, start_byte: 20, end_byte: 50,  start_row: 3,  end_row: 5 },
            Symbol { name: "Sibling".into(), kind: SymbolKind::Class, start_byte: 110, end_byte: 200, start_row: 12, end_row: 20 },
        ];
        let out = format_symbols_with_callees(&syms, None);
        assert!(out.contains("class Outer"));
        assert!(out.contains("  method inner"));
        assert!(out.contains("class Sibling"));
        assert!(!out.contains("  class Sibling"), "Sibling indented? {}", out);
    }

    #[test]
    fn callees_render_as_arrow_suffix() {
        let syms = vec![
            Symbol { name: "quiet".into(), kind: SymbolKind::Function, start_byte: 0, end_byte: 10, start_row: 1, end_row: 1 },
            Symbol { name: "loud".into(),  kind: SymbolKind::Function, start_byte: 20, end_byte: 40, start_row: 3, end_row: 3 },
        ];
        let callees = vec![
            vec![],
            vec!["helper".into(), "splice".into()],
        ];
        let out = format_symbols_with_callees(&syms, Some(&callees));
        assert!(out.contains("fn quiet L1-L1\n"), "quiet line wrong: {out:?}");
        assert!(!out.lines().next().unwrap().contains('→'), "quiet should be silent: {out:?}");
        assert!(out.contains("fn loud L3-L3 → helper, splice"), "loud suffix wrong: {out:?}");
    }

    #[test]
    fn callees_never_truncated() {
        let syms = vec![Symbol { name: "busy".into(), kind: SymbolKind::Function, start_byte: 0, end_byte: 10, start_row: 1, end_row: 1 }];
        let names: Vec<String> = (0..10).map(|i| format!("fn_{i}")).collect();
        let callees = vec![names];
        let out = format_symbols_with_callees(&syms, Some(&callees));
        assert!(out.contains("fn_9"), "tail missing: {out:?}");
        assert!(!out.contains("more"), "truncation present: {out:?}");
        assert!(out.contains("fn_0"), "head missing: {out:?}");
    }
}
