#[derive(Debug, Clone)]
pub(crate) struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub end_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolKind {
    Function,
    Method,
    Class,
    Impl,
    Const,
    Var,
    Type,
    Enum,
    Module,
    Imports,
    Field,
    Other,
}

impl SymbolKind {
    pub(crate) fn from_tag(tag: &str) -> Self {
        match tag {
            "fn" | "function" => SymbolKind::Function,
            "method" => SymbolKind::Method,
            "class" | "struct" | "trait" | "interface" => SymbolKind::Class,
            "impl" => SymbolKind::Impl,
            "const" => SymbolKind::Const,
            "var" | "let" => SymbolKind::Var,
            "type" => SymbolKind::Type,
            "enum" => SymbolKind::Enum,
            "mod" | "module" | "namespace" => SymbolKind::Module,
            "imports" => SymbolKind::Imports,
            "field" => SymbolKind::Field,
            _ => SymbolKind::Other,
        }
    }

    pub(crate) fn tag(self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Method   => "method",
            SymbolKind::Class    => "class",
            SymbolKind::Impl     => "impl",
            SymbolKind::Const    => "const",
            SymbolKind::Var      => "var",
            SymbolKind::Type     => "type",
            SymbolKind::Enum     => "enum",
            SymbolKind::Module   => "mod",
            SymbolKind::Imports  => "imports",
            SymbolKind::Field    => "field",
            SymbolKind::Other    => "?",
        }
    }
}
