use crate::ast::LangSpec;

pub(crate) fn symbol_query(lang: LangSpec) -> &'static str {
    match lang {
        LangSpec::Rust       => RUST_QUERY,
        LangSpec::Python     => PYTHON_QUERY,
        LangSpec::Go         => GO_QUERY,
        LangSpec::JavaScript => JS_QUERY,
        LangSpec::TypeScript => TS_QUERY,
        LangSpec::Tsx        => TS_QUERY,
    }
}

const RUST_QUERY: &str = r#"
(function_item
  name: (identifier) @fn.name) @fn.def

(struct_item
  name: (type_identifier) @class.name) @class.def

(field_declaration
  name: (field_identifier) @field.name) @field.def

(enum_item
  name: (type_identifier) @enum.name) @enum.def

(trait_item
  name: (type_identifier) @class.name) @class.def

(impl_item
  type: (type_identifier) @impl.name) @impl.def

(type_item
  name: (type_identifier) @type.name) @type.def

(mod_item
  name: (identifier) @mod.name) @mod.def

(const_item
  name: (identifier) @const.name) @const.def

(static_item
  name: (identifier) @const.name) @const.def
"#;

const PYTHON_QUERY: &str = r#"
(function_definition
  name: (identifier) @fn.name) @fn.def

(class_definition
  name: (identifier) @class.name) @class.def
"#;

const GO_QUERY: &str = r#"
(function_declaration
  name: (identifier) @fn.name) @fn.def

(method_declaration
  name: (field_identifier) @method.name) @method.def

(type_declaration
  (type_spec
    name: (type_identifier) @type.name)) @type.def

(const_declaration
  (const_spec
    name: (identifier) @const.name)) @const.def

(var_declaration
  (var_spec
    name: (identifier) @var.name)) @var.def
"#;

const JS_QUERY: &str = r#"
(function_declaration
  name: (identifier) @fn.name) @fn.def

(class_declaration
  name: (identifier) @class.name) @class.def

(method_definition
  name: (property_identifier) @method.name) @method.def

(lexical_declaration
  (variable_declarator
    name: (identifier) @const.name)) @const.def

(variable_declaration
  (variable_declarator
    name: (identifier) @var.name)) @var.def

(function_expression
  name: (identifier) @fn.name) @fn.def

(generator_function_declaration
  name: (identifier) @fn.name) @fn.def
"#;

const TS_QUERY: &str = r#"
(function_declaration
  name: (identifier) @fn.name) @fn.def

(class_declaration
  name: (type_identifier) @class.name) @class.def

(interface_declaration
  name: (type_identifier) @class.name) @class.def

(method_definition
  name: (property_identifier) @method.name) @method.def

(method_signature
  name: (property_identifier) @method.name) @method.def

(type_alias_declaration
  name: (type_identifier) @type.name) @type.def

(enum_declaration
  name: (identifier) @enum.name) @enum.def

(lexical_declaration
  (variable_declarator
    name: (identifier) @const.name)) @const.def

(variable_declaration
  (variable_declarator
    name: (identifier) @var.name)) @var.def

(function_expression
  name: (identifier) @fn.name) @fn.def

(generator_function_declaration
  name: (identifier) @fn.name) @fn.def
"#;
