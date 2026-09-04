use hyperion_bytecode::{Chunk, Opcode};
use hyperion_core::memory::nan_box::Value;
use hyperion_parser::lexer::Token;
use hyperion_parser::parser::ast::{Expr, Program, Stmt};

use crate::symbol_table::SymbolTable;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub enum DefaultValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
    Constant(String),
    ClassConstant(String, String),
    Array(Vec<(Option<DefaultValue>, DefaultValue)>),
    New(String, Vec<DefaultValue>),
}

static GLOBAL_CLOSURE_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static GLOBAL_ANON_CLASS_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledParam {
    pub name: String,
    pub type_hint: Option<String>,
    pub has_default: bool,
    pub default_value: Option<DefaultValue>,
    /// Declared `&$x`: the VM aliases the caller's slot instead of copying it.
    pub by_ref: bool,
    pub is_variadic: bool,
}

pub struct CompiledFunction {
    pub name: String,
    pub arity: usize,
    pub params: Vec<CompiledParam>,
    pub chunk: Chunk,
    pub is_closure: bool,
    pub is_static: bool,
    pub visibility: hyperion_parser::parser::ast::Visibility,
    pub num_locals: usize,
    pub attributes: Vec<CompiledAttribute>,
}

#[derive(Clone, Debug)]
pub struct CompiledAttribute {
    pub name: String,
    pub args: Vec<(Option<String>, Value)>,
}

pub struct CompiledClass {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub traits: Vec<String>,
    pub methods: Vec<CompiledFunction>,
    pub static_properties: Vec<(String, Value, hyperion_parser::parser::ast::Visibility)>,
    pub default_properties: Vec<(String, Value, hyperion_parser::parser::ast::Visibility)>,
    pub attributes: Vec<CompiledAttribute>,
    pub is_interface: bool,
    pub is_trait: bool,
    pub is_enum: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_readonly: bool,
}

#[derive(Clone, Debug)]
pub struct TraitDefinition {
    pub name: String,
    pub uses: Vec<String>,
    pub trait_aliases: Vec<(Option<String>, String, String)>,
    pub methods: Vec<Stmt>,
    pub properties: Vec<Stmt>,
    pub aliases: HashMap<String, String>,
    pub namespace: String,
}

pub struct CompilationResult {
    pub main_chunk: Chunk,
    pub functions: Vec<CompiledFunction>,
    pub classes: Vec<CompiledClass>,
    pub class_registry: Vec<String>,
    pub traits_registry: HashMap<String, TraitDefinition>,
    pub interfaces_registry: HashMap<String, Stmt>,
    /// `(interface, constant, value)` for every `const` declared in an interface
    /// body, so `HasVersion::VERSION` resolves without the interface having to be
    /// registered as a class.
    ///
    /// Registering one would be the shorter route, but an interface is not a
    /// class: `class_exists()` would start answering true for it, and Laravel's
    /// container branches on exactly that before trying to instantiate. These
    /// land straight in the static table instead, which is where `Foo::NAME`
    /// looks.
    pub interface_constants: Vec<(String, String, Value)>,
}

pub struct Compiler {
    chunk: Chunk,
    symbol_table: SymbolTable,
    pub functions: Vec<CompiledFunction>,
    classes: Vec<CompiledClass>,
    class_registry: Vec<String>, // Maps class_id -> class_name
    break_targets: Vec<Vec<usize>>,
    continue_targets: Vec<Vec<usize>>,
    current_namespace: String,
    current_class: Option<String>,
    compiling_class_extends: Option<String>,
    compiling_class_constants: Vec<(String, Value)>,
    current_function: Option<String>,
    current_trait: Option<String>,
    aliases: HashMap<String, String>,
    traits_registry: HashMap<String, TraitDefinition>,
    interfaces_registry: HashMap<String, Stmt>,
    /// Constants declared in interface bodies — see `CompilationResult`.
    interface_constants: Vec<(String, String, Value)>,
    /// Which parameter positions of each function/method are declared `&$x`.
    /// Keyed by lowercase short name — a call site is compiled before the
    /// declaration is reached, so this is filled by a pre-pass over the program.
    /// Only positions listed here get a ref pushed; the VM's arg-binding funnel
    /// collapses everything else back to a value.
    by_ref_params: HashMap<String, Vec<usize>>,
    pub file_path: String,
    foreach_depth: usize,
    in_nullsafe_chain: bool,
    nullsafe_exit_jumps: Vec<usize>,
}

impl Compiler {
    pub fn new(file_path: String) -> Self {
        let abs_path = std::fs::canonicalize(&file_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(file_path.clone());
        let mut chunk = Chunk::new();
        // Reserve space for locals (will be patched later)
        chunk.write_opcode(Opcode::Reserve);
        chunk.write_byte(0);
        let mut class_registry = Vec::new();
        let builtins = vec![
            "stdClass",
            "Exception",
            "Error",
            "Closure",
            "Throwable",
            "ArrayAccess",
            "Countable",
            "Iterator",
            "IteratorAggregate",
            "Serializable",
            "Stringable",
            "JsonSerializable",
            "Generator",
            "UnitEnum",
            "BackedEnum",
            "ReflectionFunction",
            "ReflectionMethod",
            "ReflectionClass",
            "ReflectionObject",
            "ReflectionProperty",
            "ReflectionParameter",
            "ReflectionType",
            "ReflectionNamedType",
            "ReflectionException",
        ];
        for b in builtins {
            class_registry.push(b.to_string());
        }

        let mut interfaces_registry = HashMap::new();
        // Insert dummy AST for ArrayAccess to pass validation
        interfaces_registry.insert(
            "ArrayAccess".to_string(),
            Stmt::Interface {
                name: "ArrayAccess".to_string(),
                extends: vec![],
                methods: vec![
                    Stmt::Function {
                        name: "offsetExists".to_string(),
                        params: vec![hyperion_parser::parser::ast::ParamDef {
                            name: "offset".to_string(),
                            type_hint: None,
                            has_default: false,
                            default_expr: None,
                            is_variadic: false,
                            by_ref: false,
                            promoted_visibility: None,
                        }],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "offsetGet".to_string(),
                        params: vec![hyperion_parser::parser::ast::ParamDef {
                            name: "offset".to_string(),
                            type_hint: None,
                            has_default: false,
                            default_expr: None,
                            is_variadic: false,
                            by_ref: false,
                            promoted_visibility: None,
                        }],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "offsetSet".to_string(),
                        params: vec![
                            hyperion_parser::parser::ast::ParamDef {
                                name: "offset".to_string(),
                                type_hint: None,
                                has_default: false,
                                default_expr: None,
                                is_variadic: false,
                            by_ref: false,
                                promoted_visibility: None,
                            },
                            hyperion_parser::parser::ast::ParamDef {
                                name: "value".to_string(),
                                type_hint: None,
                                has_default: false,
                                default_expr: None,
                                is_variadic: false,
                            by_ref: false,
                                promoted_visibility: None,
                            },
                        ],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "offsetUnset".to_string(),
                        params: vec![hyperion_parser::parser::ast::ParamDef {
                            name: "offset".to_string(),
                            type_hint: None,
                            has_default: false,
                            default_expr: None,
                            is_variadic: false,
                            by_ref: false,
                            promoted_visibility: None,
                        }],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                ],
            },
        );
        interfaces_registry.insert(
            "Countable".to_string(),
            Stmt::Interface {
                name: "Countable".to_string(),
                extends: vec![],
                methods: vec![Stmt::Function {
                    name: "count".to_string(),
                    params: vec![],
                    body: vec![],
                    is_static: false,
                    visibility: hyperion_parser::parser::ast::Visibility::Public,
                    attributes: vec![],
                }],
            },
        );
        interfaces_registry.insert(
            "Iterator".to_string(),
            Stmt::Interface {
                name: "Iterator".to_string(),
                extends: vec![],
                methods: vec![
                    Stmt::Function {
                        name: "current".to_string(),
                        params: vec![],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "key".to_string(),
                        params: vec![],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "next".to_string(),
                        params: vec![],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "rewind".to_string(),
                        params: vec![],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                    Stmt::Function {
                        name: "valid".to_string(),
                        params: vec![],
                        body: vec![],
                        is_static: false,
                        visibility: hyperion_parser::parser::ast::Visibility::Public,
                        attributes: vec![],
                    },
                ],
            },
        );

        Compiler {
            chunk,
            symbol_table: SymbolTable::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            class_registry,
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            current_namespace: String::new(),
            current_class: None,
            compiling_class_extends: None,
            compiling_class_constants: Vec::new(),
            current_function: None,
            current_trait: None,
            aliases: HashMap::new(),
            traits_registry: HashMap::new(),
            interfaces_registry,
            interface_constants: Vec::new(),
            by_ref_params: HashMap::new(),
            file_path: abs_path,
            foreach_depth: 0,
            in_nullsafe_chain: false,
            nullsafe_exit_jumps: Vec::new(),
        }
    }

    pub fn new_child(&self) -> Self {
        let mut child = Compiler::new(self.file_path.clone());
        child.current_namespace = self.current_namespace.clone();
        child.current_class = self.current_class.clone();
        child.compiling_class_extends = self.compiling_class_extends.clone();
        child.compiling_class_constants = self.compiling_class_constants.clone();
        child.current_function = self.current_function.clone();
        child.current_trait = self.current_trait.clone();
        child.aliases = self.aliases.clone();
        child.traits_registry = self.traits_registry.clone();
        child.interfaces_registry = self.interfaces_registry.clone();
        child.class_registry = self.class_registry.clone();
        child.by_ref_params = self.by_ref_params.clone();
        child.foreach_depth = self.foreach_depth;
        child
    }

    pub fn prefill_locals(&mut self, locals: &[String]) {
        for local in locals {
            self.symbol_table.add_local(local);
        }
    }

    pub fn resolve_class_name(&self, name: &str) -> String {
        if name.starts_with('$') {
            return name.to_string();
        }
        if name.eq_ignore_ascii_case("self") {
            if let Some(ref cur) = self.current_class {
                return cur.clone();
            }
            return name.to_string();
        }
        if name == "static" || name == "parent" {
            return name.to_string();
        }

        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        let first_segment = name.split('\\').next().unwrap_or(name);
        if let Some(fqcn) = self.aliases.get(first_segment) {
            if first_segment == name {
                return fqcn.clone();
            } else {
                let rest = &name[first_segment.len()..];
                return format!("{}{}", fqcn, rest);
            }
        }

        if !self.current_namespace.is_empty() {
            return format!("{}\\{}", self.current_namespace, name);
        }

        name.to_string()
    }

    /// Emit code that yields a reference to an lvalue.
    ///
    /// Only lvalues can be referenced. Taking `&` of anything else is a PHP
    /// compile error; here we degrade to evaluating the operand by value so a
    /// stray `&` cannot take down the whole parse.
    fn compile_make_ref(&mut self, target: Expr) {
        match target {
            Expr::Variable(name) => {
                if let Some(id) = Self::get_superglobal_id(&name) {
                    self.chunk.write_opcode(Opcode::GetSuperglobal);
                    self.chunk.write_byte(id);
                } else {
                    let local_idx = self.resolve_local(&name);
                    self.chunk.write_opcode(Opcode::MakeRefLocal);
                    self.chunk.write_byte(local_idx);
                }
            }
            Expr::ArrayGet { array, key } => {
                // Referencing `$a['k']` must be able to *create* the element
                // (PHP autovivifies), so the container is referenced too —
                // that is what makes `$cur = &$cur['a']` chains work.
                self.compile_make_ref(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    self.chunk.write_opcode(Opcode::Constant);
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::MakeRefElement);
            }
            other => self.compile_expr(other),
        }
    }
    /// Compile the *container* side of an `ArraySet` expression.
    ///
    /// For a plain variable / property / method-call the container is emitted
    /// with `compile_expr`.  For an `ArrayGet { array, key }` we emit the
    /// parent container recursively then the key, then `ArrayGetForWrite` so
    /// that a missing key is auto-vivified: a new empty `PhpArray` is created,
    /// stored back at that key in the parent, and its handle is pushed for the
    /// subsequent `ArraySet` to fill in.
    ///
    /// This makes `$this->routes['GET']['/'] = $route` work even when
    /// `$this->routes['GET']` has never been assigned before.
    fn compile_array_lhs(&mut self, expr: Expr) {
        match expr {
            Expr::ArrayGet { array, key } => {
                // Recursively compile the parent in "lhs" mode so arbitrarily
                // deep chains all get auto-vivification.
                self.compile_array_lhs(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    self.chunk.write_opcode(Opcode::Constant);
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::ArrayGetForWrite);
            }
            Expr::Variable(ref name) => {
                if let Some(id) = Self::get_superglobal_id(name) {
                    self.chunk.write_opcode(Opcode::GetSuperglobal);
                    self.chunk.write_byte(id);
                } else {
                    let local_idx = self.resolve_local(name);
                    self.chunk.write_opcode(Opcode::EnsureLocalArray);
                    self.chunk.write_byte(local_idx);
                }
            }
            Expr::PropertyGet { object, property } => {
                if let Expr::Identifier(ref name) = *property {
                    self.compile_expr(*object);
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetPropertyForWrite);
                    self.chunk.write_short(name_idx);
                } else if let Expr::LiteralString(ref name) = *property {
                    self.compile_expr(*object);
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetPropertyForWrite);
                    self.chunk.write_short(name_idx);
                } else {
                    self.compile_expr(*object);
                    self.compile_expr(*property);
                    self.chunk.write_opcode(Opcode::GetPropertyDynamicForWrite);
                }
            }
            other => self.compile_expr(other),
        }
    }

    fn compile_destructure_target(&mut self, target: &Expr) {
        match target {
            Expr::Variable(name) => {
                let local_idx = self.resolve_local(name);
                self.chunk.write_opcode(Opcode::SetLocal);
                self.chunk.write_byte(local_idx);
                self.chunk.write_opcode(Opcode::Pop);
            }
            Expr::PropertyGet { object, property } => {
                let tmp_idx = self.resolve_local("#destruct_tmp");
                self.chunk.write_opcode(Opcode::SetLocal);
                self.chunk.write_byte(tmp_idx);
                self.chunk.write_opcode(Opcode::Pop);

                if let Expr::Identifier(ref name) = **property {
                    self.compile_expr((**object).clone());
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(name.clone())) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::GetLocal);
                    self.chunk.write_byte(tmp_idx);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(prop_idx);
                    self.chunk.write_opcode(Opcode::Pop);
                } else if let Expr::LiteralString(ref name) = **property {
                    self.compile_expr((**object).clone());
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(name.clone())) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::GetLocal);
                    self.chunk.write_byte(tmp_idx);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(prop_idx);
                    self.chunk.write_opcode(Opcode::Pop);
                } else {
                    self.compile_expr((**object).clone());
                    self.compile_expr((**property).clone());
                    self.chunk.write_opcode(Opcode::GetLocal);
                    self.chunk.write_byte(tmp_idx);
                    self.chunk.write_opcode(Opcode::SetPropertyDynamic);
                    self.chunk.write_opcode(Opcode::Pop);
                }
            }
            Expr::ArrayGet { array, key } => {
                let tmp_idx = self.resolve_local("#destruct_tmp");
                self.chunk.write_opcode(Opcode::SetLocal);
                self.chunk.write_byte(tmp_idx);
                self.chunk.write_opcode(Opcode::Pop);

                self.compile_array_lhs((**array).clone());
                if let Some(ref k) = key {
                    self.compile_expr((**k).clone());
                } else {
                    self.chunk.write_opcode(Opcode::Constant);
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::GetLocal);
                self.chunk.write_byte(tmp_idx);
                self.chunk.write_opcode(Opcode::ArraySet);
                self.chunk.write_opcode(Opcode::Pop);
            }
            Expr::StaticPropertyGet { class_name, property } => {
                let tmp_idx = self.resolve_local("#destruct_tmp");
                self.chunk.write_opcode(Opcode::SetLocal);
                self.chunk.write_byte(tmp_idx);
                self.chunk.write_opcode(Opcode::Pop);

                let prop_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(
                        Box::into_raw(Box::new(property.clone())) as *mut ()
                    ));
                let resolved_class = self.resolve_class_name(&class_name);
                let class_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(
                        Box::into_raw(Box::new(resolved_class)) as *mut ()
                    ));
                self.chunk.write_opcode(Opcode::GetLocal);
                self.chunk.write_byte(tmp_idx);
                self.chunk.write_opcode(Opcode::SetStatic);
                self.chunk.write_short(class_idx);
                self.chunk.write_short(prop_idx);
                self.chunk.write_opcode(Opcode::Pop);
            }
            Expr::ArrayDestructure { elements, .. } | Expr::Array(elements) => {
                let mut auto_idx = 0i32;
                for (key_opt, sub_target_opt) in elements {
                    if let Some(sub_target) = sub_target_opt {
                        self.chunk.write_opcode(Opcode::Dup);
                        if let Some(key_expr) = key_opt {
                            self.compile_expr(key_expr.clone());
                        } else {
                            let idx = self.chunk.add_constant(Value::new_int(auto_idx));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(idx);
                        }
                        self.chunk.write_opcode(Opcode::ArrayGet);
                        self.compile_destructure_target(sub_target);
                    }
                    if key_opt.is_none() {
                        auto_idx += 1;
                    }
                }
                self.chunk.write_opcode(Opcode::Pop);
            }
            Expr::ListDestructure { vars, .. } | Expr::List(vars) => {
                let mut auto_idx = 0i32;
                for var_opt in vars {
                    if let Some(sub_target) = var_opt {
                        self.chunk.write_opcode(Opcode::Dup);
                        let idx = self.chunk.add_constant(Value::new_int(auto_idx));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                        self.chunk.write_opcode(Opcode::ArrayGet);
                        self.compile_destructure_target(sub_target);
                    }
                    auto_idx += 1;
                }
                self.chunk.write_opcode(Opcode::Pop);
            }
            _ => {
                self.chunk.write_opcode(Opcode::Pop);
            }
        }
    }

    pub fn compile_param(&self, p: hyperion_parser::parser::ast::ParamDef) -> CompiledParam {        let type_hint = p.type_hint.map(|th| self.resolve_type_hint(&th));
        let default_value = p
            .default_expr
            .as_ref()
            .and_then(|expr| self.get_default_value(expr));
        CompiledParam {
            name: p.name,
            type_hint,
            has_default: p.has_default,
            default_value,
            by_ref: p.by_ref,
            is_variadic: p.is_variadic,
        }
    }

    pub fn resolve_type_hint(&self, hint: &str) -> String {
        if hint.contains('|') {
            hint.split('|')
                .map(|part| self.resolve_single_type_hint(part))
                .collect::<Vec<_>>()
                .join("|")
        } else if hint.contains('&') {
            hint.split('&')
                .map(|part| self.resolve_single_type_hint(part))
                .collect::<Vec<_>>()
                .join("&")
        } else {
            self.resolve_single_type_hint(hint)
        }
    }

    fn resolve_single_type_hint(&self, hint: &str) -> String {
        let mut clean_hint = hint;
        let mut prefix = "";
        if clean_hint.starts_with('?') {
            prefix = "?";
            clean_hint = &clean_hint[1..];
        }

        match clean_hint.to_lowercase().as_str() {
            "int" | "integer" | "string" | "bool" | "boolean" | "float" | "double" | "array"
            | "callable" | "iterable" | "mixed" | "object" | "void" | "null" | "self"
            | "parent" | "static" => hint.to_string(),
            _ => {
                format!("{}{}", prefix, self.resolve_class_name(clean_hint))
            }
        }
    }

    fn get_default_value(&self, expr: &Expr) -> Option<DefaultValue> {
        match expr {
            Expr::LiteralInt(i) => Some(DefaultValue::Int(*i)),
            Expr::LiteralFloat(f) => Some(DefaultValue::Float(*f)),
            Expr::LiteralString(s) => Some(DefaultValue::String(s.clone())),
            Expr::LiteralNull => Some(DefaultValue::Null),
            Expr::LiteralBool(b) => Some(DefaultValue::Bool(*b)),
            Expr::UnaryMinus(inner) => {
                match &**inner {
                    Expr::LiteralInt(i) => Some(DefaultValue::Int(-*i)),
                    Expr::LiteralFloat(f) => Some(DefaultValue::Float(-*f)),
                    _ => None,
                }
            }
            Expr::Identifier(name) => Some(DefaultValue::Constant(name.clone())),
            Expr::ClassConstFetch { class_name, constant_name } => {
                let resolved = if class_name == "self" || class_name == "static" {
                    self.current_class.clone().unwrap_or_else(|| class_name.clone())
                } else {
                    self.resolve_class_name(class_name)
                };
                Some(DefaultValue::ClassConstant(resolved, constant_name.clone()))
            }
            Expr::Array(elements) => {
                let mut items = Vec::new();
                for (k, v_opt) in elements {
                    let key = if let Some(ref key_expr) = k {
                        Some(self.get_default_value(key_expr)?)
                    } else {
                        None
                    };
                    if let Some(ref v) = v_opt {
                        let val = self.get_default_value(v)?;
                        items.push((key, val));
                    }
                }
                Some(DefaultValue::Array(items))
            }
            Expr::New { class_name, arguments } => {
                let resolved = self.resolve_class_name(class_name);
                let mut args = Vec::new();
                for arg in arguments {
                    match arg {
                        Expr::NamedArgument { value, .. } => {
                            args.push(self.get_default_value(value)?);
                        }
                        _ => {
                            args.push(self.get_default_value(arg)?);
                        }
                    }
                }
                Some(DefaultValue::New(resolved, args))
            }
            _ => None,
        }
    }



    fn get_superglobal_id(name: &str) -> Option<u8> {
        match name {
            "_SERVER" => Some(0),
            "_GET" => Some(1),
            "_POST" => Some(2),
            "_COOKIE" => Some(3),
            "_FILES" => Some(4),
            "_ENV" => Some(5),
            "_REQUEST" => Some(6),
            "GLOBALS" => Some(7),
            _ => None,
        }
    }

    pub fn compile(mut self, program: Program) -> CompilationResult {
        // Signatures first: a call may be compiled before the function it calls
        // is declared, and the by-ref decision has to be made at the call site.
        self.collect_by_ref_params(&program);

        let is_worker_active = std::env::var("HYPERION_WORKER").is_ok()
            || std::env::var("HYPERION_OCTANE").is_ok();
        
        let is_entrypoint = self.file_path.ends_with("public/index.php") 
            || self.file_path.ends_with("index.php");

        if is_worker_active && is_entrypoint {
            let total_stmts = program.len();
            for (idx, stmt) in program.into_iter().enumerate() {
                let is_last_stmt = idx + 1 == total_stmts;
                if is_last_stmt {
                    let loop_start = self.chunk.code.len();
                    
                    let fn_name_ptr = Box::into_raw(Box::new("hyperion_accept_request".to_string()));
                    let fn_idx = self.chunk.add_constant(Value::new_string_ptr(fn_name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(fn_idx);
                    self.chunk.write_opcode(Opcode::Call);
                    self.chunk.write_byte(0); // arity = 0

                    let exit_jump = self.emit_jump(Opcode::JumpIfFalse);

                    self.compile_stmt(stmt);

                    self.emit_loop(loop_start);
                    self.patch_jump(exit_jump);
                } else {


                    self.compile_stmt(stmt);
                }
            }
        } else {
            for stmt in program {
                self.compile_stmt(stmt);
            }
        }

        // Patch the Reserve instruction at index 1
        let num_locals = self.symbol_table.locals_count() as u8;
        self.chunk.local_names = self
            .symbol_table
            .locals
            .iter()
            .map(|l| l.name.clone())
            .collect();
        self.chunk.code[1] = num_locals;

        // Push null as the default return value of the script
        let null_idx = self
            .chunk
            .add_constant(hyperion_core::memory::nan_box::Value::null());
        self.chunk.write_opcode(Opcode::Constant);
        self.chunk.write_short(null_idx);
        self.chunk.write_opcode(Opcode::ReturnValue);

        CompilationResult {
            main_chunk: self.chunk,
            functions: self.functions,
            classes: self.classes,
            class_registry: self.class_registry,
            traits_registry: self.traits_registry,
            interfaces_registry: self.interfaces_registry,
            interface_constants: self.interface_constants,
        }
    }

    fn emit_jump(&mut self, opcode: Opcode) -> usize {
        self.chunk.write_opcode(opcode);
        self.chunk.write_short(0xffff); // Placeholder
        self.chunk.code.len() - 2 // Return the offset of the placeholder
    }

    fn patch_jump(&mut self, offset: usize) {
        // Calculate jump distance: current length - offset - 2 (the 2 bytes of the jump itself)
        // Actually, if offset is where the placeholder starts, current length is where we are jumping to.
        // Jump distance is from AFTER the placeholder to the current location.
        // The placeholder is at `offset` and `offset+1`.
        // Next instruction starts at `offset+2`.
        let jump = self.chunk.code.len() - offset - 2;
        if jump > u16::MAX as usize {
            panic!("Too much code to jump over (exceeds 16-bit limit).");
        }
        self.chunk.patch_short(offset, jump as u16);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.chunk.write_opcode(Opcode::Loop);
        // Calculate distance from AFTER the loop instruction to the loop_start
        let jump = self.chunk.code.len() + 2 - loop_start;
        if jump > u16::MAX as usize {
            panic!("Loop body too large (exceeds 16-bit limit).");
        }
        self.chunk.write_short(jump as u16);
    }

    /// Record every `&$x` parameter position in the program before compiling it,
    /// so call sites that appear above a declaration still bind by reference.
    pub fn collect_by_ref_params(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Function { name, params, body, .. } => {
                    let positions: Vec<usize> = params
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.by_ref)
                        .map(|(i, _)| i)
                        .collect();
                    if !positions.is_empty() {
                        let short = name.rsplit('\\').next().unwrap_or(name).to_lowercase();
                        self.by_ref_params.insert(short.clone(), positions.clone());
                        hyperion_core::types::native_signatures::register_by_ref_param(&short, positions);
                    }
                    self.collect_by_ref_params(body);
                }
                Stmt::Class { methods, .. } => self.collect_by_ref_params(methods),
                Stmt::Trait { methods, .. } => self.collect_by_ref_params(methods),
                Stmt::Interface { methods, .. } => self.collect_by_ref_params(methods),
                Stmt::Enum { methods, .. } => self.collect_by_ref_params(methods),
                Stmt::Block(stmts) => self.collect_by_ref_params(stmts),
                Stmt::If { then_branch, else_branch, .. } => {
                    self.collect_by_ref_params(then_branch);
                    if let Some(else_branch) = else_branch {
                        self.collect_by_ref_params(else_branch);
                    }
                }
                Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => self.collect_by_ref_params(body),
                Stmt::For { body, .. } | Stmt::Foreach { body, .. } => self.collect_by_ref_params(body),
                Stmt::Switch { cases, default, .. } => {
                    for (_, case_stmts) in cases {
                        self.collect_by_ref_params(case_stmts);
                    }
                    if let Some(default_stmts) = default {
                        self.collect_by_ref_params(default_stmts);
                    }
                }
                Stmt::TryCatch { try_body, catches, finally_body, .. } => {
                    self.collect_by_ref_params(try_body);
                    for catch in catches {
                        self.collect_by_ref_params(&catch.body);
                    }
                    if let Some(finally_body) = finally_body {
                        self.collect_by_ref_params(finally_body);
                    }
                }
                _ => {}
            }
        }
    }

    /// True when argument `index` of `callee` must be passed by reference.
    /// Native out-parameters come from the shared table in `hyperion-core`, so
    /// the thunk the VM builds for the same native agrees with what we emit.
    fn arg_is_by_ref(&self, callee: &str, index: usize) -> bool {
        if hyperion_core::types::native_signatures::is_by_ref_param(callee, index) {
            return true;
        }
        let short = callee.rsplit('\\').next().unwrap_or(callee).to_lowercase();
        self.by_ref_params
            .get(&short)
            .is_some_and(|v| v.contains(&index))
    }

    /// Emit one call argument, taking a reference when the callee's signature
    /// asks for it and the argument is something a reference can point at.
    fn compile_argument(&mut self, callee: &str, index: usize, arg: Expr) {
        let refable = matches!(arg, Expr::Variable(_) | Expr::ArrayGet { .. });
        if refable && self.arg_is_by_ref(callee, index) {
            self.compile_make_ref(arg);
        } else {
            self.compile_expr(arg);
        }
    }

    fn resolve_local(&mut self, name: &str) -> u8 {
        if let Some(idx) = self.symbol_table.resolve_local(name) {
            idx
        } else {
            self.symbol_table.add_local(name)
        }
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr);
                self.chunk.write_opcode(Opcode::Pop);
            }
            Stmt::Echo(exprs) => {
                // `echo a, b, c;` writes each operand in turn.
                for expr in exprs {
                    self.compile_expr(expr);
                    self.chunk.write_opcode(Opcode::Echo);
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expr(condition);
                let then_jump = self.emit_jump(Opcode::JumpIfFalse);

                // Typically need to pop the condition value if it's evaluated,
                // but let's assume the VM handles the pop in JumpIfFalse.
                for stmt in then_branch {
                    self.compile_stmt(stmt);
                }

                if let Some(else_branch_stmts) = else_branch {
                    let else_jump = self.emit_jump(Opcode::Jump);
                    self.patch_jump(then_jump);
                    for stmt in else_branch_stmts {
                        self.compile_stmt(stmt);
                    }
                    self.patch_jump(else_jump);
                } else {
                    self.patch_jump(then_jump);
                }
            }
            Stmt::Foreach {
                iterable,
                key_var,
                value_var,
                value_by_ref,
                body,
            } => {
                // `foreach ($a as &$v)` must iterate the array in place, so the
                // iterable is referenced rather than copied onto the stack.
                if value_by_ref {
                    self.compile_make_ref(iterable);
                } else {
                    self.compile_expr(iterable);
                }
                self.chunk.write_opcode(Opcode::IterInit);
                self.chunk.write_short(0xffff);
                let iter_init_jump = self.chunk.code.len() - 2;

                let loop_start = self.chunk.code.len();

                self.break_targets.push(Vec::new());
                self.continue_targets.push(Vec::new());

                let key_local = if let Some(k) = key_var {
                    self.resolve_local(&k)
                } else {
                    255 // means no key
                };
                let value_local = self.resolve_local(&value_var);

                self.chunk.write_opcode(Opcode::IterNext);
                self.chunk.write_byte(key_local);
                self.chunk.write_byte(value_local);
                self.chunk.write_byte(value_by_ref as u8);
                self.chunk.write_short(0xffff);
                let iter_next_jump_offset = self.chunk.code.len() - 2;

                self.foreach_depth += 1;
                for stmt in body {
                    self.compile_stmt(stmt);
                }
                self.foreach_depth -= 1;

                let continues = self.continue_targets.pop().unwrap();
                for c in continues {
                    self.patch_jump(c);
                }

                self.emit_loop(loop_start);

                self.patch_jump(iter_init_jump);
                self.patch_jump(iter_next_jump_offset);

                let breaks = self.break_targets.pop().unwrap();
                for b in breaks {
                    self.patch_jump(b);
                }

                self.chunk.write_opcode(Opcode::Pop); // pop index
                self.chunk.write_opcode(Opcode::Pop); // pop iterable
            }
            Stmt::While { condition, body } => {
                let loop_start = self.chunk.code.len();
                self.compile_expr(condition);
                let exit_jump = self.emit_jump(Opcode::JumpIfFalse);

                self.break_targets.push(Vec::new());
                self.continue_targets.push(Vec::new());

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                let continues = self.continue_targets.pop().unwrap();
                for c in continues {
                    self.patch_jump(c);
                }

                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);

                let breaks = self.break_targets.pop().unwrap();
                for b in breaks {
                    self.patch_jump(b);
                }
            }
            Stmt::For {
                init,
                condition,
                increment,
                body,
            } => {
                for expr in init {
                    self.compile_expr(expr.clone());
                    self.chunk.write_opcode(Opcode::Pop);
                }

                let loop_start = self.chunk.code.len();

                let mut exit_jump = None;
                if !condition.is_empty() {
                    for (i, expr) in condition.iter().enumerate() {
                        self.compile_expr(expr.clone());
                        if i < condition.len() - 1 {
                            self.chunk.write_opcode(Opcode::Pop);
                        }
                    }
                    exit_jump = Some(self.emit_jump(Opcode::JumpIfFalse));
                }

                self.break_targets.push(Vec::new());
                self.continue_targets.push(Vec::new());

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                let continues = self.continue_targets.pop().unwrap();
                for c in continues {
                    self.patch_jump(c);
                }

                for expr in increment {
                    self.compile_expr(expr.clone());
                    self.chunk.write_opcode(Opcode::Pop);
                }

                self.emit_loop(loop_start);

                if let Some(jump) = exit_jump {
                    self.patch_jump(jump);
                }

                let breaks = self.break_targets.pop().unwrap();
                for b in breaks {
                    self.patch_jump(b);
                }
            }
            Stmt::Function {
                name, params, body, is_static, visibility, attributes,
            } => {
                let mut compiled_attrs = Vec::new();
                for attr in &attributes {
                    let mut resolved_args = Vec::new();
                    for arg in &attr.arguments {
                        match arg {
                            Expr::NamedArgument { name, value } => {
                                let val = expr_to_literal_value(value, self);
                                resolved_args.push((Some(name.clone()), val));
                            }
                            _ => {
                                let val = expr_to_literal_value(arg, self);
                                resolved_args.push((None, val));
                            }
                        }
                    }
                    let resolved_attr_name = self.resolve_class_name(&attr.name);
                    compiled_attrs.push(CompiledAttribute {
                        name: resolved_attr_name,
                        args: resolved_args,
                    });
                }

                let mut fn_compiler = self.new_child();
                fn_compiler.current_function = Some(name.clone());
                fn_compiler.symbol_table.add_local("this"); // Reserve local 0 for func_val/obj_val
                for param in &params {
                    fn_compiler.symbol_table.add_local(&param.name);
                }

                let is_generator = has_yield_stmt(&body);
                if is_generator {
                    fn_compiler.chunk.write_opcode(Opcode::CreateGenerator);
                }

                // Inject variadic packing if needed
                let mut variadic_index = None;
                for (i, p) in params.iter().enumerate() {
                    if p.is_variadic {
                        variadic_index = Some(i as u8);
                    }
                }
                if let Some(_idx) = variadic_index {
                    fn_compiler.chunk.write_opcode(Opcode::PackVariadic);
                    fn_compiler.chunk.write_byte(params.len() as u8 - 1);
                }

                for fn_stmt in body {
                    fn_compiler.compile_stmt(fn_stmt);
                }

                let num_locals = fn_compiler.symbol_table.locals_count() as u8;
                fn_compiler.chunk.local_names = fn_compiler
                    .symbol_table
                    .locals
                    .iter()
                    .map(|l| l.name.clone())
                    .collect();
                fn_compiler.chunk.code[1] = num_locals; // Patch Reserve opcode

                let null_idx = fn_compiler.chunk.add_constant(Value::null());
                fn_compiler.chunk.write_opcode(Opcode::Constant);
                fn_compiler.chunk.write_short(null_idx);
                fn_compiler.chunk.write_opcode(Opcode::ReturnValue);

                let mut required_arity = 0;
                for p in &params {
                    if !p.has_default && !p.is_variadic {
                        required_arity += 1;
                    }
                }

                let compiled_params = params
                    .into_iter()
                    .map(|p| fn_compiler.compile_param(p))
                    .collect::<Vec<_>>();

                self.functions.extend(fn_compiler.functions);
                self.classes.extend(fn_compiler.classes);
                let resolved_name = if !self.current_namespace.is_empty() {
                    format!("{}\\{}", self.current_namespace, name)
                } else {
                    name.clone()
                };
                self.functions.push(CompiledFunction {
                    name: resolved_name,
                    arity: required_arity,
                    params: compiled_params,
                    chunk: fn_compiler.chunk,
                    is_closure: false,
                    is_static,
                    visibility,
                    num_locals: fn_compiler.symbol_table.locals_count(),
                    attributes: compiled_attrs,
                });
            }

            Stmt::Return(expr) => {
                self.compile_expr(expr);
                self.chunk.write_opcode(Opcode::ReturnValue);
            }
            Stmt::Namespace(name) => {
                self.current_namespace = name;
            }
            Stmt::Use { class_name, alias } => {
                let first_segment = class_name
                    .split('\\')
                    .last()
                    .unwrap_or(&class_name)
                    .to_string();
                let key = alias.unwrap_or(first_segment);
                self.aliases.insert(key, class_name);
            }
            Stmt::Trait {
                name,
                uses,
                trait_aliases,
                methods,
                properties,
            } => {
                let resolved_name = self.resolve_class_name(&name);
                let resolved_uses: Vec<String> = uses.iter().map(|u| self.resolve_class_name(u)).collect();
                hyperion_core::hyp_debug!("DEBUG COMPILER: Registered trait {} with uses {:?}", resolved_name, resolved_uses);
                self.traits_registry.insert(
                    resolved_name.clone(),
                    TraitDefinition {
                        name: resolved_name,
                        uses: resolved_uses,
                        trait_aliases,
                        methods,
                        properties,
                        aliases: self.aliases.clone(),
                        namespace: self.current_namespace.clone(),
                    },
                );
            }
            Stmt::Interface { name, extends, methods } => {
                let resolved_name = self.resolve_class_name(&name);
                let resolved_extends: Vec<String> = extends.iter().map(|e| self.resolve_class_name(e)).collect();
                // An interface constant is readable off the interface itself
                // (`HasVersion::VERSION`), not only off implementors. Implementors
                // get their own copy when the class is compiled — that copy is
                // what lets a class override the constant — so this side only has
                // to make the interface's own name resolvable.
                let mut static_properties = Vec::new();
                for member in &methods {
                    if let Stmt::ConstDeclaration { name: c_name, value, visibility } = member {
                        let val = expr_to_literal_value(value, self);
                        self.interface_constants.push((
                            resolved_name.clone(),
                            c_name.clone(),
                            val,
                        ));
                        static_properties.push((c_name.clone(), val, *visibility));
                    }
                }
                self.interfaces_registry.insert(
                    resolved_name.clone(),
                    Stmt::Interface {
                        name: resolved_name.clone(),
                        extends: resolved_extends.clone(),
                        methods: methods.clone(),
                    },
                );

                let mut compiled_methods = Vec::new();
                for member in &methods {
                    if let Stmt::Function {
                        name: m_name,
                        params,
                        is_static,
                        visibility,
                        ..
                    } = member {
                        let mut compiled_params = Vec::new();
                        let mut required_arity = 0;
                        for param in params {
                            let type_hint = param.type_hint.as_ref().map(|th| self.resolve_type_hint(th));
                            if !param.has_default {
                                required_arity += 1;
                            }
                            compiled_params.push(CompiledParam {
                                name: param.name.clone(),
                                type_hint,
                                has_default: param.has_default,
                                default_value: None,
                                by_ref: param.by_ref,
                                is_variadic: param.is_variadic,
                            });
                        }
                        let mut chunk = hyperion_bytecode::Chunk::new();
                        chunk.write_opcode(hyperion_bytecode::Opcode::Return);
                        compiled_methods.push(CompiledFunction {
                            name: m_name.clone(),
                            arity: required_arity,
                            params: compiled_params,
                            chunk,
                            is_closure: false,
                            is_static: *is_static,
                            visibility: *visibility,
                            num_locals: 0,
                            attributes: Vec::new(),
                        });
                    }
                }

                self.classes.push(CompiledClass {
                    name: resolved_name,
                    methods: compiled_methods,
                    extends: None,
                    implements: resolved_extends,
                    traits: Vec::new(),
                    static_properties,
                    default_properties: Vec::new(),
                    attributes: Vec::new(),
                    is_interface: true,
                    is_trait: false,
                    is_enum: false,
                    is_abstract: false,
                    is_final: false,
                    is_readonly: false,
                });
            }
            Stmt::PropertyDeclaration { .. } => {
                // Ignore for now. They will be processed during object instantiation or class registration in future.
            }
            Stmt::ConstDeclaration { name, value, .. } => {
                // Reached only for a `const` outside any class body, which
                // declares a global constant. Class-body constants are collected
                // from `properties` when the class itself is compiled.
                //
                // The value is compiled as an expression rather than folded, so
                // `const PATH = __DIR__ . '/x'` and a reference to an earlier
                // constant both work; PHP evaluates a file-scope `const` at the
                // point the declaration is reached.
                self.compile_expr(value);
                let name_ptr = Box::into_raw(Box::new(self.resolve_class_name(&name)));
                let idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                self.chunk.write_opcode(Opcode::DeclareConst);
                self.chunk.write_short(idx);
            }
            Stmt::Class {
                name,
                extends,
                methods,
                properties,
                implements,
                uses,
                trait_aliases,
                attributes,
                is_abstract,
                is_final,
                is_readonly,
            } => {
                let resolved_name = self.resolve_class_name(&name);
                let resolved_extends = extends
                    .as_ref()
                    .map(|parent| self.resolve_class_name(parent));
                let _class_id = self.class_registry.len();
                self.class_registry.push(resolved_name.clone());

                let old_cur_class = self.current_class.replace(resolved_name.clone());
                let old_cur_extends = self.compiling_class_extends.clone();
                self.compiling_class_extends = resolved_extends.clone();
                let old_compiling_consts = std::mem::take(&mut self.compiling_class_constants);

                let mut method_entries: Vec<(Stmt, Option<(HashMap<String, String>, String)>)> = methods
                    .into_iter()
                    .map(|m| (m, None))
                    .collect();
                let mut property_entries: Vec<(Stmt, Option<(HashMap<String, String>, String)>)> = properties
                    .into_iter()
                    .map(|p| (p, None))
                    .collect();

                // Trait flattening
                let direct_traits: Vec<String> = uses
                    .iter()
                    .map(|u| self.resolve_class_name(u))
                    .collect();
                let mut trait_queue: std::collections::VecDeque<String> = direct_traits.iter().cloned().collect();
                let mut processed_traits = std::collections::HashSet::new();
                let mut accumulated_trait_aliases = trait_aliases.clone();
                while let Some(resolved_trait) = trait_queue.pop_front() {
                    hyperion_core::hyp_debug!("DEBUG COMPILER: trait_queue popped {}", resolved_trait);
                    if processed_traits.contains(&resolved_trait) {
                        continue;
                    }
                    processed_traits.insert(resolved_trait.clone());

                    if !self.traits_registry.contains_key(&resolved_trait) {
                        self.try_compile_external_class(&resolved_trait);
                    }
                    if let Some(trait_def) = self.traits_registry.get(&resolved_trait).cloned() {
                        for a in &trait_def.trait_aliases {
                            accumulated_trait_aliases.push(a.clone());
                        }
                        for u in trait_def.uses {
                            trait_queue.push_back(u);
                        }
                        for t_m in trait_def.methods {
                            if let Stmt::Function { name: ref t_m_name, body: ref t_m_body, .. } = t_m {
                                let mut existing_idx = None;
                                for (idx, (c_m, _)) in method_entries.iter().enumerate() {
                                    if let Stmt::Function { name: ref c_m_name, .. } = c_m {
                                        if c_m_name == t_m_name {
                                            existing_idx = Some(idx);
                                            break;
                                        }
                                    }
                                }
                                if let Some(idx) = existing_idx {
                                    if let Stmt::Function { body: ref c_m_body, .. } = &method_entries[idx].0 {
                                        if c_m_body.is_empty() && !t_m_body.is_empty() {
                                            method_entries[idx] = (t_m.clone(), Some((trait_def.aliases.clone(), trait_def.namespace.clone())));
                                        }
                                    }
                                } else {
                                    method_entries.push((t_m.clone(), Some((trait_def.aliases.clone(), trait_def.namespace.clone()))));
                                }

                                // Check trait_aliases for method aliases
                                for (t_opt, orig_name, alias_name) in &accumulated_trait_aliases {
                                    let matches_trait = match t_opt {
                                        Some(tn) => {
                                            let r_tn = self.resolve_class_name(tn);
                                            r_tn.eq_ignore_ascii_case(&resolved_trait)
                                                || tn.eq_ignore_ascii_case(&resolved_trait)
                                                || resolved_trait.ends_with(tn)
                                        }
                                        None => true,
                                    };
                                    if matches_trait && orig_name.eq_ignore_ascii_case(t_m_name) {
                                        let mut alias_fn = t_m.clone();
                                        if let Stmt::Function { name: ref mut n, .. } = alias_fn {
                                            *n = alias_name.clone();
                                        }
                                        method_entries.push((alias_fn, Some((trait_def.aliases.clone(), trait_def.namespace.clone()))));
                                    }
                                }
                            }
                        }
                        for t_p in trait_def.properties {
                            // Traits contribute constants as well as properties,
                            // and both are matched by name so a member the class
                            // declares itself is not overwritten.
                            if let Some(t_p_name) = class_member_name(&t_p) {
                                let exists = property_entries
                                    .iter()
                                    .any(|(c_p, _)| class_member_name(c_p) == Some(t_p_name));
                                if !exists {
                                    property_entries.push((t_p, Some((trait_def.aliases.clone(), trait_def.namespace.clone()))));
                                }
                            }
                        }
                    } else {
                        hyperion_core::hyp_debug!("WARNING: Trait not found: {}", resolved_trait);
                    }
                }

                // Ensure any accumulated aliases whose target methods were inherited are added
                for (_t_opt, orig_name, alias_name) in &accumulated_trait_aliases {
                    let mut alias_to_add = None;
                    for (m, ctx) in &method_entries {
                        if let Stmt::Function { name: ref m_name, .. } = m {
                            if m_name.eq_ignore_ascii_case(orig_name) {
                                let mut alias_fn = m.clone();
                                if let Stmt::Function { name: ref mut n, .. } = alias_fn {
                                    *n = alias_name.clone();
                                }
                                alias_to_add = Some((alias_fn, ctx.clone()));
                                break;
                            }
                        }
                    }
                    if let Some(entry) = alias_to_add {
                        if !method_entries.iter().any(|(m, _)| match m {
                            Stmt::Function { name, .. } => name.eq_ignore_ascii_case(alias_name),
                            _ => false,
                        }) {
                            method_entries.push(entry);
                        }
                    }
                }

                // Interface validation
                let mut method_names = Vec::new();
                for (m, _) in &method_entries {
                    if let Stmt::Function { name: m_name, .. } = m {
                        method_names.push(m_name.clone());
                    }
                }
                for iface_name in &implements {
                    let resolved_iface = self.resolve_class_name(&iface_name);
                    if !self.interfaces_registry.contains_key(&resolved_iface) {
                        self.try_compile_external_class(&resolved_iface);
                    }
                    if let Some(iface_stmt) = self.interfaces_registry.get(&resolved_iface) {
                        if let Stmt::Interface { methods: iface_methods, .. } = iface_stmt {
                            for im in iface_methods {
                                if let Stmt::Function { name: im_name, .. } = im {
                                    if !method_names.contains(im_name) {
                                        // Some traits define abstract methods that are implemented
                                    }
                                }
                            }
                        }
                    }
                }

                let mut default_properties = Vec::new();
                let mut compiled_methods = Vec::new();
                for (method_stmt, trait_context) in method_entries {
                    if let Stmt::Function {
                        name: method_name,
                        params,
                        body,
                        is_static,
                        visibility,
                        attributes: method_attributes,
                    } = method_stmt
                    {
                        if method_name.eq_ignore_ascii_case("__construct") {
                            for param in &params {
                                if let Some(vis) = param.promoted_visibility {
                                    if !default_properties.iter().any(|(p, _, _)| p == &param.name) {
                                        let val = match &param.default_expr {
                                            Some(e) => expr_to_literal_value(e, self),
                                            None => Value::null(),
                                        };
                                        default_properties.push((param.name.clone(), val, vis));
                                    }
                                }
                            }
                        }
                        let mut compiled_method_attrs = Vec::new();
                        for attr in &method_attributes {
                            let mut resolved_args = Vec::new();
                            for arg in &attr.arguments {
                                match arg {
                                    Expr::NamedArgument { name, value } => {
                                        let val = expr_to_literal_value(value, self);
                                        resolved_args.push((Some(name.clone()), val));
                                    }
                                    _ => {
                                        let val = expr_to_literal_value(arg, self);
                                        resolved_args.push((None, val));
                                    }
                                }
                            }
                            let resolved_attr_name = self.resolve_class_name(&attr.name);
                            compiled_method_attrs.push(CompiledAttribute {
                                name: resolved_attr_name,
                                args: resolved_args,
                            });
                        }

                        let mut method_compiler = self.new_child();
                        method_compiler.current_class = Some(resolved_name.clone());
                        method_compiler.current_function = Some(method_name.clone());
                        if let Some((trait_aliases, trait_ns)) = trait_context {
                            method_compiler.aliases = trait_aliases;
                            method_compiler.current_namespace = trait_ns;
                        }

                        // Reserve slot 0 for $this
                        method_compiler.symbol_table.add_local("this");
                        for param in &params {
                            method_compiler.symbol_table.add_local(&param.name);
                        }

                        let is_generator = has_yield_stmt(&body);
                        if is_generator {
                            method_compiler.chunk.write_opcode(Opcode::CreateGenerator);
                        }

                        // Inject property promotion
                        for param in &params {
                            if param.promoted_visibility.is_some() {
                                method_compiler.compile_expr(Expr::Variable("this".to_string()));
                                let local_idx = method_compiler.resolve_local(&param.name);
                                method_compiler.chunk.write_opcode(Opcode::GetLocal);
                                method_compiler.chunk.write_byte(local_idx);
                                let prop_ptr = Box::into_raw(Box::new(param.name.clone()));
                                let prop_idx = method_compiler
                                    .chunk
                                    .add_constant(Value::new_string_ptr(prop_ptr as *mut ()));
                                method_compiler.chunk.write_opcode(Opcode::SetProperty);
                                method_compiler.chunk.write_short(prop_idx);
                                method_compiler.chunk.write_opcode(Opcode::Pop);
                            }
                        }

                        // Inject variadic packing if needed
                        let mut variadic_index = None;
                        for (i, p) in params.iter().enumerate() {
                            if p.is_variadic {
                                variadic_index = Some(i as u8);
                            }
                        }

                        if let Some(v_idx) = variadic_index {
                            method_compiler.chunk.write_opcode(Opcode::PackVariadic);
                            method_compiler.chunk.write_byte(v_idx);
                        }

                        if method_name == "getArrayableItems" {
                            hyperion_core::hyp_debug!("DEBUG AST for getArrayableItems: {:#?}", body);
                        }

                        for stmt in body {
                            method_compiler.compile_stmt(stmt);
                        }

                        method_compiler.chunk.write_opcode(Opcode::Constant);
                        let null_idx = method_compiler.chunk.add_constant(Value::null());
                        method_compiler.chunk.write_short(null_idx);
                        method_compiler.chunk.write_opcode(Opcode::ReturnValue);

                        let required_arity = params
                            .iter()
                            .filter(|p| !p.has_default && !p.is_variadic)
                            .count();
                        let compiled_params = params
                            .into_iter()
                            .map(|p| method_compiler.compile_param(p))
                            .collect::<Vec<_>>();

                        let num_locals = method_compiler.symbol_table.locals_count() as u8;
                        method_compiler.chunk.local_names = method_compiler
                            .symbol_table
                            .locals
                            .iter()
                            .map(|l| l.name.clone())
                            .collect();
                        method_compiler.chunk.code[1] = num_locals;

                        if name.ends_with("Translator") {
                            hyperion_core::hyp_debug!("DEBUG COMPILER: Class {} compiled method: {}", name, method_name);
                        }
                        compiled_methods.push(CompiledFunction {
                            name: method_name.clone(),
                            arity: required_arity,
                            params: compiled_params,
                            chunk: method_compiler.chunk,
                            is_closure: false,
                            is_static,
                            visibility,
                            num_locals: num_locals as usize,
                            attributes: compiled_method_attrs,
                        });
                        self.functions.extend(method_compiler.functions);
                        self.classes.extend(method_compiler.classes);
                    }
                }

                let mut static_properties = Vec::new();
                // PASS 1: Evaluate class constants first so they are available for property default values
                for (prop_stmt, trait_context) in &property_entries {
                    let old_aliases = self.aliases.clone();
                    let old_ns = self.current_namespace.clone();
                    if let Some((trait_aliases, trait_ns)) = trait_context {
                        self.aliases = trait_aliases.clone();
                        self.current_namespace = trait_ns.clone();
                    }

                    if let Stmt::ConstDeclaration {
                        name,
                        value,
                        visibility,
                    } = prop_stmt
                    {
                        let val = expr_to_literal_value(value, self);
                        static_properties.push((name.clone(), val, *visibility));
                        self.compiling_class_constants.push((name.clone(), val));
                    }

                    self.aliases = old_aliases;
                    self.current_namespace = old_ns;
                }

                // Constants declared on an implemented interface are visible on
                // the class (`J::IC` where `interface I { const IC = 3; }`).
                // Static-property lookup walks `extends` at runtime but not the
                // interface list, so they are copied down here. A constant the
                // class declares itself wins, which is why this runs second.
                for iface_name in &implements {
                    let resolved_iface = self.resolve_class_name(iface_name);
                    let iface_members = match self.interfaces_registry.get(&resolved_iface) {
                        Some(Stmt::Interface { methods: m, .. }) => m.clone(),
                        _ => continue,
                    };
                    for member in &iface_members {
                        if let Stmt::ConstDeclaration {
                            name,
                            value,
                            visibility,
                        } = member
                        {
                            if static_properties.iter().any(|(n, _, _)| n == name) {
                                continue;
                            }
                            let val = expr_to_literal_value(value, self);
                            static_properties.push((name.clone(), val, *visibility));
                            self.compiling_class_constants.push((name.clone(), val));
                        }
                    }
                }

                // PASS 2: Evaluate property declarations (which can now reference self::CONST / static::CONST)
                for (prop_stmt, trait_context) in &property_entries {
                    let old_aliases = self.aliases.clone();
                    let old_ns = self.current_namespace.clone();
                    if let Some((trait_aliases, trait_ns)) = trait_context {
                        self.aliases = trait_aliases.clone();
                        self.current_namespace = trait_ns.clone();
                    }

                    if let Stmt::PropertyDeclaration {
                        name,
                        initial_value,
                        is_static,
                        visibility,
                        is_readonly: _,
                    } = prop_stmt
                    {
                        let val = match initial_value {
                            Some(e) => expr_to_literal_value(e, self),
                            None => Value::null(),
                        };
                        if *is_static {
                            static_properties.push((name.clone(), val, *visibility));
                        } else {
                            default_properties.push((name.clone(), val, *visibility));
                        }
                    }

                    self.aliases = old_aliases;
                    self.current_namespace = old_ns;
                }

                let resolved_implements = implements
                    .iter()
                    .map(|i| self.resolve_class_name(i))
                    .collect();

                if resolved_name == "Illuminate\\Foundation\\Application" || resolved_name == "Application" {
                    hyperion_core::hyp_debug!("DEBUG COMPILER: Application extends: {:?}", resolved_extends);
                }
                let mut compiled_attrs = Vec::new();
                for attr in &attributes {
                    let mut resolved_args = Vec::new();
                    for arg in &attr.arguments {
                        match arg {
                            Expr::NamedArgument { name, value } => {
                                let val = expr_to_literal_value(value, self);
                                resolved_args.push((Some(name.clone()), val));
                            }
                            _ => {
                                let val = expr_to_literal_value(arg, self);
                                resolved_args.push((None, val));
                            }
                        }
                    }
                    let resolved_attr_name = self.resolve_class_name(&attr.name);
                    compiled_attrs.push(CompiledAttribute {
                        name: resolved_attr_name,
                        args: resolved_args,
                    });
                }

                self.classes.push(CompiledClass {
                    name: resolved_name,
                    extends: resolved_extends,
                    implements: resolved_implements,
                    traits: direct_traits,
                    methods: compiled_methods,
                    static_properties,
                    default_properties,
                    attributes: compiled_attrs,
                    is_interface: false,
                    is_trait: false,
                    is_enum: false,
                    is_abstract,
                    is_final,
                    is_readonly,
                });

                self.current_class = old_cur_class;
                self.compiling_class_extends = old_cur_extends;
                self.compiling_class_constants = old_compiling_consts;
            }
            Stmt::Enum {
                name,
                backed_type,
                cases,
                implements,
                methods,
            } => {
                let resolved_name = self.resolve_class_name(&name);
                self.class_registry.push(resolved_name.clone());

                let extends_class = if backed_type.is_some() {
                    Some("BackedEnum".to_string())
                } else {
                    Some("UnitEnum".to_string())
                };

                let mut compiled_methods = Vec::new();
                let mut enum_constants = Vec::new();
                for method_stmt in methods {
                    match method_stmt {
                        Stmt::Function {
                            name: method_name,
                            params,
                            body,
                            is_static,
                            visibility,
                            ..
                        } => {
                            let mut method_compiler = self.new_child();
                            method_compiler.current_class = Some(resolved_name.clone());
                            method_compiler.current_function = Some(method_name.clone());

                            // Reserve slot 0 for $this
                            method_compiler.symbol_table.add_local("this");
                            for param in &params {
                                method_compiler.symbol_table.add_local(&param.name);
                            }

                            let is_generator = has_yield_stmt(&body);
                            if is_generator {
                                method_compiler.chunk.write_opcode(Opcode::CreateGenerator);
                            }

                            // Inject variadic packing if needed
                            let mut variadic_index = None;
                            for (i, p) in params.iter().enumerate() {
                                if p.is_variadic {
                                    variadic_index = Some(i as u8);
                                }
                            }

                            if let Some(v_idx) = variadic_index {
                                method_compiler.chunk.write_opcode(Opcode::PackVariadic);
                                method_compiler.chunk.write_byte(v_idx);
                            }

                            for stmt in body {
                                method_compiler.compile_stmt(stmt.clone());
                            }

                            method_compiler.chunk.write_opcode(Opcode::Constant);
                            let null_idx = method_compiler.chunk.add_constant(Value::null());
                            method_compiler.chunk.write_short(null_idx);
                            method_compiler.chunk.write_opcode(Opcode::ReturnValue);

                            let required_arity = params
                                .iter()
                                .filter(|p| !p.has_default && !p.is_variadic)
                                .count();
                            let compiled_params = params
                                .into_iter()
                                .map(|p| method_compiler.compile_param(p))
                                .collect::<Vec<_>>();

                            let num_locals = method_compiler.symbol_table.locals_count() as u8;
                            method_compiler.chunk.local_names = method_compiler
                                .symbol_table
                                .locals
                                .iter()
                                .map(|l| l.name.clone())
                                .collect();
                            method_compiler.chunk.code[1] = num_locals;

                            compiled_methods.push(CompiledFunction {
                                name: method_name.clone(),
                                arity: required_arity,
                                params: compiled_params,
                                chunk: method_compiler.chunk,
                                is_closure: false,
                                is_static,
                                visibility,
                                num_locals: num_locals as usize,
                                attributes: Vec::new(),
                            });
                            self.functions.extend(method_compiler.functions);
                            self.classes.extend(method_compiler.classes);
                        }
                        Stmt::ConstDeclaration {
                            name,
                            value,
                            visibility,
                        } => {
                            let val = expr_to_literal_value(&value, self);
                            enum_constants.push((name, value, val, visibility));
                        }
                        _ => {}
                    }
                }

                let resolved_implements: Vec<String> = implements
                    .into_iter()
                    .map(|i| self.resolve_class_name(&i))
                    .collect();

                let class_id = self.class_registry.len() - 1;
                let mut static_props = Vec::new();
                for (case_name, value_expr) in &cases {
                    let mut obj = hyperion_core::types::object::PhpObject::new_with_name(0, resolved_name.clone());
                    let name_val = {
                        let ptr = Box::into_raw(Box::new(case_name.clone()));
                        Value::new_string_ptr(ptr as *mut ())
                    };
                    obj.properties.insert("name".to_string(), name_val);
                    if let Some(val_expr) = value_expr {
                        let val = expr_to_literal_value(val_expr, self);
                        obj.properties.insert("value".to_string(), val);
                    }
                    let obj_ptr = Box::into_raw(Box::new(obj));
                    let enum_case_val = Value::new_object_ptr(obj_ptr as *mut ());
                    static_props.push((case_name.clone(), enum_case_val, hyperion_parser::parser::ast::Visibility::Public));
                }

                for (name, _, val, visibility) in &enum_constants {
                    static_props.push((name.clone(), *val, *visibility));
                }

                self.classes.push(CompiledClass {
                    name: resolved_name.clone(),
                    extends: extends_class,
                    implements: resolved_implements,
                    traits: Vec::new(),
                    methods: compiled_methods,
                    static_properties: static_props,
                    default_properties: Vec::new(),
                    attributes: Vec::new(),
                    is_interface: false,
                    is_trait: false,
                    is_enum: true,
                    is_abstract: false,
                    is_final: true,
                    is_readonly: false,
                });

                for (case_name, value_expr) in cases {
                    let class_name_ptr = Box::into_raw(Box::new(resolved_name.clone()));
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(class_name_ptr as *mut ()));

                    self.chunk.write_opcode(Opcode::New);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_opcode(Opcode::Dup);
                    self.chunk.write_opcode(Opcode::CallConstruct);
                    self.chunk.write_byte(0);
                    self.chunk.write_byte(0);
                    self.chunk.write_opcode(Opcode::Pop); // Pop constructor return value

                    self.chunk.write_opcode(Opcode::Dup);
                    let name_prop_ptr = Box::into_raw(Box::new("name".to_string()));
                    let name_prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_prop_ptr as *mut ()));

                    let case_name_ptr = Box::into_raw(Box::new(case_name.clone()));
                    let case_name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(case_name_ptr as *mut ()));

                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(case_name_idx);

                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(name_prop_idx);
                    self.chunk.write_opcode(Opcode::Pop);

                    if let Some(val_expr) = value_expr {
                        self.chunk.write_opcode(Opcode::Dup);

                        let val_prop_ptr = Box::into_raw(Box::new("value".to_string()));
                        let val_prop_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(val_prop_ptr as *mut ()));

                        self.compile_expr(val_expr);

                        self.chunk.write_opcode(Opcode::SetProperty);
                        self.chunk.write_short(val_prop_idx);
                        self.chunk.write_opcode(Opcode::Pop);
                    }

                    let case_static_ptr = Box::into_raw(Box::new(case_name.clone()));
                    let case_static_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(case_static_ptr as *mut ()));

                    self.chunk.write_opcode(Opcode::SetStatic);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(case_static_idx);
                    self.chunk.write_opcode(Opcode::Pop);
                }

                for (name, value, _, _) in enum_constants {
                    let class_name_ptr = Box::into_raw(Box::new(resolved_name.clone()));
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(class_name_ptr as *mut ()));
                    let const_name_ptr = Box::into_raw(Box::new(name));
                    let const_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(const_name_ptr as *mut ()));

                    self.compile_expr(value);
                    self.chunk.write_opcode(Opcode::SetStatic);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(const_idx);
                    self.chunk.write_opcode(Opcode::Pop);
                }
            }
            Stmt::Break(num) => {
                let jump = self.emit_jump(Opcode::Jump);
                let level = num.unwrap_or(1) as usize;
                if level > 0 && self.break_targets.len() >= level {
                    let idx = self.break_targets.len() - level;
                    self.break_targets[idx].push(jump);
                } else if let Some(targets) = self.break_targets.last_mut() {
                    targets.push(jump);
                }
            }
            Stmt::Continue(num) => {
                let jump = self.emit_jump(Opcode::Jump);
                let level = num.unwrap_or(1) as usize;
                if level > 0 && self.continue_targets.len() >= level {
                    let idx = self.continue_targets.len() - level;
                    self.continue_targets[idx].push(jump);
                } else if let Some(targets) = self.continue_targets.last_mut() {
                    targets.push(jump);
                }
            }
            Stmt::TryCatch {
                try_body,
                catches,
                finally_body,
            } => {
                let start_ip = self.chunk.code.len();

                for stmt in try_body {
                    self.compile_stmt(stmt);
                }

                let end_try_jump = self.emit_jump(Opcode::Jump);
                let end_ip = self.chunk.code.len(); // Right before catch blocks begin

                let mut catch_end_jumps = Vec::new();

                let stack_depth = self.foreach_depth * 2;
                for catch in catches {
                    let catch_ip = self.chunk.code.len();

                    let catch_var_name = catch.var.clone().unwrap_or_default();
                    for catch_type in &catch.types {
                        let resolved_catch_class = self.resolve_class_name(catch_type);
                        self.chunk
                            .exception_handlers
                            .push(hyperion_bytecode::ExceptionHandler {
                                start_ip,
                                end_ip,
                                catch_ip,
                                catch_var: catch_var_name.clone(),
                                catch_class: resolved_catch_class,
                                stack_depth,
                            });
                    }

                    if let Some(var) = &catch.var {
                        let catch_var_id = self.resolve_local(var);
                        self.chunk.write_opcode(Opcode::SetLocal);
                        self.chunk.write_byte(catch_var_id);
                        self.chunk.write_opcode(Opcode::Pop);
                    } else {
                        self.chunk.write_opcode(Opcode::Pop);
                    }

                    for stmt in catch.body {
                        self.compile_stmt(stmt);
                    }

                    catch_end_jumps.push(self.emit_jump(Opcode::Jump));
                }

                let finally_ip = self.chunk.code.len();

                if finally_body.is_some() {
                    self.chunk
                        .finally_handlers
                        .push(hyperion_bytecode::FinallyHandler {
                            start_ip,
                            end_ip: finally_ip, // Cover both try and catch blocks
                            finally_ip,
                            stack_depth,
                        });
                }

                self.patch_jump(end_try_jump);
                for jump in catch_end_jumps {
                    self.patch_jump(jump);
                }

                if let Some(finally_stmts) = finally_body {
                    for stmt in finally_stmts {
                        self.compile_stmt(stmt);
                    }
                    self.chunk.write_opcode(Opcode::EndFinally);
                }
            }

            Stmt::Throw(expr) => {
                self.compile_expr(expr);
                self.chunk.write_opcode(Opcode::Throw);
            }
            Stmt::Switch {
                condition,
                cases,
                default,
            } => {
                // Check if all cases are literal ints/strings for O(1) jump table
                let mut is_static_table = true;
                for (case_expr, _) in &cases {
                    match case_expr {
                        Expr::LiteralInt(_) => {}
                        _ => {
                            is_static_table = false;
                            break;
                        }
                    }
                }

                if is_static_table {
                    self.compile_expr(condition);
                    let mut jump_table = std::collections::HashMap::new();

                    // We need to write a placeholder SwitchTable instruction
                    self.chunk.write_opcode(Opcode::SwitchTable);
                    let table_idx_offset = self.chunk.code.len();
                    self.chunk.write_byte(0); // table index placeholder
                    let default_jump_offset = self.chunk.code.len();
                    self.chunk.write_short(0xffff); // To be patched

                    self.break_targets.push(Vec::new());

                    let mut case_offsets = Vec::new();

                    for (case_expr, body) in cases {
                        let offset = self.chunk.code.len();
                        case_offsets.push(offset as u16);
                        let val = match case_expr {
                            Expr::LiteralInt(i) => Value::new_int(i as i32),
                            // Note: String keys in static tables are not properly nanboxed yet in compile time
                            // We will use standard Jump for strings for now, only Int is fully safe to nanbox inline without GC
                            _ => unreachable!(),
                        };
                        let rel_offset = offset - (default_jump_offset + 2);
                        jump_table.insert(val, rel_offset as u16);

                        for stmt in body {
                            self.compile_stmt(stmt);
                        }
                    }

                    let _default_offset = self.chunk.code.len();
                    self.patch_jump(default_jump_offset); // Point default to here

                    if let Some(def_body) = default {
                        for stmt in def_body {
                            self.compile_stmt(stmt);
                        }
                    }

                    let table_idx = self.chunk.add_jump_table(jump_table);
                    self.chunk.code[table_idx_offset] = table_idx;

                    let breaks = self.break_targets.pop().unwrap();
                    for b in breaks {
                        self.patch_jump(b);
                    }
                } else {
                    // True PHP switch compilation supporting fallthrough
                    self.break_targets.push(Vec::new());

                    let mut case_body_jumps = Vec::new();

                    // Step 1: Comparison preamble
                    for (case_expr, _) in &cases {
                        self.compile_expr(condition.clone());
                        self.compile_expr(case_expr.clone());
                        self.chunk.write_opcode(Opcode::Equals);
                        self.chunk.write_opcode(Opcode::Not);
                        let jump_to_body = self.emit_jump(Opcode::JumpIfFalse);
                        case_body_jumps.push(jump_to_body);
                    }

                    // If no cases matched, jump to default or end
                    let default_jump = self.emit_jump(Opcode::Jump);

                    // Step 2: Case bodies in sequence (natural fallthrough)
                    for (i, (_, body)) in cases.into_iter().enumerate() {
                        self.patch_jump(case_body_jumps[i]);
                        for stmt in body {
                            self.compile_stmt(stmt);
                        }
                    }

                    // Step 3: Default body
                    self.patch_jump(default_jump);
                    if let Some(def_body) = default {
                        for stmt in def_body {
                            self.compile_stmt(stmt);
                        }
                    }

                    // Step 4: Patch all breaks to end of switch
                    let breaks = self.break_targets.pop().unwrap();
                    for b in breaks {
                        self.patch_jump(b);
                    }
                }
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s);
                }
            }
            Stmt::DoWhile { body, condition } => {
                let loop_start = self.chunk.code.len();

                self.break_targets.push(Vec::new());
                self.continue_targets.push(Vec::new());

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                let continues = self.continue_targets.pop().unwrap();
                for c in continues {
                    self.patch_jump(c);
                }

                self.compile_expr(condition);
                let exit_jump = self.emit_jump(Opcode::JumpIfFalse);
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);

                let breaks = self.break_targets.pop().unwrap();
                for b in breaks {
                    self.patch_jump(b);
                }
            }
            Stmt::GlobalVar(vars) => {
                for name in vars {
                    let local_idx = self.resolve_local(&name);
                    self.compile_make_ref(Expr::ArrayGet {
                        array: Box::new(Expr::Variable("GLOBALS".to_string())),
                        key: Some(Box::new(Expr::LiteralString(name))),
                    });
                    self.chunk.write_opcode(Opcode::BindRefLocal);
                    self.chunk.write_byte(local_idx);
                    self.chunk.write_opcode(Opcode::Pop);
                }
            }
            Stmt::Unset(vars) => {
                for expr in vars {
                    match expr {
                        Expr::Variable(name) => {
                            let null_idx = self.chunk.add_constant(Value::null());
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(null_idx);
                            let local_idx = self.resolve_local(&name);
                            self.chunk.write_opcode(Opcode::SetLocal);
                            self.chunk.write_byte(local_idx);
                            self.chunk.write_opcode(Opcode::Pop);
                        }
                        Expr::VariableVariable(name_expr) => {
                            self.compile_expr(*name_expr);
                            let null_idx = self.chunk.add_constant(Value::null());
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(null_idx);
                            self.chunk.write_opcode(Opcode::SetLocalDynamic);
                            self.chunk.write_opcode(Opcode::Pop);
                        }
                        Expr::ArrayGet { array, key } => {
                            self.compile_expr(*array);
                            if let Some(k) = key {
                                self.compile_expr(*k);
                            } else {
                                let null_idx = self.chunk.add_constant(Value::null());
                                self.chunk.write_opcode(Opcode::Constant);
                                self.chunk.write_short(null_idx);
                            }
                            self.chunk.write_opcode(Opcode::ArrayDelete);
                        }
                        Expr::PropertyGet { object, property } => {
                            self.compile_expr(*object);
                            // A bare `$o->name` parses the name as an identifier;
                            // emit it as the string the VM keys properties by.
                            // `$o->$n` is a genuine expression and compiles as one.
                            match *property {
                                Expr::Identifier(name) => {
                                    let name_ptr = Box::into_raw(Box::new(name));
                                    let idx = self
                                        .chunk
                                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                                    self.chunk.write_opcode(Opcode::Constant);
                                    self.chunk.write_short(idx);
                                }
                                other => self.compile_expr(other),
                            }
                            self.chunk.write_opcode(Opcode::PropertyDelete);
                        }
                        _ => {}
                    }
                }
            }
            Stmt::StaticVar(vars) => {
                for (name, init_expr_opt) in vars {
                    let local_idx = self.resolve_local(&name);
                    let name_str = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self.chunk.add_constant(Value::new_string_ptr(name_str as *mut ()));

                    self.chunk.write_opcode(Opcode::GetStaticVar);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_byte(local_idx);
                    let jump_pos = self.chunk.code.len();
                    self.chunk.write_short(0xffff);

                    if let Some(init_expr) = init_expr_opt {
                        self.compile_expr(init_expr);
                    } else {
                        self.chunk.write_opcode(Opcode::Constant);
                        let null_idx = self.chunk.add_constant(Value::null());
                        self.chunk.write_short(null_idx);
                    }

                    self.chunk.write_opcode(Opcode::InitStaticVar);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_byte(local_idx);

                    let target = self.chunk.code.len();
                    let offset = target - (jump_pos + 2);
                    self.chunk.code[jump_pos] = ((offset >> 8) & 0xff) as u8;
                    self.chunk.code[jump_pos + 1] = (offset & 0xff) as u8;
                }
            }
            _ => todo!("Task 3: Compiler missing feature"),
        }
    }

    fn compile_expr_quiet(&mut self, expr: Expr) {
        match expr {
            Expr::Throw(_) => {
                // Should never happen quietly
            }
            Expr::Variable(name) => {
                if let Some(id) = Self::get_superglobal_id(&name) {
                    self.chunk.write_opcode(Opcode::GetSuperglobal);
                    self.chunk.write_byte(id);
                } else {
                    let local_idx = self.resolve_local(&name);
                    self.chunk.write_opcode(Opcode::GetLocalQuiet);
                    self.chunk.write_byte(local_idx);
                }
            }
            _ => self.compile_expr(expr),
        }
    }

    fn compile_isset_expr(&mut self, expr: Expr) {
        match expr {
            Expr::ArrayGet { array, key } => {
                self.compile_expr_quiet(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::ArrayIsset);
            }
            Expr::PropertyGet { object, property } => {
                self.compile_expr_quiet(*object);
                if let Expr::Identifier(ref name) = *property {
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::PropertyIsset);
                    self.chunk.write_short(name_idx);
                } else if let Expr::LiteralString(ref name) = *property {
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::PropertyIsset);
                    self.chunk.write_short(name_idx);
                } else {
                    self.compile_expr(*property);
                    self.chunk.write_opcode(Opcode::PropertyIssetDynamic);
                }
            }
            other => {
                self.compile_expr_quiet(other);
                let null_idx = self.chunk.add_constant(Value::null());
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(null_idx);
                self.chunk.write_opcode(Opcode::StrictNotEquals);
            }
        }
    }

    fn emit_binary_op_for_token(&mut self, operator: Token) {
        let op = match operator {
            Token::PlusAssign | Token::Plus => Opcode::Add,
            Token::MinusAssign | Token::Minus => Opcode::Subtract,
            Token::MultiplyAssign | Token::Multiply => Opcode::Multiply,
            Token::DivideAssign | Token::Divide => Opcode::Divide,
            Token::ModuloAssign | Token::Modulo => Opcode::Modulo,
            Token::DotAssign | Token::Dot => Opcode::Concat,
            Token::NullCoalesceAssign => Opcode::LogicalOr, // Simplification
            Token::BitwiseOrAssign | Token::Pipe => Opcode::BitwiseOr,
            Token::BitwiseAndAssign | Token::Ampersand => Opcode::BitwiseAnd,
            Token::BitwiseXorAssign | Token::BitwiseXor => Opcode::BitwiseXor,
            Token::ShiftLeftAssign | Token::ShiftLeft => Opcode::ShiftLeft,
            Token::ShiftRightAssign | Token::ShiftRight => Opcode::ShiftRight,
            Token::PowerAssign | Token::Power => Opcode::Power,
            _ => unimplemented!("CompoundAssign op {:?}", operator),
        };
        self.chunk.write_opcode(op);
    }

    fn compile_compound_assign(&mut self, target: Expr, operator: Token, value: Expr) {
        let is_null_coalesce = operator == Token::NullCoalesceAssign;
        match target {
            Expr::Variable(name) => {
                let local_idx = self.resolve_local(&name);
                self.chunk.write_opcode(Opcode::GetLocalQuiet);
                self.chunk.write_byte(local_idx);
                
                if is_null_coalesce {
                    let jump_not_null = self.emit_jump(Opcode::JumpIfNotNull);
                    self.chunk.write_opcode(Opcode::Pop); // pop the null
                    self.compile_expr(value);
                    self.chunk.write_opcode(Opcode::SetLocal);
                    self.chunk.write_byte(local_idx);
                    self.patch_jump(jump_not_null);
                } else {
                    self.compile_expr(value);
                    self.emit_binary_op_for_token(operator);
                    self.chunk.write_opcode(Opcode::SetLocal);
                    self.chunk.write_byte(local_idx);
                }
            }
            Expr::VariableVariable(var_name_expr) => {
                self.compile_expr(*var_name_expr.clone());
                self.chunk.write_opcode(Opcode::Dup);
                self.chunk.write_opcode(Opcode::GetLocalDynamic);

                if is_null_coalesce {
                    let jump_not_null = self.emit_jump(Opcode::JumpIfNotNull);
                    self.chunk.write_opcode(Opcode::Pop); // pop the null
                    self.compile_expr(value);
                    self.chunk.write_opcode(Opcode::SetLocalDynamic);
                    let jump_end = self.emit_jump(Opcode::Jump);
                    self.patch_jump(jump_not_null);
                    self.chunk.write_opcode(Opcode::Swap);
                    self.chunk.write_opcode(Opcode::Pop); // pop var_name
                    self.patch_jump(jump_end);
                } else {
                    self.compile_expr(value);
                    self.emit_binary_op_for_token(operator);
                    self.chunk.write_opcode(Opcode::SetLocalDynamic);
                }
            }
            Expr::PropertyGet { object, property } => {
                self.compile_expr(*object);
                self.chunk.write_opcode(Opcode::Dup);

                let name_idx = if let Expr::Identifier(n) = *property {
                    let name_ptr = Box::into_raw(Box::new(n));
                    self.chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()))
                } else {
                    let name_ptr = Box::into_raw(Box::new("unsupported".to_string()));
                    self.chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()))
                };

                self.chunk.write_opcode(Opcode::GetProperty);
                self.chunk.write_short(name_idx);

                if is_null_coalesce {
                    let jump_is_null = self.emit_jump(Opcode::JumpIfNull);
                    self.chunk.write_opcode(Opcode::Swap);
                    self.chunk.write_opcode(Opcode::Pop);
                    let jump_end = self.emit_jump(Opcode::Jump);

                    self.patch_jump(jump_is_null);
                    self.chunk.write_opcode(Opcode::Pop); // pop the null
                    self.compile_expr(value);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(name_idx);
                    self.patch_jump(jump_end);
                } else {
                    self.compile_expr(value);
                    self.emit_binary_op_for_token(operator);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(name_idx);
                }
            }
            Expr::ArrayGet { array, key } => {
                self.compile_expr(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::Dup2); // stack: [arr, key, arr, key]
                self.chunk.write_opcode(Opcode::ArrayGet); // stack: [arr, key, old_val]
                
                if is_null_coalesce {
                    let jump_is_null = self.emit_jump(Opcode::JumpIfNull);
                    self.chunk.write_opcode(Opcode::Swap);
                    self.chunk.write_opcode(Opcode::Pop);
                    self.chunk.write_opcode(Opcode::Swap);
                    self.chunk.write_opcode(Opcode::Pop);
                    let jump_end = self.emit_jump(Opcode::Jump);

                    self.patch_jump(jump_is_null);
                    self.chunk.write_opcode(Opcode::Pop); // pop the null
                    self.compile_expr(value);
                    self.chunk.write_opcode(Opcode::ArraySet);
                    self.patch_jump(jump_end);
                } else {
                    self.compile_expr(value); // stack: [arr, key, old_val, rhs]
                    self.emit_binary_op_for_token(operator); // stack: [arr, key, new_val]
                    self.chunk.write_opcode(Opcode::ArraySet); // stack: [new_val]
                }
            }
            Expr::StaticPropertyGet { class_name, property } => {
                if class_name == "static" {
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property.clone())) as *mut ()
                        ));
                    
                    self.chunk.write_opcode(Opcode::LateStaticPropertyGet);
                    self.chunk.write_short(prop_idx);

                    if is_null_coalesce {
                        let jump_not_null = self.emit_jump(Opcode::JumpIfNotNull);
                        self.chunk.write_opcode(Opcode::Pop); // pop the null

                        self.compile_expr(value);

                        self.chunk.write_opcode(Opcode::LateStaticPropertySet);
                        self.chunk.write_short(prop_idx);

                        self.patch_jump(jump_not_null);
                    } else {
                        self.compile_expr(value);
                        self.emit_binary_op_for_token(operator);

                        self.chunk.write_opcode(Opcode::LateStaticPropertySet);
                        self.chunk.write_short(prop_idx);
                    }
                } else {
                    let resolved_class = self.resolve_class_name(&class_name);
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(resolved_class)) as *mut (),
                        ));
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property.clone())) as *mut ()
                        ));
                        
                    self.chunk.write_opcode(Opcode::GetStatic);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(prop_idx);

                    if is_null_coalesce {
                        let jump_not_null = self.emit_jump(Opcode::JumpIfNotNull);
                        self.chunk.write_opcode(Opcode::Pop); // pop the null

                        self.compile_expr(value);

                        self.chunk.write_opcode(Opcode::SetStatic);
                        self.chunk.write_short(class_idx);
                        self.chunk.write_short(prop_idx);

                        self.patch_jump(jump_not_null);
                    } else {
                        self.compile_expr(value);
                        self.emit_binary_op_for_token(operator);

                        self.chunk.write_opcode(Opcode::SetStatic);
                        self.chunk.write_short(class_idx);
                        self.chunk.write_short(prop_idx);
                    }
                }
            }
            _ => unimplemented!("Compound assignment to {:?}", target),
        }
    }

    fn compile_inc_dec(&mut self, target: Expr, is_pre: bool, op: Opcode) {
        match target {
            Expr::Variable(name) => {
                let local_idx = self.resolve_local(&name);
                self.chunk.write_opcode(Opcode::GetLocalQuiet);
                self.chunk.write_byte(local_idx);
                if !is_pre {
                    self.chunk.write_opcode(Opcode::Dup); // push original value
                }
                self.chunk.write_opcode(op);
                if is_pre {
                    self.chunk.write_opcode(Opcode::Dup); // push new value
                }
                self.chunk.write_opcode(Opcode::SetLocal);
                self.chunk.write_byte(local_idx);
                self.chunk.write_opcode(Opcode::Pop); // Pop the set local return value
            }
            Expr::PropertyGet { object, property } => {
                self.compile_expr(*object);
                self.chunk.write_opcode(Opcode::Dup);

                let name_idx = if let Expr::Identifier(n) = *property {
                    let name_ptr = Box::into_raw(Box::new(n));
                    self.chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()))
                } else {
                    let name_ptr = Box::into_raw(Box::new("unsupported".to_string()));
                    self.chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()))
                };

                self.chunk.write_opcode(Opcode::GetProperty);
                self.chunk.write_short(name_idx);

                if !is_pre {
                    let tmp_local = self.resolve_local("#incdec_tmp");
                    self.chunk.write_opcode(Opcode::Dup);
                    self.chunk.write_opcode(Opcode::SetLocal);
                    self.chunk.write_byte(tmp_local);
                    self.chunk.write_opcode(Opcode::Pop);
                }

                self.chunk.write_opcode(op);
                self.chunk.write_opcode(Opcode::SetProperty);
                self.chunk.write_short(name_idx);

                if !is_pre {
                    let tmp_local = self.resolve_local("#incdec_tmp");
                    self.chunk.write_opcode(Opcode::Pop);
                    self.chunk.write_opcode(Opcode::GetLocalQuiet);
                    self.chunk.write_byte(tmp_local);
                }
            }
            Expr::ArrayGet { array, key } => {
                self.compile_expr(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::Dup2);
                self.chunk.write_opcode(Opcode::ArrayGet);

                if !is_pre {
                    let tmp_local = self.resolve_local("#incdec_tmp");
                    self.chunk.write_opcode(Opcode::Dup);
                    self.chunk.write_opcode(Opcode::SetLocal);
                    self.chunk.write_byte(tmp_local);
                    self.chunk.write_opcode(Opcode::Pop);
                }

                self.chunk.write_opcode(op);
                self.chunk.write_opcode(Opcode::ArraySet);

                if !is_pre {
                    let tmp_local = self.resolve_local("#incdec_tmp");
                    self.chunk.write_opcode(Opcode::Pop);
                    self.chunk.write_opcode(Opcode::GetLocalQuiet);
                    self.chunk.write_byte(tmp_local);
                }
            }
            _ => {
                hyperion_core::hyp_debug!("Unsupported Inc/Dec target: {:?}", target);
            }
        }
    }

    fn has_nullsafe(expr: &Expr) -> bool {
        match expr {
            Expr::NullsafePropertyGet { .. }
            | Expr::NullsafeMethodCall { .. }
            | Expr::DynamicNullsafeMethodCall { .. } => true,
            Expr::MethodCall { object, .. }
            | Expr::DynamicMethodCall { object, .. }
            | Expr::PropertyGet { object, .. } => Self::has_nullsafe(object),
            Expr::ArrayGet { array, .. } => Self::has_nullsafe(array),
            _ => false,
        }
    }

    pub fn compile_expr(&mut self, expr: Expr) {
        if !self.in_nullsafe_chain && Self::has_nullsafe(&expr) {
            self.in_nullsafe_chain = true;
            let jump_start = self.nullsafe_exit_jumps.len();
            self.compile_expr_inner(expr);
            let jumps: Vec<usize> = self.nullsafe_exit_jumps.drain(jump_start..).collect();
            for jump in jumps {
                self.patch_jump(jump);
            }
            self.in_nullsafe_chain = false;
        } else {
            self.compile_expr_inner(expr);
        }
    }

    fn compile_expr_inner(&mut self, expr: Expr) {
        match expr {
            Expr::Throw(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::Throw);
            }
            Expr::LiteralInt(i) => {
                let val = if i >= (i32::MIN as i64) && i <= (i32::MAX as i64) {
                    Value::new_int(i as i32)
                } else {
                    Value::new_float(i as f64)
                };
                let idx = self.chunk.add_constant(val);
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }
            Expr::LiteralFloat(f) => {
                let idx = self.chunk.add_constant(Value::new_float(f));
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }
            Expr::Variable(name) => {
                if let Some(id) = Self::get_superglobal_id(&name) {
                    self.chunk.write_opcode(Opcode::GetSuperglobal);
                    self.chunk.write_byte(id);
                } else {
                    let local_idx = self.resolve_local(&name);
                    self.chunk.write_opcode(Opcode::GetLocal);
                    self.chunk.write_byte(local_idx);
                }
            }
            Expr::CastInt(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastInt);
            }
            Expr::CastString(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastString);
            }
            Expr::CastFloat(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastFloat);
            }
            Expr::CastBool(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastBool);
            }
            Expr::CastArray(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastArray);
            }
            Expr::CastObject(inner) => {
                self.compile_expr(*inner);
                self.chunk.write_opcode(Opcode::CastObject);
            }
            Expr::InterpolatedString(parts) => {
                if parts.is_empty() {
                    let idx = self.chunk.add_constant(Value::new_string_ptr(
                        Box::into_raw(Box::new("".to_string())) as *mut ()
                    ));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else {
                    self.compile_expr(parts[0].clone());
                    for part in parts.into_iter().skip(1) {
                        self.compile_expr(part);
                        self.chunk.write_opcode(Opcode::Concat);
                    }
                }
            }
            Expr::VariableVariable(expr) => {
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::GetLocalDynamic);
            }
            Expr::VariableVariableAssign { target, value } => {
                self.compile_expr(*target);
                self.compile_expr(*value);
                self.chunk.write_opcode(Opcode::SetLocalDynamic);
            }
            Expr::Assignment { target, value } => {
                if let Some(id) = Self::get_superglobal_id(&target) {
                    self.compile_expr(*value);
                    self.chunk.write_opcode(Opcode::SetSuperglobal);
                    self.chunk.write_byte(id);
                } else {
                    let local_idx = self.resolve_local(&target);
                    if let Expr::MakeRef(_) = &*value {
                        // `$a = &$b` rebinds the slot to a shared cell rather than
                        // copying a value into it, so it must not go through
                        // SetLocal (which writes *through* an existing ref).
                        self.compile_expr(*value);
                        self.chunk.write_opcode(Opcode::BindRefLocal);
                        self.chunk.write_byte(local_idx);
                    } else {
                        self.compile_expr(*value);
                        self.chunk.write_opcode(Opcode::SetLocal);
                        self.chunk.write_byte(local_idx);
                    }
                }
            }
            Expr::ArrayDestructure { elements, value } => {
                self.compile_expr(*value);
                let mut auto_idx = 0i32;
                for (key_opt, target_opt) in elements {
                    let has_key = key_opt.is_some();
                    if let Some(target_expr) = target_opt {
                        self.chunk.write_opcode(Opcode::Dup);
                        if let Some(key_expr) = key_opt {
                            self.compile_expr(key_expr);
                        } else {
                            let idx = self.chunk.add_constant(Value::new_int(auto_idx));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(idx);
                        }
                        self.chunk.write_opcode(Opcode::ArrayGet);
                        self.compile_destructure_target(&target_expr);
                    }
                    if !has_key {
                        auto_idx += 1;
                    }
                }
            }
            Expr::ListDestructure { vars, value } => {
                self.compile_expr(*value);
                let mut auto_idx = 0i32;
                for var_opt in vars {
                    if let Some(target_expr) = var_opt {
                        self.chunk.write_opcode(Opcode::Dup);
                        let idx = self.chunk.add_constant(Value::new_int(auto_idx));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                        self.chunk.write_opcode(Opcode::ArrayGet);
                        self.compile_destructure_target(&target_expr);
                    }
                    auto_idx += 1;
                }
            }
            Expr::MakeRef(inner) => self.compile_make_ref(*inner),
            Expr::ArraySet { array, key, value } => {
                // For nested writes like $a[$k1][$k2] = $v, the intermediate
                // fetch of $a[$k1] must use ArrayGetForWrite so that a missing
                // key is auto-vivified and the new inner array handle is stored
                // back into $a[$k1] before the outer ArraySet runs.
                self.compile_array_lhs(*array);
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    self.chunk.write_opcode(Opcode::Constant);
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_short(null_idx);
                }
                self.compile_expr(*value);
                self.chunk.write_opcode(Opcode::ArraySet);
            }

            Expr::Array(elements) => {
                // Emit CallIntrinsic 9 with 0 arguments (creates empty array)
                self.chunk.write_opcode(Opcode::CallIntrinsic);
                self.chunk.write_byte(9);
                self.chunk.write_byte(0);

                for (key_opt, val_opt) in elements {
                    if let Some(val_expr) = val_opt {
                        if let Expr::Unpack(unpacked) = val_expr {
                            self.compile_expr(*unpacked);
                            self.chunk.write_opcode(Opcode::CallIntrinsic);
                            self.chunk.write_byte(10);
                            self.chunk.write_byte(2);
                        } else {
                            self.chunk.write_opcode(Opcode::Dup);
                            if let Some(key_expr) = key_opt {
                                self.compile_expr(key_expr.clone());
                            } else {
                                self.chunk.write_opcode(Opcode::Constant);
                                let null_idx = self.chunk.add_constant(Value::null());
                                self.chunk.write_short(null_idx);
                            }
                            self.compile_expr(val_expr.clone());
                            self.chunk.write_opcode(Opcode::ArraySet);
                            self.chunk.write_opcode(Opcode::Pop);
                        }
                    }
                }
            }
            Expr::PropertySet {
                object,
                property,
                value,
            } => {
                if let Expr::Identifier(ref name) = *property {
                    self.compile_expr(*object);
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(name.clone())) as *mut ()
                        ));
                    self.compile_expr(*value);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(prop_idx);
                } else if let Expr::LiteralString(ref name) = *property {
                    self.compile_expr(*object);
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(name.clone())) as *mut ()
                        ));
                    self.compile_expr(*value);
                    self.chunk.write_opcode(Opcode::SetProperty);
                    self.chunk.write_short(prop_idx);
                } else {
                    self.compile_expr(*object);
                    self.compile_expr(*property);
                    self.compile_expr(*value);
                    self.chunk.write_opcode(Opcode::SetPropertyDynamic);
                }
            }
            Expr::StaticPropertySet {
                class_name,
                property,
                value,
            } => {
                self.compile_expr(*value);
                if class_name == "static" {
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property)) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::LateStaticPropertySet);
                    self.chunk.write_short(prop_idx);
                } else {
                    let resolved_class = self.resolve_class_name(&class_name);
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(resolved_class)) as *mut (),
                        ));
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property)) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::SetStatic);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(prop_idx);
                }
            }
            Expr::FirstClassCallable(inner) => {
                let callable_expr = match *inner {
                    Expr::Call { callee, .. } => *callee,
                    Expr::MethodCall { object, method, .. } => {
                        Expr::Array(vec![(None, Some(*object)), (None, Some(Expr::LiteralString(method)))])
                    }
                    Expr::StaticMethodCall {
                        class_name, method, ..
                    } => Expr::Array(vec![
                        (None, Some(Expr::LiteralString(class_name))),
                        (None, Some(Expr::LiteralString(method))),
                    ]),
                    Expr::DynamicStaticMethodCall {
                        class_name, method, ..
                    } => Expr::Array(vec![(None, Some(*class_name)), (None, Some(*method))]),
                    Expr::DynamicMethodCall {
                        object, method, ..
                    } => Expr::Array(vec![(None, Some(*object)), (None, Some(*method))]),
                    _ => *inner,
                };

                let closure_class = self
                    .chunk
                    .add_constant(Value::new_string_ptr(Box::into_raw(Box::new(
                        "Closure".to_string(),
                    )) as *mut ()));
                let from_callable_method =
                    self.chunk
                        .add_constant(Value::new_string_ptr(Box::into_raw(Box::new(
                            "fromCallable".to_string(),
                        )) as *mut ()));

                self.compile_expr(callable_expr);

                self.chunk.write_opcode(Opcode::StaticMethodCall);
                self.chunk.write_short(closure_class);
                self.chunk.write_short(from_callable_method);
                self.chunk.write_byte(1);
                self.chunk.write_byte(0);
            }
            Expr::Call { callee, arguments } => {
                let mut intrinsic_id = 0;
                let mut name_opt = None;
                if let Expr::Variable(ref name) = &*callee {
                    name_opt = Some(name.as_str());
                } else if let Expr::Identifier(ref name) = &*callee {
                    name_opt = Some(name.as_str());
                }

                if let Some(name) = name_opt {
                    let local_name = if name.contains('\\') {
                        name.split('\\').last().unwrap()
                    } else {
                        name
                    };
                    if local_name == "isset" {
                        if arguments.is_empty() {
                            let false_idx = self.chunk.add_constant(Value::new_bool(false));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(false_idx);
                            return;
                        }

                        let mut false_jumps = Vec::new();
                        for arg in &arguments {
                            if let Expr::NamedArgument { value, .. } = arg {
                                self.compile_expr(*value.clone());
                            } else {
                                self.compile_expr(arg.clone());
                            }
                            let jump = self.emit_jump(Opcode::JumpIfNull);
                            self.chunk.write_opcode(Opcode::Pop); // pop the non-null value
                            false_jumps.push(jump);
                        }

                        let true_idx = self.chunk.add_constant(Value::new_bool(true));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(true_idx);
                        let end_jump = self.emit_jump(Opcode::Jump);

                        for jump in false_jumps {
                            self.patch_jump(jump);
                            self.chunk.write_opcode(Opcode::Pop); // pop the null value
                        }
                        let false_idx = self.chunk.add_constant(Value::new_bool(false));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(false_idx);

                        self.patch_jump(end_jump);
                        return;
                    } else if local_name == "compact" {
                        self.chunk.write_opcode(Opcode::CallIntrinsic);
                        self.chunk.write_byte(9); // Create empty array
                        self.chunk.write_byte(0);

                        for arg in &arguments {
                            if let Expr::LiteralString(ref var_name) = arg {
                                if let Some(local_idx) = self.symbol_table.resolve_local(var_name) {
                                    self.chunk.write_opcode(Opcode::Dup);
                                    let key_ptr = Box::into_raw(Box::new(var_name.clone()));
                                    let key_idx = self.chunk.add_constant(Value::new_string_ptr(key_ptr as *mut ()));
                                    self.chunk.write_opcode(Opcode::Constant);
                                    self.chunk.write_short(key_idx);
                                    self.chunk.write_opcode(Opcode::GetLocal);
                                    self.chunk.write_byte(local_idx);
                                    self.chunk.write_opcode(Opcode::ArraySet);
                                    self.chunk.write_opcode(Opcode::Pop);
                                }
                            }
                        }
                        return;
                    } else if local_name == "empty" {
                        if let Some(arg) = arguments.first() {
                            if let Expr::NamedArgument { value, .. } = arg {
                                self.compile_expr(*value.clone());
                            } else {
                                self.compile_expr(arg.clone());
                            }
                        } else {
                            let true_idx = self.chunk.add_constant(Value::new_bool(true));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(true_idx);
                            return;
                        }

                        let true_jump = self.emit_jump(Opcode::JumpIfFalse);
                        self.chunk.write_opcode(Opcode::Pop); // pop truthy value
                        let false_idx = self.chunk.add_constant(Value::new_bool(false));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(false_idx);
                        let end_jump = self.emit_jump(Opcode::Jump);

                        self.patch_jump(true_jump);
                        self.chunk.write_opcode(Opcode::Pop); // pop falsy value
                        let true_idx = self.chunk.add_constant(Value::new_bool(true));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(true_idx);

                        self.patch_jump(end_jump);
                        return;
                    }

                    if name.starts_with("__hyperion_intrinsic_") {
                        let _intrinsic_id = match name {
                            "__hyperion_intrinsic_reflection_class_get_methods" => 1,
                            "__hyperion_intrinsic_reflection_class_get_method" => 2,
                            "__hyperion_intrinsic_reflection_method_get_parameters" => 3,
                            "__hyperion_intrinsic_reflection_parameter_get_type" => 4,
                            "__hyperion_intrinsic_reflection_parameter_get_default" => 5,
                            "__hyperion_intrinsic_reflection_parameter_has_default" => 6,
                            "__hyperion_intrinsic_reflection_class_new_instance_args" => 7,
                            "__hyperion_intrinsic_reflection_class_is_instantiable" => 8,
                            "__hyperion_intrinsic_create_array" => 9,
                            "__hyperion_intrinsic_call_method" => 10,
                            "__hyperion_intrinsic_preg_match_offsets" => 11,
                            "__hyperion_intrinsic_get_class" => 12,
                            "__hyperion_intrinsic_reflection_class_implements_interface" => 14,
                            "__hyperion_intrinsic_reflection_property_set_value" => 15,
                            "__hyperion_intrinsic_reflection_property_get_value" => 18,
                            "__hyperion_intrinsic_reflection_property_is_public" => 19,
                            "__hyperion_intrinsic_reflection_property_is_protected" => 20,
                            "__hyperion_intrinsic_reflection_property_is_private" => 21,
                            "__hyperion_intrinsic_reflection_property_is_static" => 22,
                            "__hyperion_intrinsic_reflection_method_invoke_args" => 23,
                            "__hyperion_intrinsic_reflection_method_is_public" => 24,
                            "__hyperion_intrinsic_reflection_method_is_protected" => 25,
                            "__hyperion_intrinsic_reflection_method_is_private" => 26,
                            "__hyperion_intrinsic_reflection_method_is_static" => 27,
                            "__hyperion_intrinsic_reflection_class_get_properties" => 28,
                            "__hyperion_intrinsic_reflection_class_get_parent_class" => 29,
                            _ => 0,
                        };
                    } else if name == "spl_autoload_register" || name == "\\spl_autoload_register" {
                        intrinsic_id = 13;
                    } else if name == "spl_autoload_unregister"
                        || name == "\\spl_autoload_unregister"
                    {
                        intrinsic_id = 16;
                    } else if name == "spl_autoload_functions" || name == "\\spl_autoload_functions"
                    {
                        intrinsic_id = 17;
                    } else if name == "spl_autoload_functions" || name == "\\spl_autoload_functions"
                    {
                        intrinsic_id = 17;
                    } else if name == "unset" || name == "\\unset" {
                        intrinsic_id = 32;
                    }
                }

                if intrinsic_id > 0 {
                    let arity = arguments.len();
                    for arg in arguments {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name.clone()));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value.clone());
                        } else {
                            self.compile_expr(arg.clone());
                        }
                    }
                    self.chunk.write_opcode(Opcode::CallIntrinsic);
                    self.chunk.write_byte(intrinsic_id);
                    self.chunk.write_byte(arity as u8);
                } else {
                    if let Expr::Identifier(ref name) = &*callee {
                        let resolved = self.resolve_class_name(name);
                        let str_ptr = Box::into_raw(Box::new(resolved));
                        let idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                    } else {
                        self.compile_expr((*callee).clone());
                    }



                    let mut has_unpack = false;
                    for arg in &arguments {
                        if matches!(arg, Expr::Unpack(_)) {
                            has_unpack = true;
                            break;
                        }
                    }

                    if has_unpack {
                        self.chunk.write_opcode(Opcode::PushUnpackMarker);
                        for arg in arguments {
                            if let Expr::Unpack(arr) = arg {
                                self.compile_expr(*arr);
                                self.chunk.write_opcode(Opcode::UnpackArray);
                            } else {
                                self.compile_expr(arg.clone());
                            }
                        }
                        self.chunk.write_opcode(Opcode::CallUnpacked);
                    } else {
                        let arity = arguments.len();
                        let callee_name = name_opt.unwrap_or("").to_string();
                        let mut num_named = 0;
                        for (i, arg) in arguments.into_iter().enumerate() {
                            if let Expr::NamedArgument { name, value } = arg {
                                num_named += 1;
                                let name_ptr = Box::into_raw(Box::new(name.clone()));
                                let name_idx = self
                                    .chunk
                                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                                self.chunk.write_opcode(Opcode::Constant);
                                self.chunk.write_short(name_idx);
                                self.compile_expr(*value.clone());
                            } else {
                                self.compile_argument(&callee_name, i, arg.clone());
                            }
                        }

                        if num_named > 0 {
                            self.chunk.write_opcode(Opcode::CallNamed);
                            self.chunk.write_byte(arity as u8);
                            self.chunk.write_byte(num_named as u8);
                        } else {
                            self.chunk.write_opcode(Opcode::Call);
                            self.chunk.write_byte(arity as u8);
                        }
                    }
                }
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                self.compile_expr(*condition);
                let jump_false = self.emit_jump(Opcode::JumpIfFalse);
                self.compile_expr(*true_expr);
                let jump_end = self.emit_jump(Opcode::Jump);
                self.patch_jump(jump_false);
                self.compile_expr(*false_expr);
                self.patch_jump(jump_end);
            }
            Expr::Elvis {
                condition,
                false_expr,
            } => {
                self.compile_expr(*condition);
                self.chunk.write_opcode(Opcode::Dup); // Duplicate condition result
                let jump_false = self.emit_jump(Opcode::JumpIfFalse);

                // If true, condition result is already on stack
                let jump_end = self.emit_jump(Opcode::Jump);

                self.patch_jump(jump_false);
                self.chunk.write_opcode(Opcode::Pop); // Pop false condition
                self.compile_expr(*false_expr);

                self.patch_jump(jump_end);
            }

            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                // Constant Folding (Alpha optimization) - only for int arithmetic
                if let (Expr::LiteralInt(l), Expr::LiteralInt(r)) = (&*left, &*right) {
                    let folded = match operator {
                        Token::Plus => Some(Value::new_int((l + r) as i32)),
                        Token::Minus => Some(Value::new_int((l - r) as i32)),
                        Token::Multiply => Some(Value::new_int((l * r) as i32)),
                        Token::Divide if *r != 0 => {
                            if l % r == 0 {
                                Some(Value::new_int((l / r) as i32))
                            } else {
                                Some(Value::new_float((*l as f64) / (*r as f64)))
                            }
                        }
                        _ => None,
                    };
                    if let Some(val) = folded {
                        let idx = self.chunk.add_constant(val);
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                        return;
                    }
                }

                if operator == Token::InstanceOf {
                    self.compile_expr(*left);
                    if let Expr::Identifier(ref name) = *right {
                        let resolved_name = self.resolve_class_name(name);
                        let name_ptr = Box::into_raw(Box::new(resolved_name));
                        let idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                    } else {
                        self.compile_expr(*right);
                    }
                    self.chunk.write_opcode(Opcode::InstanceOf);
                    return;
                }

                if matches!(operator, Token::LogicalAnd | Token::LogicalOr | Token::NullCoalesce) {
                    if operator == Token::LogicalAnd {
                        self.compile_expr(*left);
                        self.chunk.write_opcode(Opcode::Not);
                        self.chunk.write_opcode(Opcode::Not); // cast to bool
                        self.chunk.write_opcode(Opcode::Dup); // stack: [bool, bool]
                        let jump = self.emit_jump(Opcode::JumpIfFalse); // pops one bool
                        
                        self.chunk.write_opcode(Opcode::Pop); // pop the true
                        self.compile_expr(*right);
                        self.chunk.write_opcode(Opcode::Not);
                        self.chunk.write_opcode(Opcode::Not); // cast to bool
                        
                        self.patch_jump(jump);
                        return;
                    } else if operator == Token::LogicalOr {
                        self.compile_expr(*left);
                        self.chunk.write_opcode(Opcode::Not);
                        self.chunk.write_opcode(Opcode::Not); // cast to bool
                        self.chunk.write_opcode(Opcode::Dup); // stack: [bool, bool]
                        
                        self.chunk.write_opcode(Opcode::Not); // invert for jump, stack: [bool, !bool]
                        let jump = self.emit_jump(Opcode::JumpIfFalse); // jumps if !bool is false (i.e. bool is true)
                        
                        self.chunk.write_opcode(Opcode::Pop); // pop the false
                        self.compile_expr(*right);
                        self.chunk.write_opcode(Opcode::Not);
                        self.chunk.write_opcode(Opcode::Not); // cast to bool
                        
                        self.patch_jump(jump);
                        return;
                    } else if operator == Token::NullCoalesce {
                        self.compile_expr(*left);
                        let jump = self.emit_jump(Opcode::JumpIfNotNull);
                        
                        self.chunk.write_opcode(Opcode::Pop); // Pop the null
                        self.compile_expr(*right);
                        let end_jump = self.emit_jump(Opcode::Jump);
                        
                        self.patch_jump(jump);
                        self.patch_jump(end_jump);
                        return;
                    }
                }

                self.compile_expr(*left);
                self.compile_expr(*right);

                match operator {
                    Token::Plus => self.chunk.write_opcode(Opcode::Add),
                    Token::Minus => self.chunk.write_opcode(Opcode::Subtract),
                    Token::Multiply => self.chunk.write_opcode(Opcode::Multiply),
                    Token::Divide => self.chunk.write_opcode(Opcode::Divide),
                    Token::Modulo => self.chunk.write_opcode(Opcode::Modulo),
                    Token::Dot => self.chunk.write_opcode(Opcode::Concat),
                    Token::Equals => self.chunk.write_opcode(Opcode::Equals),
                    Token::StrictEquals => self.chunk.write_opcode(Opcode::StrictEquals),
                    Token::NotEquals => self.chunk.write_opcode(Opcode::NotEquals),
                    Token::StrictNotEquals => self.chunk.write_opcode(Opcode::StrictNotEquals),
                    Token::LessThan => self.chunk.write_opcode(Opcode::LessThan),
                    Token::GreaterThan => self.chunk.write_opcode(Opcode::GreaterThan),
                    Token::LessThanOrEqual => self.chunk.write_opcode(Opcode::LessThanOrEqual),
                    Token::GreaterThanOrEqual => {
                        self.chunk.write_opcode(Opcode::GreaterThanOrEqual)
                    }
                    Token::Spaceship => self.chunk.write_opcode(Opcode::CompareSpaceship),
                    Token::Pipe => self.chunk.write_opcode(Opcode::BitwiseOr),
                    Token::Ampersand => self.chunk.write_opcode(Opcode::BitwiseAnd),
                    Token::BitwiseXor => self.chunk.write_opcode(Opcode::BitwiseXor),
                    Token::ShiftLeft => self.chunk.write_opcode(Opcode::ShiftLeft),
                    Token::ShiftRight => self.chunk.write_opcode(Opcode::ShiftRight),
                    Token::Power => self.chunk.write_opcode(Opcode::Power),
                    _ => unimplemented!("Operator {:?} not implemented", operator),
                }
            }
            Expr::LiteralString(s) => {
                let str_ptr = Box::into_raw(Box::new(s));
                let idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }

            Expr::LiteralNull => {
                let idx = self.chunk.add_constant(Value::null());
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }
            Expr::LiteralBool(b) => {
                let idx = self.chunk.add_constant(Value::new_bool(b));
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }
            Expr::Identifier(name) => {
                if name.eq_ignore_ascii_case("__DIR__") {
                    // Magic constant substitution
                    let dir = std::path::Path::new(&self.file_path)
                        .parent()
                        .unwrap_or(std::path::Path::new(""))
                        .to_str()
                        .unwrap_or("")
                        .to_string();
                    let str_ptr = Box::into_raw(Box::new(dir));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__FILE__") {
                    let str_ptr = Box::into_raw(Box::new(self.file_path.clone()));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__FUNCTION__") {
                    let func_name = self.current_function.clone().unwrap_or_default();
                    let str_ptr = Box::into_raw(Box::new(func_name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__METHOD__") {
                    let method_name = match (&self.current_class, &self.current_function) {
                        (Some(cls), Some(func)) => format!("{}::{}", cls, func),
                        (None, Some(func)) => func.clone(),
                        _ => String::new(),
                    };
                    let str_ptr = Box::into_raw(Box::new(method_name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__CLASS__") {
                    let class_name = self.current_class.clone().unwrap_or_default();
                    let str_ptr = Box::into_raw(Box::new(class_name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__TRAIT__") {
                    let trait_name = self.current_trait.clone().unwrap_or_default();
                    let str_ptr = Box::into_raw(Box::new(trait_name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__NAMESPACE__") {
                    let str_ptr = Box::into_raw(Box::new(self.current_namespace.clone()));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else if name.eq_ignore_ascii_case("__LINE__") {
                    let idx = self.chunk.add_constant(Value::new_int(0));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else {
                    let str_ptr = Box::into_raw(Box::new(name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::FetchConstant);
                    self.chunk.write_short(idx);
                }
            }
            Expr::New {
                class_name,
                arguments,
            } => {
                if class_name.starts_with('$') {
                    // It's a variable! Let's load the variable onto the stack.
                    self.compile_expr(Expr::Variable(class_name[1..].to_string()));
                    self.chunk.write_opcode(Opcode::NewDynamic);
                } else if let Some((target_class, prop_name)) = class_name.split_once("::$") {
                    if target_class == "static" {
                        self.compile_expr(Expr::LateStaticPropertyGet {
                            property: prop_name.to_string(),
                        });
                    } else {
                        self.compile_expr(Expr::StaticPropertyGet {
                            class_name: target_class.to_string(),
                            property: prop_name.to_string(),
                        });
                    }
                    self.chunk.write_opcode(Opcode::NewDynamic);
                } else if let Some((target_class, const_name)) = class_name.split_once("::") {
                    self.compile_expr(Expr::ClassConstFetch {
                        class_name: target_class.to_string(),
                        constant_name: const_name.to_string(),
                    });
                    self.chunk.write_opcode(Opcode::NewDynamic);
                } else {
                    let resolved_name = self.resolve_class_name(&class_name);
                    let name_ptr = Box::into_raw(Box::new(resolved_name));
                    let idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));

                    self.chunk.write_opcode(Opcode::New);
                    self.chunk.write_short(idx);
                }

                self.chunk.write_opcode(Opcode::Dup);

                let mut has_unpack = false;
                for arg in &arguments {
                    if matches!(arg, Expr::Unpack(_)) {
                        has_unpack = true;
                        break;
                    }
                }

                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    self.chunk.write_opcode(Opcode::CallConstructUnpacked);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for arg in arguments {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    self.chunk.write_opcode(Opcode::CallConstruct);
                    self.chunk.write_byte(arity as u8);
                    self.chunk.write_byte(num_named as u8);
                }

                self.chunk.write_opcode(Opcode::Pop); // Pop constructor return value
            }
            Expr::NewDynamic {
                class_expr,
                arguments,
            } => {
                self.compile_expr(*class_expr);
                self.chunk.write_opcode(Opcode::NewDynamic);

                self.chunk.write_opcode(Opcode::Dup);

                let mut has_unpack = false;
                for arg in &arguments {
                    if matches!(arg, Expr::Unpack(_)) {
                        has_unpack = true;
                        break;
                    }
                }

                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    self.chunk.write_opcode(Opcode::CallConstructUnpacked);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for arg in arguments {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    self.chunk.write_opcode(Opcode::CallConstruct);
                    self.chunk.write_byte(arity as u8);
                    self.chunk.write_byte(num_named as u8);
                }

                self.chunk.write_opcode(Opcode::Pop); // Pop constructor return value
            }
            Expr::MethodCall {
                object,
                method,
                arguments,
            } => {
                self.compile_expr(*object);
                let was_nullsafe = self.in_nullsafe_chain;
                self.in_nullsafe_chain = false;
                let has_unpack = arguments.iter().any(|arg| matches!(arg, Expr::Unpack(_)));
                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    let name_ptr = Box::into_raw(Box::new(method));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_opcode(Opcode::MethodCallUnpacked);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for (i, arg) in arguments.into_iter().enumerate() {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_argument(&method, i, arg);
                        }
                    }
                    let name_ptr = Box::into_raw(Box::new(method));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::MethodCall);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_byte(arity as u8);
                    self.chunk.write_byte(num_named as u8);
                }
                self.in_nullsafe_chain = was_nullsafe;
            }
            Expr::DynamicMethodCall {
                object,
                method,
                arguments,
            } => {
                self.compile_expr(*object);
                let was_nullsafe = self.in_nullsafe_chain;
                self.in_nullsafe_chain = false;
                self.chunk.write_opcode(Opcode::PushUnpackMarker);
                for arg in arguments {
                    if let Expr::Unpack(arr) = arg {
                        self.compile_expr(*arr);
                        self.chunk.write_opcode(Opcode::UnpackArray);
                    } else {
                        self.compile_expr(arg);
                    }
                }
                self.compile_expr(*method);
                self.in_nullsafe_chain = was_nullsafe;
                self.chunk.write_opcode(Opcode::MethodCallUnpacked);
            }
            Expr::PropertyGet { object, property } => {
                if let Expr::Identifier(ref name) = *property {
                    self.compile_expr(*object);
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetProperty);
                    self.chunk.write_short(name_idx);
                } else if let Expr::LiteralString(ref name) = *property {
                    self.compile_expr(*object);
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetProperty);
                    self.chunk.write_short(name_idx);
                } else {
                    self.compile_expr(*object);
                    let was_nullsafe = self.in_nullsafe_chain;
                    self.in_nullsafe_chain = false;
                    self.compile_expr(*property);
                    self.in_nullsafe_chain = was_nullsafe;
                    self.chunk.write_opcode(Opcode::GetPropertyDynamic);
                }
            }
            Expr::NullCoalesce { left, right } => {
                self.compile_expr(*left);
                let jump_if_not_null = self.emit_jump(Opcode::JumpIfNotNull);
                self.chunk.write_opcode(Opcode::Pop); // Pop the null
                self.compile_expr(*right);
                self.patch_jump(jump_if_not_null);
            }
            Expr::NullsafePropertyGet { object, property } => {
                if let Expr::Identifier(ref name) = *property {
                    self.compile_expr(*object);
                    let jump_if_null = self.emit_jump(Opcode::JumpIfNull);
                    if self.in_nullsafe_chain {
                        self.nullsafe_exit_jumps.push(jump_if_null);
                    }
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetProperty);
                    self.chunk.write_short(name_idx);
                    if !self.in_nullsafe_chain {
                        self.patch_jump(jump_if_null);
                    }
                } else if let Expr::LiteralString(ref name) = *property {
                    self.compile_expr(*object);
                    let jump_if_null = self.emit_jump(Opcode::JumpIfNull);
                    if self.in_nullsafe_chain {
                        self.nullsafe_exit_jumps.push(jump_if_null);
                    }
                    let name_ptr = Box::into_raw(Box::new(name.clone()));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::GetProperty);
                    self.chunk.write_short(name_idx);
                    if !self.in_nullsafe_chain {
                        self.patch_jump(jump_if_null);
                    }
                } else {
                    self.compile_expr(*object);
                    let jump_if_null = self.emit_jump(Opcode::JumpIfNull);
                    if self.in_nullsafe_chain {
                        self.nullsafe_exit_jumps.push(jump_if_null);
                    }
                    let was_nullsafe = self.in_nullsafe_chain;
                    self.in_nullsafe_chain = false;
                    self.compile_expr(*property);
                    self.in_nullsafe_chain = was_nullsafe;
                    self.chunk.write_opcode(Opcode::GetPropertyDynamic);
                    if !self.in_nullsafe_chain {
                        self.patch_jump(jump_if_null);
                    }
                }
            }
            Expr::NullsafeMethodCall {
                object,
                method,
                arguments,
            } => {
                self.compile_expr(*object);
                let jump_if_null = self.emit_jump(Opcode::JumpIfNull);
                if self.in_nullsafe_chain {
                    self.nullsafe_exit_jumps.push(jump_if_null);
                }

                let was_nullsafe = self.in_nullsafe_chain;
                self.in_nullsafe_chain = false;
                let has_unpack = arguments.iter().any(|arg| matches!(arg, Expr::Unpack(_)));
                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    let name_ptr = Box::into_raw(Box::new(method));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_opcode(Opcode::MethodCallUnpacked);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for arg in arguments {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    let name_ptr = Box::into_raw(Box::new(method));
                    let name_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::MethodCall);
                    self.chunk.write_short(name_idx);
                    self.chunk.write_byte(arity as u8);
                    self.chunk.write_byte(num_named as u8);
                }
                self.in_nullsafe_chain = was_nullsafe;

                if !self.in_nullsafe_chain {
                    self.patch_jump(jump_if_null);
                }
            }
            Expr::DynamicNullsafeMethodCall {
                object,
                method,
                arguments,
            } => {
                self.compile_expr(*object);
                let jump_if_null = self.emit_jump(Opcode::JumpIfNull);
                if self.in_nullsafe_chain {
                    self.nullsafe_exit_jumps.push(jump_if_null);
                }
                let was_nullsafe = self.in_nullsafe_chain;
                self.in_nullsafe_chain = false;
                self.chunk.write_opcode(Opcode::PushUnpackMarker);
                for arg in arguments {
                    if let Expr::Unpack(arr) = arg {
                        self.compile_expr(*arr);
                        self.chunk.write_opcode(Opcode::UnpackArray);
                    } else {
                        self.compile_expr(arg);
                    }
                }
                self.compile_expr(*method);
                self.in_nullsafe_chain = was_nullsafe;
                self.chunk.write_opcode(Opcode::MethodCallUnpacked);
                if !self.in_nullsafe_chain {
                    self.patch_jump(jump_if_null);
                }
            }

            Expr::StaticPropertyGet {
                class_name,
                property,
            } => {
                if class_name.eq_ignore_ascii_case("static") {
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property)) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::LateStaticPropertyGet);
                    self.chunk.write_short(prop_idx);
                } else {
                    let resolved_class = self.resolve_class_name(&class_name);
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(resolved_class)) as *mut (),
                        ));
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(property)) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::GetStatic);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(prop_idx);
                }
            }
            Expr::LateStaticMethodCall { method, arguments } => {
                let arity = arguments.len();
                let mut num_named = 0;
                for (i, arg) in arguments.into_iter().enumerate() {
                    if let Expr::NamedArgument { name, value } = arg {
                        let name_ptr = Box::into_raw(Box::new(name));
                        let name_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(name_idx);
                        self.compile_expr(*value);
                        num_named += 1;
                    } else {
                        self.compile_argument(&method, i, arg);
                    }
                }
                let name_ptr = Box::into_raw(Box::new(method));
                let name_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                self.chunk.write_opcode(Opcode::LateStaticMethodCall);
                self.chunk.write_short(name_idx);
                self.chunk.write_byte(arity as u8);
                self.chunk.write_byte(num_named as u8);
            }
            Expr::LateStaticPropertyGet { property } => {
                let name_ptr = Box::into_raw(Box::new(property));
                let name_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                self.chunk.write_opcode(Opcode::LateStaticPropertyGet);
                self.chunk.write_short(name_idx);
            }
            Expr::Match {
                subject,
                arms,
                default_arm,
            } => {
                self.compile_expr(*subject);

                let mut end_jumps = Vec::new();
                let mut next_arm_jumps = Vec::new();

                for (conditions, result_expr) in arms {
                    if !next_arm_jumps.is_empty() {
                        for jump in next_arm_jumps.drain(..) {
                            self.patch_jump(jump);
                        }
                    }

                    let mut condition_jumps = Vec::new();

                    for cond in conditions {
                        self.chunk.write_opcode(Opcode::Dup); // Dup subject
                        self.compile_expr(cond);
                        self.chunk.write_opcode(Opcode::StrictEquals); // Match uses strict equality

                        let jump_to_next_cond = self.emit_jump(Opcode::JumpIfFalse);

                        let jump_to_result = self.emit_jump(Opcode::Jump);
                        condition_jumps.push(jump_to_result);

                        self.patch_jump(jump_to_next_cond);
                    }

                    let jump_to_next_arm = self.emit_jump(Opcode::Jump);
                    next_arm_jumps.push(jump_to_next_arm);

                    for jump in condition_jumps {
                        self.patch_jump(jump);
                    }

                    self.chunk.write_opcode(Opcode::Pop); // Pop subject
                    self.compile_expr(result_expr);
                    let jump_to_end = self.emit_jump(Opcode::Jump);
                    end_jumps.push(jump_to_end);
                }

                if !next_arm_jumps.is_empty() {
                    for jump in next_arm_jumps.drain(..) {
                        self.patch_jump(jump);
                    }
                }

                self.chunk.write_opcode(Opcode::Pop); // Pop subject

                if let Some(def_expr) = default_arm {
                    self.compile_expr(*def_expr);
                } else {
                    let err_ptr = Box::into_raw(Box::new(format!("Unhandled match case in {} in function {:?}", self.file_path, self.current_function)));
                    let err_msg = self
                        .chunk
                        .add_constant(Value::new_string_ptr(err_ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(err_msg);
                    self.chunk.write_opcode(Opcode::Throw);
                }

                for jump in end_jumps {
                    self.patch_jump(jump);
                }
            }
            Expr::NamedArgument { name: _, value } => {
                // Fallback for constructs like New or StaticMethodCall that don't yet support named args fully
                self.compile_expr(*value);
            }
            Expr::StaticMethodCall {
                class_name,
                method,
                arguments,
            } => {
                let has_unpack = arguments.iter().any(|arg| matches!(arg, Expr::Unpack(_)));
                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    let resolved_class = if class_name.eq_ignore_ascii_case("static") {
                        class_name
                    } else if class_name.eq_ignore_ascii_case("self") {
                        if let Some(ref cur) = self.current_class {
                            format!("self\0{}", cur)
                        } else {
                            class_name
                        }
                    } else if class_name.eq_ignore_ascii_case("parent") {
                        if let Some(ref ext) = self.compiling_class_extends {
                            format!("parent\0{}", ext)
                        } else {
                            class_name
                        }
                    } else {
                        self.resolve_class_name(&class_name)
                    };
                    let class_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(resolved_class)) as *mut (),
                        ));
                    let method_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(method)) as *mut (),
                        ));
                    self.chunk.write_opcode(Opcode::StaticMethodCallUnpacked);
                    self.chunk.write_short(class_idx);
                    self.chunk.write_short(method_idx);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for (i, arg) in arguments.into_iter().enumerate() {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_argument(&method, i, arg);
                        }
                    }
                    if class_name.eq_ignore_ascii_case("static") {
                        let method_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(
                                Box::into_raw(Box::new(method)) as *mut (),
                            ));
                        self.chunk.write_opcode(Opcode::LateStaticMethodCall);
                        self.chunk.write_short(method_idx);
                        self.chunk.write_byte(arity as u8);
                        self.chunk.write_byte(num_named as u8);
                    } else {
                        let resolved_class = if class_name.eq_ignore_ascii_case("self") {
                            if let Some(ref cur) = self.current_class {
                                format!("self\0{}", cur)
                            } else {
                                class_name
                            }
                        } else if class_name.eq_ignore_ascii_case("parent") {
                            if let Some(ref ext) = self.compiling_class_extends {
                                format!("parent\0{}", ext)
                            } else {
                                class_name
                            }
                        } else {
                            self.resolve_class_name(&class_name)
                        };
                        let class_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(
                                Box::into_raw(Box::new(resolved_class)) as *mut (),
                            ));
                        let method_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(
                                Box::into_raw(Box::new(method)) as *mut (),
                            ));
                        self.chunk.write_opcode(Opcode::StaticMethodCall);
                        self.chunk.write_short(class_idx);
                        self.chunk.write_short(method_idx);
                        self.chunk.write_byte(arity as u8);
                        self.chunk.write_byte(num_named as u8);
                    }
                }
            }
            Expr::DynamicStaticMethodCall {
                class_name,
                method,
                arguments,
            } => {
                if let Expr::LiteralString(ref s) = *class_name {
                    let resolved = if s.eq_ignore_ascii_case("static")
                        || s.eq_ignore_ascii_case("self")
                        || s.eq_ignore_ascii_case("parent")
                    {
                        s.clone()
                    } else {
                        self.resolve_class_name(s)
                    };
                    let ptr = Box::into_raw(Box::new(resolved));
                    let idx = self.chunk.add_constant(Value::new_string_ptr(ptr as *mut ()));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else {
                    self.compile_expr(*class_name);
                }
                self.compile_expr(*method);

                let null_idx = self
                    .chunk
                    .add_constant(hyperion_core::memory::nan_box::Value::null());
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(null_idx);

                let has_unpack = arguments.iter().any(|arg| matches!(arg, Expr::Unpack(_)));
                if has_unpack {
                    self.chunk.write_opcode(Opcode::PushUnpackMarker);
                    for arg in arguments {
                        if let Expr::Unpack(arr) = arg {
                            self.compile_expr(*arr);
                            self.chunk.write_opcode(Opcode::UnpackArray);
                        } else {
                            self.compile_expr(arg);
                        }
                    }
                    self.chunk.write_opcode(Opcode::DynamicStaticMethodCallUnpacked);
                } else {
                    let arity = arguments.len();
                    let mut num_named = 0;
                    for arg in arguments {
                        if let Expr::NamedArgument { name, value } = arg {
                            let name_ptr = Box::into_raw(Box::new(name));
                            let name_idx = self
                                .chunk
                                .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                            self.chunk.write_opcode(Opcode::Constant);
                            self.chunk.write_short(name_idx);
                            self.compile_expr(*value);
                            num_named += 1;
                        } else {
                            self.compile_expr(arg);
                        }
                    }

                    self.chunk.write_opcode(Opcode::DynamicStaticMethodCall);
                    self.chunk.write_byte(arity as u8);
                    self.chunk.write_byte(num_named as u8);
                }
            }
            Expr::ArrayGet { array, key } => {
                self.compile_expr(*array);
                let was_nullsafe = self.in_nullsafe_chain;
                self.in_nullsafe_chain = false;
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    self.chunk.write_opcode(Opcode::Constant);
                    let null_idx = self.chunk.add_constant(Value::null());
                    self.chunk.write_short(null_idx);
                }
                self.in_nullsafe_chain = was_nullsafe;
                self.chunk.write_opcode(Opcode::ArrayGet);
            }
            Expr::Require(path) => {
                self.compile_expr(*path);
                self.chunk.write_opcode(Opcode::Include); // Internal opcode
            }
            Expr::Include(path) => {
                self.compile_expr(*path);
                self.chunk.write_opcode(Opcode::Include);
            }
            Expr::RequireOnce(path) => {
                self.compile_expr(*path);
                self.chunk.write_opcode(Opcode::IncludeOnce);
            }
            Expr::IncludeOnce(path) => {
                self.compile_expr(*path);
                self.chunk.write_opcode(Opcode::IncludeOnce);
            }
            // Expr::Match removed because it's handled above
            Expr::Closure { params, uses, body } => {
                let mut closure_compiler = self.new_child();
                closure_compiler.current_function = Some("{closure}".to_string());
                closure_compiler.symbol_table.add_local("<closure>"); // Reserve local 0 for func_val

                // 1. Setup local variables (parameters)
                for param in &params {
                    closure_compiler.symbol_table.add_local(&param.name);
                }
                let mut actual_uses = uses.clone();
                if !actual_uses.iter().any(|(u, _)| u == "this") {
                    actual_uses.push(("this".to_string(), false));
                }

                // Push all explicitly captured values onto the current stack
                for (var, by_ref) in &actual_uses {
                    let local_idx = self.resolve_local(&var);
                    if *by_ref {
                        self.chunk.write_opcode(Opcode::MakeRefLocal);
                        self.chunk.write_byte(local_idx);
                    } else {
                        self.chunk.write_opcode(Opcode::GetLocal);
                        self.chunk.write_byte(local_idx);
                    }
                }
                for (var, _) in &actual_uses {
                    closure_compiler.symbol_table.add_local(&var); // Hidden locals!
                }
                let is_generator = has_yield_stmt(&body);
                if is_generator {
                    closure_compiler.chunk.write_opcode(Opcode::CreateGenerator);
                }
                let mut variadic_index = None;
                for (i, p) in params.iter().enumerate() {
                    if p.is_variadic {
                        variadic_index = Some(i as u8);
                    }
                }
                if let Some(v_idx) = variadic_index {
                    closure_compiler.chunk.write_opcode(Opcode::PackVariadic);
                    closure_compiler.chunk.write_byte(v_idx);
                }
                for stmt in body {
                    closure_compiler.compile_stmt(stmt.clone());
                }

                // Implicit return null at the end
                let null_idx = closure_compiler
                    .chunk
                    .add_constant(hyperion_core::memory::nan_box::Value::null());
                closure_compiler.chunk.write_opcode(Opcode::Constant);
                closure_compiler.chunk.write_short(null_idx);
                closure_compiler.chunk.write_opcode(Opcode::ReturnValue);

                let num_locals = closure_compiler.symbol_table.locals_count() as u8;
                closure_compiler.chunk.local_names = closure_compiler
                    .symbol_table
                    .locals
                    .iter()
                    .map(|l| l.name.clone())
                    .collect();
                closure_compiler.chunk.code[1] = num_locals;

                let mut required_arity = 0;
                for p in &params {
                    if !p.has_default && !p.is_variadic {
                        required_arity += 1;
                    }
                }

                let compiled_params = params
                    .into_iter()
                    .map(|p| closure_compiler.compile_param(p))
                    .collect::<Vec<_>>();

                let closure_name = format!(
                    "closure#{}#{}",
                    self.file_path,
                    GLOBAL_CLOSURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                );

                let closure_func = CompiledFunction {
                    name: closure_name.clone(),
                    arity: required_arity,
                    params: compiled_params,
                    chunk: closure_compiler.chunk,
                    is_closure: true,
                    is_static: false,
                    visibility: hyperion_parser::parser::ast::Visibility::Public,
                    num_locals: closure_compiler.symbol_table.locals_count(),
                    attributes: Vec::new(),
                };

                let name_ptr = Box::into_raw(Box::new(closure_name));
                let name_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));

                self.functions.extend(closure_compiler.functions);
                self.classes.extend(closure_compiler.classes);
                self.functions.push(closure_func);

                self.chunk.write_opcode(Opcode::MakeClosure);
                self.chunk.write_short(name_idx);
                self.chunk.write_byte(actual_uses.len() as u8);
            }
            Expr::ArrowFunction { params, body } => {
                let mut closure_compiler = self.new_child();
                closure_compiler.current_function = Some("{closure}".to_string());
                closure_compiler.symbol_table.add_local("<closure>"); // Reserve local 0 for func_val

                let mut free_vars = Vec::new();
                Self::collect_free_variables(&body, &params, &mut Vec::new(), &mut free_vars);

                // PHP semantics: arrow functions always implicitly capture $this from the
                // enclosing scope if $this is available (i.e. we are inside a method).
                // collect_free_variables may miss $this when it appears inside static
                // method call arguments (e.g. `Str::is($x, $this->foo())`), so we
                // unconditionally inject it here if the parent scope has a 'this' local.
                if !free_vars.contains(&"this".to_string()) {
                    if self.symbol_table.resolve_local("this").is_some() {
                        free_vars.push("this".to_string());
                    }
                }

                for param in &params {
                    closure_compiler.symbol_table.add_local(&param.name);
                }
                // Push all implicitly captured values onto the current stack
                for var in &free_vars {
                    let local_idx = self.resolve_local(&var);
                    self.chunk.write_opcode(Opcode::GetLocal);
                    self.chunk.write_byte(local_idx);
                }
                for var in &free_vars {
                    closure_compiler.symbol_table.add_local(&var); // Hidden locals!
                }
                let mut variadic_index = None;
                for (i, p) in params.iter().enumerate() {
                    if p.is_variadic {
                        variadic_index = Some(i as u8);
                    }
                }
                if let Some(v_idx) = variadic_index {
                    closure_compiler.chunk.write_opcode(Opcode::PackVariadic);
                    closure_compiler.chunk.write_byte(v_idx);
                }
                closure_compiler.compile_expr(*body.clone());
                closure_compiler.chunk.write_opcode(Opcode::ReturnValue);

                let num_locals = closure_compiler.symbol_table.locals_count() as u8;
                closure_compiler.chunk.local_names = closure_compiler
                    .symbol_table
                    .locals
                    .iter()
                    .map(|l| l.name.clone())
                    .collect();
                closure_compiler.chunk.code[1] = num_locals;

                let mut required_arity = 0;
                for p in &params {
                    if !p.has_default && !p.is_variadic {
                        required_arity += 1;
                    }
                }

                let compiled_params = params
                    .into_iter()
                    .map(|p| closure_compiler.compile_param(p))
                    .collect::<Vec<_>>();

                let closure_name = format!(
                    "closure#{}#{}",
                    self.file_path,
                    GLOBAL_CLOSURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                );

                let closure_func = CompiledFunction {
                    name: closure_name.clone(),
                    arity: required_arity,
                    params: compiled_params,
                    chunk: closure_compiler.chunk,
                    is_closure: true,
                    is_static: false,
                    visibility: hyperion_parser::parser::ast::Visibility::Public,
                    num_locals: closure_compiler.symbol_table.locals_count(),
                    attributes: Vec::new(),
                };

                let name_ptr = Box::into_raw(Box::new(closure_name));
                let name_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));

                self.functions.extend(closure_compiler.functions);
                self.classes.extend(closure_compiler.classes);
                self.functions.push(closure_func);

                self.chunk.write_opcode(Opcode::MakeClosure);
                self.chunk.write_short(name_idx);
                self.chunk.write_byte(free_vars.len() as u8);
            }
            Expr::UnaryNot(expr) => {
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::Not);
            }
            Expr::UnaryMinus(expr) => {
                // Compile 0 - expr
                let idx = self.chunk.add_constant(Value::new_int(0));
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::Subtract);
            }

            Expr::Clone(expr) => {
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::Clone);
            }
            Expr::Unpack(expr) => {
                // TODO: Implement actual unpacking via Opcode::Unpack
                self.compile_expr(*expr);
            }
            Expr::Yield { key, value } => {
                if let Some(k) = key {
                    self.compile_expr(*k);
                } else {
                    let null_val = Value::null();
                    let null_idx = self.chunk.add_constant(null_val);
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(null_idx);
                }
                if let Some(v) = value {
                    self.compile_expr(*v);
                } else {
                    let null_val = Value::null();
                    let null_idx = self.chunk.add_constant(null_val);
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(null_idx);
                }
                self.chunk.write_opcode(Opcode::Yield);
            }
            Expr::YieldFrom(iterable) => {
                self.compile_expr(*iterable);
                self.chunk.write_opcode(Opcode::IterInit);
                self.chunk.write_short(0xffff);
                let iter_init_jump = self.chunk.code.len() - 2;

                let loop_start = self.chunk.code.len();

                let key_local = self.resolve_local("#yf_k");
                let value_local = self.resolve_local("#yf_v");

                self.chunk.write_opcode(Opcode::IterNext);
                self.chunk.write_byte(key_local);
                self.chunk.write_byte(value_local);
                self.chunk.write_byte(0);
                self.chunk.write_short(0xffff);
                let iter_next_jump_offset = self.chunk.code.len() - 2;

                self.chunk.write_opcode(Opcode::GetLocal);
                self.chunk.write_byte(key_local);
                self.chunk.write_opcode(Opcode::GetLocal);
                self.chunk.write_byte(value_local);
                self.chunk.write_opcode(Opcode::Yield);
                self.chunk.write_opcode(Opcode::Pop);

                self.emit_loop(loop_start);

                self.patch_jump(iter_init_jump);
                self.patch_jump(iter_next_jump_offset);

                self.chunk.write_opcode(Opcode::Pop); // pop index
                self.chunk.write_opcode(Opcode::Pop); // pop iterable

                let null_val = Value::null();
                let null_idx = self.chunk.add_constant(null_val);
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(null_idx);
            }
            Expr::ClassConstFetch {
                class_name,
                constant_name,
            } => {
                if class_name.eq_ignore_ascii_case("static") {
                    let prop_idx = self
                        .chunk
                        .add_constant(Value::new_string_ptr(
                            Box::into_raw(Box::new(constant_name)) as *mut ()
                        ));
                    self.chunk.write_opcode(Opcode::LateStaticPropertyGet);
                    self.chunk.write_short(prop_idx);
                } else {
                    let resolved_class = self.resolve_class_name(&class_name);
                    if constant_name.to_lowercase() == "class" {
                        let str_ptr = Box::into_raw(Box::new(resolved_class));
                        let idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(str_ptr as *mut ()));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(idx);
                    } else {
                        let class_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(
                                Box::into_raw(Box::new(resolved_class)) as *mut (),
                            ));
                        let prop_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(
                                Box::into_raw(Box::new(constant_name)) as *mut (),
                            ));
                        self.chunk.write_opcode(Opcode::GetStatic);
                        self.chunk.write_short(class_idx);
                        self.chunk.write_short(prop_idx);
                    }
                }
            }
            Expr::Silence(expr) => {
                self.compile_expr(*expr);
            }
            Expr::Isset(vars) => {
                if vars.is_empty() {
                    let idx = self.chunk.add_constant(Value::new_bool(false));
                    self.chunk.write_opcode(Opcode::Constant);
                    self.chunk.write_short(idx);
                } else {
                    let len = vars.len();
                    let mut jumps = Vec::new();
                    for (i, var) in vars.into_iter().enumerate() {
                        self.compile_isset_expr(var);
                        if i < len - 1 {
                            self.chunk.write_opcode(Opcode::Dup);
                            let j = self.emit_jump(Opcode::JumpIfFalse);
                            self.chunk.write_opcode(Opcode::Pop);
                            jumps.push(j);
                        }
                    }
                    for j in jumps {
                        self.patch_jump(j);
                    }
                }
            }
            Expr::Empty(expr) => {
                self.compile_expr_quiet(*expr);
                self.chunk.write_opcode(Opcode::Not);
            }
            Expr::List(_vars) => {
                let idx = self.chunk.add_constant(Value::null());
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx);
            }
            Expr::Eval(expr) => {
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::Eval);
            }
            Expr::Print(expr) => {
                self.compile_expr(*expr);
                self.chunk.write_opcode(Opcode::Echo); // Approximation
                let idx = self.chunk.add_constant(Value::new_int(1));
                self.chunk.write_opcode(Opcode::Constant);
                self.chunk.write_short(idx); // print returns 1
            }
            Expr::NewAnonymousClass {
                extends,
                implements,
                uses,
                methods,
                mut properties,
                arguments,
            } => {
                let anon_class_name = format!(
                    "class#anon#{}#{}",
                    self.file_path,
                    GLOBAL_ANON_CLASS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                );
                self.class_registry.push(anon_class_name.clone());

                let mut method_entries: Vec<(Stmt, Option<(HashMap<String, String>, String)>)> = methods
                    .into_iter()
                    .map(|m| (m, None))
                    .collect();

                // Trait flattening
                let mut trait_queue: std::collections::VecDeque<String> = uses
                    .iter()
                    .map(|u| self.resolve_class_name(u))
                    .collect();
                let mut processed_traits = std::collections::HashSet::new();
                while let Some(resolved_trait) = trait_queue.pop_front() {
                    if processed_traits.contains(&resolved_trait) {
                        continue;
                    }
                    processed_traits.insert(resolved_trait.clone());

                    if !self.traits_registry.contains_key(&resolved_trait) {
                        self.try_compile_external_class(&resolved_trait);
                    }
                    if let Some(trait_def) = self.traits_registry.get(&resolved_trait).cloned() {
                        for u in trait_def.uses {
                            trait_queue.push_back(u);
                        }
                        for t_m in trait_def.methods {
                            if let Stmt::Function { name: ref t_m_name, body: ref t_m_body, .. } = t_m {
                                let mut existing_idx = None;
                                for (idx, (c_m, _)) in method_entries.iter().enumerate() {
                                    if let Stmt::Function { name: ref c_m_name, .. } = c_m {
                                        if c_m_name == t_m_name {
                                            existing_idx = Some(idx);
                                            break;
                                        }
                                    }
                                }
                                if let Some(idx) = existing_idx {
                                    if let Stmt::Function { body: ref c_m_body, .. } = &method_entries[idx].0 {
                                        if c_m_body.is_empty() && !t_m_body.is_empty() {
                                            method_entries[idx] = (t_m, Some((trait_def.aliases.clone(), trait_def.namespace.clone())));
                                        }
                                    }
                                } else {
                                    method_entries.push((t_m, Some((trait_def.aliases.clone(), trait_def.namespace.clone()))));
                                }
                            }
                        }
                        for t_p in trait_def.properties {
                            // Traits contribute constants as well as properties,
                            // and both are matched by name so a member the class
                            // declares itself is not overwritten.
                            if let Some(t_p_name) = class_member_name(&t_p) {
                                let exists = properties
                                    .iter()
                                    .any(|c_p| class_member_name(c_p) == Some(t_p_name));
                                if !exists {
                                    properties.push(t_p);
                                }
                            }
                        }
                    } else {
                        hyperion_core::hyp_debug!("WARNING: Trait not found: {}", resolved_trait);
                    }
                }

                // Interface validation
                let mut method_names = Vec::new();
                for (m, _) in &method_entries {
                    if let Stmt::Function { name: m_name, .. } = m {
                        method_names.push(m_name.clone());
                    }
                }
                for iface_name in &implements {
                    let resolved_iface = self.resolve_class_name(&iface_name);
                    if !self.interfaces_registry.contains_key(&resolved_iface) {
                        self.try_compile_external_class(&resolved_iface);
                    }
                    if let Some(Stmt::Interface {
                        methods: i_methods, ..
                    }) = self.interfaces_registry.get(&resolved_iface)
                    {
                        for i_m in i_methods {
                            if let Stmt::Function { name: i_m_name, .. } = i_m {
                                if !method_names.contains(i_m_name) {
                                    hyperion_core::hyp_debug!("WARNING: Class {} must implement method {} from interface {}", anon_class_name, i_m_name, resolved_iface);
                                }
                            }
                        }
                    } else {
                        hyperion_core::hyp_debug!("WARNING: Interface not found: {}", resolved_iface);
                    }
                }

                // Compile methods
                let mut anon_default_properties = Vec::new();
                let mut compiled_methods = Vec::new();
                for (method_stmt, trait_scope) in method_entries {
                    if let Stmt::Function {
                        name: method_name,
                        params,
                        body,
                        is_static,
                        visibility,
                        ..
                    } = method_stmt
                    {
                        if method_name.eq_ignore_ascii_case("__construct") {
                            for param in &params {
                                if let Some(vis) = param.promoted_visibility {
                                    if !anon_default_properties.iter().any(|(p, _, _)| p == &param.name) {
                                        let val = match &param.default_expr {
                                            Some(e) => expr_to_literal_value(e, self),
                                            None => Value::null(),
                                        };
                                        anon_default_properties.push((param.name.clone(), val, vis));
                                    }
                                }
                            }
                        }
                        let mut method_compiler = self.new_child();
                        if let Some((trait_aliases, trait_namespace)) = trait_scope {
                            for (k, v) in trait_aliases {
                                method_compiler.aliases.insert(k, v);
                            }
                            method_compiler.current_namespace = trait_namespace;
                        }
                        method_compiler.symbol_table.add_local("this");
                        for param in &params {
                            method_compiler.symbol_table.add_local(&param.name);
                        }

                        let is_generator = has_yield_stmt(&body);
                        if is_generator {
                            method_compiler.chunk.write_opcode(Opcode::CreateGenerator);
                        }

                        // Inject property promotion
                        for param in &params {
                            if param.promoted_visibility.is_some() {
                                method_compiler.compile_expr(Expr::Variable("this".to_string()));
                                let local_idx = method_compiler.resolve_local(&param.name);
                                method_compiler.chunk.write_opcode(Opcode::GetLocal);
                                method_compiler.chunk.write_byte(local_idx);
                                let prop_ptr = Box::into_raw(Box::new(param.name.clone()));
                                let prop_idx = method_compiler
                                    .chunk
                                    .add_constant(Value::new_string_ptr(prop_ptr as *mut ()));
                                method_compiler.chunk.write_opcode(Opcode::SetProperty);
                                method_compiler.chunk.write_short(prop_idx);
                                method_compiler.chunk.write_opcode(Opcode::Pop);
                            }
                        }

                        // Inject variadic packing if needed
                        let mut variadic_index = None;
                        for (i, p) in params.iter().enumerate() {
                            if p.is_variadic {
                                variadic_index = Some(i as u8);
                            }
                        }

                        if let Some(v_idx) = variadic_index {
                            method_compiler.chunk.write_opcode(Opcode::PackVariadic);
                            method_compiler.chunk.write_byte(v_idx);
                        }

                        for stmt in body {
                            method_compiler.compile_stmt(stmt.clone());
                        }

                        method_compiler.chunk.write_opcode(Opcode::Constant);
                        let null_idx = method_compiler.chunk.add_constant(Value::null());
                        method_compiler.chunk.write_short(null_idx);
                        method_compiler.chunk.write_opcode(Opcode::ReturnValue);

                        let required_arity = params
                            .iter()
                            .filter(|p| !p.has_default && !p.is_variadic)
                            .count();
                        let compiled_params = params
                            .into_iter()
                            .map(|p| method_compiler.compile_param(p))
                            .collect::<Vec<_>>();

                        let num_locals = method_compiler.symbol_table.locals_count() as u8;
                        method_compiler.chunk.local_names = method_compiler
                            .symbol_table
                            .locals
                            .iter()
                            .map(|l| l.name.clone())
                            .collect();
                        method_compiler.chunk.code[1] = num_locals;

                        compiled_methods.push(CompiledFunction {
                            name: method_name.clone(),
                            arity: required_arity,
                            params: compiled_params,
                            chunk: method_compiler.chunk,
                            is_closure: false,
                            is_static,
                            visibility,
                            num_locals: num_locals as usize,
                            attributes: Vec::new(),
                        });
                        self.functions.extend(method_compiler.functions);
                        self.classes.extend(method_compiler.classes);
                    }
                }

                let resolved_extends = extends
                    .as_ref()
                    .map(|parent| self.resolve_class_name(parent));

                let resolved_implements = implements
                    .iter()
                    .map(|i| self.resolve_class_name(i))
                    .collect();
                let mut anon_static_properties = Vec::new();
                for prop_stmt in properties {
                    if let Stmt::PropertyDeclaration {
                        name,
                        initial_value,
                        is_static,
                        visibility,
                        is_readonly: _,
                    } = prop_stmt
                    {
                        let val = match initial_value {
                            Some(e) => expr_to_literal_value(&e, self),
                            None => Value::null(),
                        };
                        if is_static {
                            anon_static_properties.push((name.clone(), val, visibility));
                        } else {
                            anon_default_properties.push((name.clone(), val, visibility));
                        }
                    }
                }

                self.classes.push(CompiledClass {
                    name: anon_class_name.clone(),
                    extends: resolved_extends,
                    implements: resolved_implements,
                    traits: Vec::new(),
                    methods: compiled_methods,
                    static_properties: anon_static_properties,
                    default_properties: anon_default_properties,
                    attributes: Vec::new(),
                    is_interface: false,
                    is_trait: false,
                    is_enum: false,
                    is_abstract: false,
                    is_final: false,
                    is_readonly: false,
                });

                let name_ptr = Box::into_raw(Box::new(anon_class_name));
                let name_idx = self
                    .chunk
                    .add_constant(Value::new_string_ptr(name_ptr as *mut ()));

                self.chunk.write_opcode(Opcode::New);
                self.chunk.write_short(name_idx);

                self.chunk.write_opcode(Opcode::Dup);

                let arity = arguments.len();
                let mut num_named = 0;
                for arg in arguments {
                    if let Expr::NamedArgument { name, value } = arg {
                        let name_ptr = Box::into_raw(Box::new(name));
                        let name_idx = self
                            .chunk
                            .add_constant(Value::new_string_ptr(name_ptr as *mut ()));
                        self.chunk.write_opcode(Opcode::Constant);
                        self.chunk.write_short(name_idx);
                        self.compile_expr(*value);
                        num_named += 1;
                    } else {
                        self.compile_expr(arg);
                    }
                }

                self.chunk.write_opcode(Opcode::CallConstruct);
                self.chunk.write_byte(arity as u8);
                self.chunk.write_byte(num_named as u8);
                self.chunk.write_opcode(Opcode::Pop);
            }
            Expr::PreIncrement(expr) => self.compile_inc_dec(*expr, true, Opcode::Inc),
            Expr::PreDecrement(expr) => self.compile_inc_dec(*expr, true, Opcode::Dec),
            Expr::PostIncrement(expr) => self.compile_inc_dec(*expr, false, Opcode::Inc),
            Expr::PostDecrement(expr) => self.compile_inc_dec(*expr, false, Opcode::Dec),
            Expr::CompoundAssign {
                target,
                operator,
                value,
            } => self.compile_compound_assign(*target, operator, *value),
            _ => todo!("Task 3: Compiler missing feature: {:?}", expr),
        }
    }

    fn collect_free_variables(
        expr: &Expr,
        params: &Vec<hyperion_parser::parser::ast::ParamDef>,
        bound: &mut Vec<String>,
        free: &mut Vec<String>,
    ) {
        let mut new_bound = bound.clone();
        for p in params {
            new_bound.push(p.name.clone());
        }
        match expr {
            Expr::Throw(inner) => {
                Self::collect_free_variables(inner, params, bound, free);
            }
            Expr::Variable(name) => {
                if !new_bound.contains(name) && !free.contains(name) {
                    free.push(name.clone());
                }
            }
            Expr::VariableVariable(expr) => {
                Self::collect_free_variables(expr, params, bound, free);
            }
            Expr::BinaryOp { left, right, .. } => {
                Self::collect_free_variables(left, params, bound, free);
                Self::collect_free_variables(right, params, bound, free);
            }
            Expr::MethodCall {
                object, arguments, ..
            } => {
                Self::collect_free_variables(object, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::DynamicMethodCall {
                object,
                method,
                arguments,
            }
            | Expr::DynamicNullsafeMethodCall {
                object,
                method,
                arguments,
            } => {
                Self::collect_free_variables(object, params, bound, free);
                Self::collect_free_variables(method, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::PropertyGet { object, property } => {
                Self::collect_free_variables(object, params, bound, free);
                Self::collect_free_variables(property, params, bound, free);
            }
            Expr::NullsafePropertyGet { object, property } => {
                Self::collect_free_variables(object, params, bound, free);
                Self::collect_free_variables(property, params, bound, free);
            }
            Expr::ArrayGet { array, key } => {
                Self::collect_free_variables(array, params, bound, free);
                if let Some(k) = key {
                    Self::collect_free_variables(k, params, bound, free);
                }
            }
            Expr::ArraySet { array, key, value } => {
                Self::collect_free_variables(array, params, bound, free);
                if let Some(k) = key {
                    Self::collect_free_variables(k, params, bound, free);
                }
                Self::collect_free_variables(value, params, bound, free);
            }
            Expr::Array(elements) => {
                for (key_opt, val_opt) in elements {
                    if let Some(k) = key_opt {
                        Self::collect_free_variables(k, params, bound, free);
                    }
                    if let Some(val_expr) = val_opt {
                        Self::collect_free_variables(val_expr, params, bound, free);
                    }
                }
            }
            Expr::ArrayDestructure { elements, value } => {
                Self::collect_free_variables(value, params, bound, free);
                for (key_opt, val_opt) in elements {
                    if let Some(k) = key_opt {
                        Self::collect_free_variables(k, params, bound, free);
                    }
                    if let Some(val_expr) = val_opt {
                        Self::collect_free_variables(val_expr, params, bound, free);
                    }
                }
            }
            Expr::ListDestructure { vars, value } => {
                Self::collect_free_variables(value, params, bound, free);
                for var_opt in vars {
                    if let Some(v) = var_opt {
                        Self::collect_free_variables(v, params, bound, free);
                    }
                }
            }
            Expr::PropertySet {
                object,
                property,
                value,
            } => {
                Self::collect_free_variables(object, params, bound, free);
                Self::collect_free_variables(property, params, bound, free);
                Self::collect_free_variables(value, params, bound, free);
            }
            Expr::New { arguments, .. } | Expr::NewAnonymousClass { arguments, .. } => {
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::NewDynamic { class_expr, arguments } => {
                Self::collect_free_variables(class_expr, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::Closure {
                params: _inner_params,
                body: _,
                ..
            } => {
                // For Closures with `use`, we don't implicitly capture anything!
                // But for traversal consistency (if they were nested), we might do something.
                // However, they only capture what is in `uses`, and those are evaluated at the outer scope.
                // Since this is collect_free_variables for ArrowFunction implicitly capturing things,
                // if an ArrowFunction contains a Closure, the ArrowFunction needs to capture whatever the Closure's `uses` list has!
                // Wait, that's complex. Let's just traverse `uses` if it were passed.
                // For now, we will do a simple implementation.
            }
            Expr::ArrowFunction {
                params: inner_params,
                body,
            } => {
                let mut new_bound = bound.clone();
                for p in inner_params {
                    new_bound.push(p.name.clone());
                }
                Self::collect_free_variables(body, params, &mut new_bound, free);
            }
            Expr::Call { callee, arguments } => {
                Self::collect_free_variables(callee, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::StaticMethodCall { arguments, .. } | Expr::LateStaticMethodCall { arguments, .. } => {
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::DynamicStaticMethodCall { class_name, method, arguments } => {
                Self::collect_free_variables(class_name, params, bound, free);
                Self::collect_free_variables(method, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::NullsafeMethodCall { object, arguments, .. } => {
                Self::collect_free_variables(object, params, bound, free);
                for arg in arguments {
                    Self::collect_free_variables(arg, params, bound, free);
                }
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                Self::collect_free_variables(condition, params, bound, free);
                Self::collect_free_variables(true_expr, params, bound, free);
                Self::collect_free_variables(false_expr, params, bound, free);
            }
            Expr::Elvis {
                condition,
                false_expr,
            } => {
                Self::collect_free_variables(condition, params, bound, free);
                Self::collect_free_variables(false_expr, params, bound, free);
            }
            Expr::InstanceOf {
                object,
                class_name,
            } => {
                Self::collect_free_variables(object, params, bound, free);
                Self::collect_free_variables(class_name, params, bound, free);
            }
            Expr::Assignment { target: _, value } => {
                Self::collect_free_variables(value, params, bound, free);
            }
            Expr::VariableVariableAssign { target, value } => {
                Self::collect_free_variables(target, params, bound, free);
                Self::collect_free_variables(value, params, bound, free);
            }
            Expr::CompoundAssign {
                target,
                value,
                ..
            } => {
                Self::collect_free_variables(target, params, bound, free);
                Self::collect_free_variables(value, params, bound, free);
            }
            Expr::UnaryNot(e)
            | Expr::UnaryMinus(e)
            | Expr::CastInt(e)
            | Expr::CastString(e)
            | Expr::CastBool(e)
            | Expr::CastFloat(e)
            | Expr::CastArray(e)
            | Expr::CastObject(e)
            | Expr::Clone(e)
            | Expr::Unpack(e)
            | Expr::YieldFrom(e)
            | Expr::Print(e)
            | Expr::Silence(e)
            | Expr::Eval(e)
            | Expr::BitwiseNot(e)
            | Expr::PreIncrement(e)
            | Expr::PostIncrement(e)
            | Expr::PreDecrement(e)
            | Expr::PostDecrement(e)
            | Expr::Require(e)
            | Expr::Include(e)
            | Expr::RequireOnce(e)
            | Expr::IncludeOnce(e) => {
                Self::collect_free_variables(e, params, bound, free);
            }

            Expr::BitwiseAnd(l, r)
            | Expr::BitwiseOr(l, r)
            | Expr::BitwiseXor(l, r)
            | Expr::ShiftLeft(l, r)
            | Expr::ShiftRight(l, r)
            | Expr::NullCoalesce { left: l, right: r } => {
                Self::collect_free_variables(l, params, bound, free);
                Self::collect_free_variables(r, params, bound, free);
            }
            Expr::Match {
                subject,
                arms,
                default_arm,
            } => {
                Self::collect_free_variables(subject, params, bound, free);
                for (conds, body) in arms {
                    for c in conds {
                        Self::collect_free_variables(c, params, bound, free);
                    }
                    Self::collect_free_variables(body, params, bound, free);
                }
                if let Some(d) = default_arm {
                    Self::collect_free_variables(d, params, bound, free);
                }
            }
            Expr::Yield { key, value } => {
                if let Some(k) = key {
                    Self::collect_free_variables(k, params, bound, free);
                }
                if let Some(v) = value {
                    Self::collect_free_variables(v, params, bound, free);
                }
            }
            Expr::Isset(exprs) => {
                for e in exprs {
                    Self::collect_free_variables(e, params, bound, free);
                }
            }
            Expr::Empty(e) => {
                Self::collect_free_variables(e, params, bound, free);
            }
            _ => {}
        }
    }
}

fn has_yield_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Throw(inner) => has_yield_expr(inner),
        Expr::Yield { .. } | Expr::YieldFrom(_) => true,
        Expr::BinaryOp { left, right, .. } => has_yield_expr(left) || has_yield_expr(right),
        Expr::UnaryNot(e)
        | Expr::UnaryMinus(e)
        | Expr::CastInt(e)
        | Expr::CastString(e)
        | Expr::CastBool(e)
        | Expr::CastFloat(e)
        | Expr::CastArray(e)
        | Expr::CastObject(e)
        | Expr::Require(e)
        | Expr::Include(e)
        | Expr::RequireOnce(e)
        | Expr::IncludeOnce(e)
        | Expr::Clone(e)
        | Expr::Unpack(e) => has_yield_expr(e),
        Expr::Call { callee, arguments } => {
            has_yield_expr(callee) || arguments.iter().any(has_yield_expr)
        }
        Expr::FirstClassCallable(inner) => has_yield_expr(inner),
        Expr::MethodCall {
            object, arguments, ..
        } => has_yield_expr(object) || arguments.iter().any(has_yield_expr),
        Expr::DynamicMethodCall {
            object,
            method,
            arguments,
        }
        | Expr::DynamicNullsafeMethodCall {
            object,
            method,
            arguments,
        } => {
            has_yield_expr(object)
                || has_yield_expr(method)
                || arguments.iter().any(has_yield_expr)
        }
        Expr::StaticMethodCall { arguments, .. } | Expr::LateStaticMethodCall { arguments, .. } => {
            arguments.iter().any(has_yield_expr)
        }
        Expr::DynamicStaticMethodCall {
            class_name,
            method,
            arguments,
        } => {
            has_yield_expr(class_name)
                || has_yield_expr(method)
                || arguments.iter().any(has_yield_expr)
        }
        Expr::PropertyGet { object, property } | Expr::NullsafePropertyGet { object, property } => {
            has_yield_expr(object) || has_yield_expr(property)
        }
        Expr::PropertySet {
            object,
            property,
            value,
        } => has_yield_expr(object) || has_yield_expr(property) || has_yield_expr(value),
        Expr::StaticPropertySet { value, .. } => has_yield_expr(value),
        Expr::Assignment { value, .. } => has_yield_expr(value),
        Expr::VariableVariableAssign { target, value } => has_yield_expr(target) || has_yield_expr(value),
        Expr::New { arguments, .. }
        | Expr::NewAnonymousClass { arguments, .. } => {
            arguments.iter().any(has_yield_expr)
        }
        Expr::NewDynamic { class_expr, arguments } => {
            has_yield_expr(class_expr) || arguments.iter().any(has_yield_expr)
        }
        Expr::Closure { .. } => false,
        Expr::Match {
            subject,
            arms,
            default_arm,
        } => {
            has_yield_expr(subject)
                || arms
                    .iter()
                    .any(|(conds, body)| conds.iter().any(has_yield_expr) || has_yield_expr(body))
                || default_arm
                    .as_ref()
                    .map(|e| has_yield_expr(e))
                    .unwrap_or(false)
        }
        Expr::Ternary {
            condition,
            true_expr,
            false_expr,
        } => has_yield_expr(condition) || has_yield_expr(true_expr) || has_yield_expr(false_expr),
        Expr::Elvis {
            condition,
            false_expr,
        } => has_yield_expr(condition) || has_yield_expr(false_expr),
        Expr::NullCoalesce { left, right } => has_yield_expr(left) || has_yield_expr(right),
        Expr::VariableVariable(expr) => has_yield_expr(expr),
        Expr::ArrayGet { array, key } => {
            has_yield_expr(array) || key.as_ref().map(|k| has_yield_expr(k)).unwrap_or(false)
        }
        Expr::ArraySet { array, key, value } => {
            has_yield_expr(array)
                || key.as_ref().map(|k| has_yield_expr(k)).unwrap_or(false)
                || has_yield_expr(value)
        }
        Expr::Array(elements) => elements
            .iter()
            .any(|(k, v)| k.as_ref().map(has_yield_expr).unwrap_or(false) || v.as_ref().map(has_yield_expr).unwrap_or(false)),
        Expr::ArrayDestructure { elements, value } => {
            has_yield_expr(value) || elements.iter().any(|(k, v)| k.as_ref().map(has_yield_expr).unwrap_or(false) || v.as_ref().map(has_yield_expr).unwrap_or(false))
        }
        Expr::ListDestructure { vars, value } => {
            has_yield_expr(value) || vars.iter().any(|v| v.as_ref().map(has_yield_expr).unwrap_or(false))
        }
        Expr::NamedArgument { value, .. } => has_yield_expr(value),
        _ => false,
    }
}

fn has_yield_stmt(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::ExprStmt(e) | Stmt::Return(e) | Stmt::Throw(e) => has_yield_expr(e),
        Stmt::Echo(es) => es.iter().any(has_yield_expr),
        Stmt::Block(stmts) => has_yield_stmt(stmts),
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            has_yield_expr(condition)
                || has_yield_stmt(then_branch)
                || else_branch
                    .as_ref()
                    .map(|b| has_yield_stmt(b))
                    .unwrap_or(false)
        }
        Stmt::While { condition, body } => has_yield_expr(condition) || has_yield_stmt(body),
        Stmt::Foreach { iterable, body, .. } => has_yield_expr(iterable) || has_yield_stmt(body),
        Stmt::Switch {
            condition,
            cases,
            default,
        } => {
            has_yield_expr(condition)
                || cases
                    .iter()
                    .any(|(e, s)| has_yield_expr(e) || has_yield_stmt(s))
                || default.as_ref().map(|s| has_yield_stmt(s)).unwrap_or(false)
        }
        Stmt::TryCatch {
            try_body,
            catches,
            finally_body,
        } => {
            has_yield_stmt(try_body)
                || catches.iter().any(|c| has_yield_stmt(&c.body))
                || finally_body
                    .as_ref()
                    .map(|f| has_yield_stmt(f))
                    .unwrap_or(false)
        }
        Stmt::PropertyDeclaration { initial_value, .. } => {
            initial_value.as_ref().map(has_yield_expr).unwrap_or(false)
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperion_parser::parser::ast::Expr;

    #[test]
    fn test_compile_locals() {
        let mut compiler = Compiler::new(String::new());
        // $x = 10;
        compiler.compile_stmt(Stmt::ExprStmt(Expr::Assignment {
            target: "x".to_string(),
            value: Box::new(Expr::LiteralInt(10)),
        }));
        // echo $x;
        compiler.compile_stmt(Stmt::Echo(vec![Expr::Variable("x".to_string())]));

        let chunk = compiler.chunk;

        // chunk should have:
        // OP_CONSTANT (10)
        // OP_SET_LOCAL 0
        // OP_GET_LOCAL 0
        // OP_ECHO

        assert_eq!(chunk.code[2], Opcode::Constant as u8);
        assert_eq!(chunk.code[5], Opcode::SetLocal as u8);
        assert_eq!(chunk.code[6], 0); // index of $x
        assert_eq!(chunk.code[7], Opcode::Pop as u8);
        assert_eq!(chunk.code[8], Opcode::GetLocal as u8);
        assert_eq!(chunk.code[9], 0); // index of $x
        assert_eq!(chunk.code[10], Opcode::Echo as u8);
    }

    #[test]
    fn test_compile_if() {
        let mut compiler = Compiler::new(String::new());
        // if (1) { echo 2; } else { echo 3; }
        compiler.compile_stmt(Stmt::If {
            condition: Expr::LiteralInt(1),
            then_branch: vec![Stmt::Echo(vec![Expr::LiteralInt(2)])],
            else_branch: Some(vec![Stmt::Echo(vec![Expr::LiteralInt(3)])]),
        });

        let chunk = compiler.chunk;

        // chunk should have:
        // 0: OP_CONSTANT (1)
        // 2: OP_JUMP_IF_FALSE
        // 3: MSB of jump (then)
        // 4: LSB of jump (then)
        // 5: OP_CONSTANT (2)
        // 7: OP_ECHO
        // 8: OP_JUMP
        // 9: MSB of jump (else)
        // 10: LSB of jump (else)
        // 11: OP_CONSTANT (3)
        // 13: OP_ECHO

        assert_eq!(chunk.code[2], Opcode::Constant as u8);
        assert_eq!(chunk.code[5], Opcode::JumpIfFalse as u8);

        let then_jump_dist = ((chunk.code[6] as u16) << 8) | (chunk.code[7] as u16);
        assert_eq!(then_jump_dist, 7);

        assert_eq!(chunk.code[8], Opcode::Constant as u8);
        assert_eq!(chunk.code[11], Opcode::Echo as u8);
        assert_eq!(chunk.code[12], Opcode::Jump as u8);

        let else_jump_dist = ((chunk.code[13] as u16) << 8) | (chunk.code[14] as u16);
        assert_eq!(else_jump_dist, 4);
    }

    #[test]
    fn test_compile_while() {
        let mut compiler = Compiler::new(String::new());
        // while (1) { echo 2; }
        compiler.compile_stmt(Stmt::While {
            condition: Expr::LiteralInt(1),
            body: vec![Stmt::Echo(vec![Expr::LiteralInt(2)])],
        });

        let chunk = compiler.chunk;

        // chunk should have:
        // 0: OP_CONSTANT (1)  <- loop_start
        // 2: OP_JUMP_IF_FALSE
        // 3: MSB of jump (exit)
        // 4: LSB of jump (exit)
        // 5: OP_CONSTANT (2)
        // 7: OP_ECHO
        // 8: OP_LOOP
        // 9: MSB of loop jump
        // 10: LSB of loop jump
        // 11: End

        assert_eq!(chunk.code[2], Opcode::Constant as u8);
        assert_eq!(chunk.code[5], Opcode::JumpIfFalse as u8);

        let exit_jump_dist = ((chunk.code[6] as u16) << 8) | (chunk.code[7] as u16);
        assert_eq!(exit_jump_dist, 7);

        assert_eq!(chunk.code[8], Opcode::Constant as u8);
        assert_eq!(chunk.code[11], Opcode::Echo as u8);
        assert_eq!(chunk.code[12], Opcode::Loop as u8);

        let loop_jump_dist = ((chunk.code[13] as u16) << 8) | (chunk.code[14] as u16);
        assert_eq!(loop_jump_dist, 13);
    }
}

/// The declared name of a class-body member — a property or a constant. `None`
/// for anything else, so a method or a stray statement is skipped by callers
/// that match members up by name.
fn class_member_name(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::PropertyDeclaration { name, .. } | Stmt::ConstDeclaration { name, .. } => Some(name),
        _ => None,
    }
}

impl Compiler {
    pub fn find_class_constant(&self, class_name: &str, const_name: &str) -> Option<Value> {
        let mut cur_class = if class_name.eq_ignore_ascii_case("static") {
            self.current_class.clone()
        } else if class_name.eq_ignore_ascii_case("parent") {
            self.compiling_class_extends.clone()
        } else {
            Some(class_name.to_string())
        };
        let mut visited = Vec::new();
        while let Some(cls) = cur_class.take() {
            let norm_class = cls.trim_start_matches('\\').to_lowercase();
            if visited.contains(&norm_class) {
                break;
            }
            visited.push(norm_class.clone());

            // 1. Check constants of class currently being compiled
            if let Some(ref cur) = self.current_class {
                if cur.trim_start_matches('\\').to_lowercase() == norm_class {
                    for (name, val) in &self.compiling_class_constants {
                        if name == const_name {
                            return Some(*val);
                        }
                    }
                }
            }

            // 2. Check interface constants
            for (iface, name, val) in &self.interface_constants {
                if iface.trim_start_matches('\\').to_lowercase() == norm_class && name == const_name {
                    return Some(*val);
                }
            }

            // 3. Check already compiled classes
            let mut parent_to_check = None;
            for c in &self.classes {
                if c.name.trim_start_matches('\\').to_lowercase() == norm_class {
                    for (name, val, _) in &c.static_properties {
                        if name == const_name {
                            return Some(*val);
                        }
                    }
                    if let Some(ref ext) = c.extends {
                        parent_to_check = Some(ext.clone());
                    }
                    break;
                }
            }

            // 4. If this is the class currently being compiled and it extends a parent
            if parent_to_check.is_none() {
                if let Some(ref cur) = self.current_class {
                    if cur.trim_start_matches('\\').to_lowercase() == norm_class {
                        if let Some(ref ext) = self.compiling_class_extends {
                            parent_to_check = Some(ext.clone());
                        }
                    }
                }
            }

            if let Some(parent) = parent_to_check {
                cur_class = Some(parent);
            }
        }
        let clean_class = class_name.trim_start_matches('\\');
        let qualified = format!("{}::{}", clean_class, const_name).to_uppercase();
        match qualified.as_str() {
            "PDO::PARAM_NULL" => Some(Value::new_int(0)),
            "PDO::PARAM_INT" => Some(Value::new_int(1)),
            "PDO::PARAM_STR" => Some(Value::new_int(2)),
            "PDO::PARAM_LOB" => Some(Value::new_int(3)),
            "PDO::PARAM_STMT" => Some(Value::new_int(4)),
            "PDO::PARAM_BOOL" => Some(Value::new_int(5)),
            "PDO::FETCH_LAZY" => Some(Value::new_int(1)),
            "PDO::FETCH_ASSOC" => Some(Value::new_int(2)),
            "PDO::FETCH_NUM" => Some(Value::new_int(3)),
            "PDO::FETCH_BOTH" => Some(Value::new_int(4)),
            "PDO::FETCH_OBJ" => Some(Value::new_int(5)),
            "PDO::FETCH_BOUND" => Some(Value::new_int(6)),
            "PDO::FETCH_COLUMN" => Some(Value::new_int(7)),
            "PDO::FETCH_CLASS" => Some(Value::new_int(8)),
            "PDO::FETCH_INTO" => Some(Value::new_int(9)),
            "PDO::FETCH_FUNC" => Some(Value::new_int(10)),
            "PDO::FETCH_NAMED" => Some(Value::new_int(11)),
            "PDO::FETCH_KEY_PAIR" => Some(Value::new_int(12)),
            "PDO::ATTR_AUTOCOMMIT" => Some(Value::new_int(0)),
            "PDO::ATTR_PREFETCH" => Some(Value::new_int(1)),
            "PDO::ATTR_TIMEOUT" => Some(Value::new_int(2)),
            "PDO::ATTR_ERRMODE" => Some(Value::new_int(3)),
            "PDO::ATTR_SERVER_VERSION" => Some(Value::new_int(4)),
            "PDO::ATTR_CLIENT_VERSION" => Some(Value::new_int(5)),
            "PDO::ATTR_SERVER_INFO" => Some(Value::new_int(6)),
            "PDO::ATTR_CONNECTION_STATUS" => Some(Value::new_int(7)),
            "PDO::ATTR_CASE" => Some(Value::new_int(8)),
            "PDO::ATTR_CURSOR_NAME" => Some(Value::new_int(9)),
            "PDO::ATTR_CURSOR" => Some(Value::new_int(10)),
            "PDO::ATTR_ORACLE_NULLS" => Some(Value::new_int(11)),
            "PDO::ATTR_PERSISTENT" => Some(Value::new_int(12)),
            "PDO::ATTR_STATEMENT_CLASS" => Some(Value::new_int(13)),
            "PDO::ATTR_STRINGIFY_FETCHES" => Some(Value::new_int(14)),
            "PDO::ATTR_DRIVER_NAME" => Some(Value::new_int(16)),
            "PDO::ATTR_DEFAULT_FETCH_MODE" => Some(Value::new_int(19)),
            "PDO::ATTR_EMULATE_PREPARES" => Some(Value::new_int(20)),
            "PDO::ERRMODE_SILENT" => Some(Value::new_int(0)),
            "PDO::ERRMODE_WARNING" => Some(Value::new_int(1)),
            "PDO::ERRMODE_EXCEPTION" => Some(Value::new_int(2)),
            "PDO::CASE_NATURAL" => Some(Value::new_int(0)),
            "PDO::CASE_LOWER" => Some(Value::new_int(2)),
            "PDO::CASE_UPPER" => Some(Value::new_int(1)),
            "PDO::NULL_NATURAL" => Some(Value::new_int(0)),
            "PDO::NULL_EMPTY_STRING" => Some(Value::new_int(1)),
            "PDO::NULL_TO_STRING" => Some(Value::new_int(2)),
            "REFLECTIONATTRIBUTE::IS_INSTANCEOF" => Some(Value::new_int(2)),
            _ => None,
        }
    }
}

fn expr_to_literal_value(expr: &Expr, compiler: &mut Compiler) -> Value {
    match expr {
        Expr::Throw(_) => panic!("Cannot convert throw to literal"),
        Expr::LiteralInt(i) => {
            if *i >= (i32::MIN as i64) && *i <= (i32::MAX as i64) {
                Value::new_int(*i as i32)
            } else {
                Value::new_float(*i as f64)
            }
        }
        Expr::LiteralFloat(f) => Value::new_float(*f),
        Expr::LiteralString(s) => {
            let ptr = Box::into_raw(Box::new(s.clone()));
            Value::new_string_ptr(ptr as *mut ())
        }
        Expr::InterpolatedString(parts) => {
            let mut s = String::new();
            let mut all_const = true;
            for part in parts {
                match part {
                    Expr::LiteralString(lit) => s.push_str(lit),
                    _ => {
                        all_const = false;
                        break;
                    }
                }
            }
            if all_const {
                let ptr = Box::into_raw(Box::new(s));
                Value::new_string_ptr(ptr as *mut ())
            } else {
                Value::null()
            }
        }
        Expr::LiteralBool(b) => Value::new_bool(*b),
        Expr::LiteralNull => Value::null(),
        Expr::Array(elements) if elements.is_empty() => {
            let arr = Box::new(hyperion_core::types::array::PhpArray::new());
            let arr_ptr = Box::into_raw(arr);
            Value::new_array_ptr(arr_ptr as *mut ())
        }
        Expr::Identifier(name) => {
            if name.eq_ignore_ascii_case("__DIR__") {
                let dir = std::path::Path::new(&compiler.file_path)
                    .parent()
                    .unwrap_or(std::path::Path::new(""))
                    .to_str()
                    .unwrap_or("")
                    .to_string();
                let ptr = Box::into_raw(Box::new(dir));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__FILE__") {
                let ptr = Box::into_raw(Box::new(compiler.file_path.clone()));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__FUNCTION__") {
                let func_name = compiler.current_function.clone().unwrap_or_default();
                let ptr = Box::into_raw(Box::new(func_name));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__METHOD__") {
                let method_name = match (&compiler.current_class, &compiler.current_function) {
                    (Some(cls), Some(func)) => format!("{}::{}", cls, func),
                    (None, Some(func)) => func.clone(),
                    _ => String::new(),
                };
                let ptr = Box::into_raw(Box::new(method_name));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__CLASS__") {
                let class_name = compiler.current_class.clone().unwrap_or_default();
                let ptr = Box::into_raw(Box::new(class_name));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__TRAIT__") {
                let trait_name = compiler.current_trait.clone().unwrap_or_default();
                let ptr = Box::into_raw(Box::new(trait_name));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__NAMESPACE__") {
                let ptr = Box::into_raw(Box::new(compiler.current_namespace.clone()));
                Value::new_string_ptr(ptr as *mut ())
            } else if name.eq_ignore_ascii_case("__LINE__") {
                Value::new_int(0)
            } else {
                match name.as_str() {
                    "JSON_HEX_TAG" => Value::new_int(1),
                    "JSON_HEX_AMP" => Value::new_int(2),
                    "JSON_HEX_APOS" => Value::new_int(4),
                    "JSON_HEX_QUOT" => Value::new_int(8),
                    "JSON_FORCE_OBJECT" => Value::new_int(16),
                    "JSON_NUMERIC_CHECK" => Value::new_int(32),
                    "JSON_UNESCAPED_SLASHES" => Value::new_int(64),
                    "JSON_PRETTY_PRINT" => Value::new_int(128),
                    "JSON_UNESCAPED_UNICODE" => Value::new_int(256),
                    "JSON_PARTIAL_OUTPUT_ON_ERROR" => Value::new_int(512),
                    "JSON_PRESERVE_ZERO_FRACTION" => Value::new_int(1024),
                    "JSON_INVALID_UTF8_IGNORE" => Value::new_int(1048576),
                    "JSON_INVALID_UTF8_SUBSTITUTE" => Value::new_int(2097152),
                    "JSON_THROW_ON_ERROR" => Value::new_int(4194304),
                    "JSON_ERROR_NONE" => Value::new_int(0),
                    "JSON_OBJECT_AS_ARRAY" => Value::new_int(1),
                    "JSON_BIGINT_AS_STRING" => Value::new_int(2),
                    "PHP_INT_MAX" => Value::new_int(i32::MAX),
                    "PHP_INT_MIN" => Value::new_int(i32::MIN),
                    _ => Value::null(),
                }
            }
        }
        Expr::BinaryOp {
            left,
            operator,
            right,
        } => {
            if matches!(operator, Token::Dot) {
                let left_val = expr_to_literal_value(left, compiler);
                let right_val = expr_to_literal_value(right, compiler);
                let left_s = match left_val.get_type() {
                    hyperion_core::memory::nan_box::ValueType::String => {
                        let ptr = left_val.as_string_ptr().unwrap() as *const String;
                        unsafe { (*ptr).clone() }
                    }
                    hyperion_core::memory::nan_box::ValueType::Int => {
                        left_val.as_int().unwrap().to_string()
                    }
                    hyperion_core::memory::nan_box::ValueType::Float => {
                        left_val.as_float().unwrap().to_string()
                    }
                    hyperion_core::memory::nan_box::ValueType::Bool => {
                        if left_val.as_bool().unwrap() {
                            "1".to_string()
                        } else {
                            "".to_string()
                        }
                    }
                    _ => String::new(),
                };
                let right_s = match right_val.get_type() {
                    hyperion_core::memory::nan_box::ValueType::String => {
                        let ptr = right_val.as_string_ptr().unwrap() as *const String;
                        unsafe { (*ptr).clone() }
                    }
                    hyperion_core::memory::nan_box::ValueType::Int => {
                        right_val.as_int().unwrap().to_string()
                    }
                    hyperion_core::memory::nan_box::ValueType::Float => {
                        right_val.as_float().unwrap().to_string()
                    }
                    hyperion_core::memory::nan_box::ValueType::Bool => {
                        if right_val.as_bool().unwrap() {
                            "1".to_string()
                        } else {
                            "".to_string()
                        }
                    }
                    _ => String::new(),
                };
                let concat_s = format!("{}{}", left_s, right_s);
                let ptr = Box::into_raw(Box::new(concat_s));
                Value::new_string_ptr(ptr as *mut ())
            } else {
                let left_val = expr_to_literal_value(left, compiler);
                let right_val = expr_to_literal_value(right, compiler);
                match operator {
                    Token::Pipe => {
                        if let (Some(l), Some(r)) = (left_val.as_int(), right_val.as_int()) {
                            Value::new_int(l | r)
                        } else {
                            Value::null()
                        }
                    }
                    Token::Ampersand => {
                        if let (Some(l), Some(r)) = (left_val.as_int(), right_val.as_int()) {
                            Value::new_int(l & r)
                        } else {
                            Value::null()
                        }
                    }
                    Token::BitwiseXor => {
                        if let (Some(l), Some(r)) = (left_val.as_int(), right_val.as_int()) {
                            Value::new_int(l ^ r)
                        } else {
                            Value::null()
                        }
                    }
                    Token::Plus => {
                        if let (Some(l), Some(r)) = (left_val.as_int(), right_val.as_int()) {
                            Value::new_int(l + r)
                        } else if let (Some(l), Some(r)) = (left_val.as_float(), right_val.as_float()) {
                            Value::new_float(l + r)
                        } else {
                            Value::null()
                        }
                    }
                    Token::Minus => {
                        if let (Some(l), Some(r)) = (left_val.as_int(), right_val.as_int()) {
                            Value::new_int(l - r)
                        } else if let (Some(l), Some(r)) = (left_val.as_float(), right_val.as_float()) {
                            Value::new_float(l - r)
                        } else {
                            Value::null()
                        }
                    }
                    _ => Value::null(),
                }
            }
        }
        Expr::Array(elements) => {
            use hyperion_core::types::array::PhpArray;
            let mut arr = PhpArray::new();
            let mut auto_idx: i64 = 0;
            for (key_opt, val_opt) in elements {
                let Some(val_expr) = val_opt else {
                    auto_idx += 1;
                    continue;
                };
                if let Expr::Unpack(unpacked) = val_expr {
                    let source_val = expr_to_literal_value(unpacked, compiler);
                    if let Some(src_ptr) = source_val.as_array_ptr() {
                        let src_arr = unsafe { &*(src_ptr as *const PhpArray) };
                        for (k, v) in src_arr.elements.iter() {
                            match k {
                                hyperion_core::types::array::ArrayKey::Int(_) => {
                                    arr.insert_int(auto_idx, *v);
                                    auto_idx += 1;
                                }
                                hyperion_core::types::array::ArrayKey::StringId(s) => {
                                    arr.insert_string_id(*s, *v);
                                }
                            }
                        }
                    }
                    continue;
                }
                let val = expr_to_literal_value(val_expr, compiler);
                if let Some(key_expr) = key_opt {
                    let key_val = expr_to_literal_value(key_expr, compiler);
                    if let Some(s_ptr) = key_val.as_string_ptr() {
                        let s = unsafe { &*(s_ptr as *const String) };
                        if let Ok(int_key) = s.parse::<i64>() {
                            arr.insert_int(int_key, val);
                            if int_key >= auto_idx {
                                auto_idx = int_key + 1;
                            }
                        } else {
                            if s == "Illuminate\\Foundation\\Application"
                                || s == "Illuminate/Foundation/Application"
                            {
                                let _val_s = if let Some(ptr) = val.as_string_ptr() {
                                    unsafe { (*(ptr as *const String)).clone() }
                                } else {
                                    format!("{:?}", val.get_type())
                                };
                            }
                            arr.insert_string_id(
                                hyperion_core::types::string_table::intern_string(&s),
                                val,
                            );
                        }
                    } else if let Some(ki) = key_val.as_int() {
                        arr.insert_int(ki as i64, val);
                        if ki as i64 >= auto_idx {
                            auto_idx = ki as i64 + 1;
                        }
                    } else {
                        arr.insert_int(auto_idx, val);
                        auto_idx += 1;
                    }
                } else {
                    arr.insert_int(auto_idx, val);
                    auto_idx += 1;
                }
            }
            let ptr = Box::into_raw(Box::new(arr));
            Value::new_array_ptr(ptr as *mut ())
        }
        Expr::Call { callee, arguments } => {
            // array(...) literal
            let is_array_call = match callee.as_ref() {
                Expr::Identifier(name) => name == "array",
                Expr::Variable(name) => name == "array",
                _ => false,
            };
            if is_array_call {
                use hyperion_core::types::array::PhpArray;
                let mut arr = PhpArray::new();
                let mut auto_idx: i64 = 0;
                for arg in arguments {
                    match arg {
                        Expr::ArraySet {
                            key: Some(key_expr),
                            value,
                            ..
                        } => {
                            let val = expr_to_literal_value(value, compiler);
                            let key_val = expr_to_literal_value(key_expr, compiler);
                            if let Some(s_ptr) = key_val.as_string_ptr() {
                                let s = unsafe { &*(s_ptr as *const String) };
                                if let Ok(int_key) = s.parse::<i64>() {
                                    arr.insert_int(int_key, val);
                                    if int_key >= auto_idx {
                                        auto_idx = int_key + 1;
                                    }
                                } else {
                                    arr.insert_string_id(
                                        hyperion_core::types::string_table::intern_string(&s),
                                        val,
                                    );
                                }
                            } else if let Some(ki) = key_val.as_int() {
                                arr.insert_int(ki as i64, val);
                                if ki as i64 >= auto_idx {
                                    auto_idx = ki as i64 + 1;
                                }
                            } else {
                                arr.insert_int(auto_idx, val);
                                auto_idx += 1;
                            }
                        }
                        Expr::ArraySet {
                            key: None, value, ..
                        } => {
                            let val = expr_to_literal_value(value, compiler);
                            arr.insert_int(auto_idx, val);
                            auto_idx += 1;
                        }
                        _ => {
                            let val = expr_to_literal_value(arg, compiler);
                            arr.insert_int(auto_idx, val);
                            auto_idx += 1;
                        }
                    }
                }
                let ptr = Box::into_raw(Box::new(arr));
                Value::new_array_ptr(ptr as *mut ())
            } else {
                Value::null()
            }
        }
        Expr::UnaryMinus(inner) => match inner.as_ref() {
            Expr::LiteralInt(i) => {
                let neg = -*i;
                if neg >= (i32::MIN as i64) && neg <= (i32::MAX as i64) {
                    Value::new_int(neg as i32)
                } else {
                    Value::new_float(neg as f64)
                }
            }
            Expr::LiteralFloat(f) => Value::new_float(-f),
            _ => Value::null(),
        },
        Expr::ClassConstFetch { class_name, constant_name } => {
            let resolved = compiler.resolve_class_name(class_name);
            if constant_name.to_lowercase() == "class" {
                let ptr = Box::into_raw(Box::new(resolved));
                Value::new_string_ptr(ptr as *mut ())
            } else {
                if let Some(val) = compiler.find_class_constant(&resolved, constant_name) {
                    val
                } else {
                    let ok = compiler.try_compile_external_class(&resolved);
                    if let Some(val) = compiler.find_class_constant(&resolved, constant_name) {
                        val
                    } else {
                        if resolved.contains("Level") {
                            println!("DEBUG ClassConstFetch: resolved={} const={} ok={} file_path={}", resolved, constant_name, ok, compiler.file_path);
                        }
                        Value::null()
                    }
                }
            }
        }
        Expr::NamedArgument { value, .. } => expr_to_literal_value(value, compiler),
        Expr::New {
            class_name,
            arguments,
        } => {
            let resolved = compiler.resolve_class_name(class_name);
            let mut obj = hyperion_core::types::object::PhpObject::new_with_name(0, resolved.clone());
            let mut auto_idx: i64 = 0;
            for arg in arguments {
                match arg {
                    Expr::NamedArgument { name, value } => {
                        let val = expr_to_literal_value(value, compiler);
                        obj.properties.insert(name.clone(), val);
                    }
                    _ => {
                        let val = expr_to_literal_value(arg, compiler);
                        obj.properties.insert(auto_idx.to_string(), val);
                        auto_idx += 1;
                    }
                }
            }
            let ptr = Box::into_raw(Box::new(obj));
            Value::new_object_ptr(ptr as *mut ())
        }
        Expr::NewDynamic {
            class_expr,
            arguments,
        } => {
            let class_val = expr_to_literal_value(class_expr, compiler);
            let class_name = if let Some(s_ptr) = class_val.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else {
                "stdClass".to_string()
            };
            let resolved = compiler.resolve_class_name(&class_name);
            let mut obj = hyperion_core::types::object::PhpObject::new_with_name(0, resolved.clone());
            let mut auto_idx: i64 = 0;
            for arg in arguments {
                match arg {
                    Expr::NamedArgument { name, value } => {
                        let val = expr_to_literal_value(value, compiler);
                        obj.properties.insert(name.clone(), val);
                    }
                    _ => {
                        let val = expr_to_literal_value(arg, compiler);
                        obj.properties.insert(auto_idx.to_string(), val);
                        auto_idx += 1;
                    }
                }
            }
            let ptr = Box::into_raw(Box::new(obj));
            Value::new_object_ptr(ptr as *mut ())
        }
        _ => Value::null(),
    }
}

#[derive(Clone, Default)]
struct ProjectAutoload {
    classmap: HashMap<String, String>,
    psr4: Vec<(String, Vec<String>)>,
}

static PROJECT_AUTOLOAD: OnceLock<std::sync::RwLock<HashMap<String, ProjectAutoload>>> = OnceLock::new();

thread_local! {
    static COMPILING_CLASSES: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

fn find_project_root_from_path(file_path: &str) -> Option<String> {
    let p = std::path::Path::new(file_path);
    let mut curr = if p.is_file() {
        p.parent()
    } else {
        Some(p)
    };
    while let Some(dir) = curr {
        if dir.join("vendor/composer/autoload_classmap.php").exists()
            || dir.join("vendor/composer/autoload_psr4.php").exists()
        {
            return Some(dir.to_string_lossy().to_string());
        }
        curr = dir.parent();
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut curr_cwd = Some(cwd.as_path());
        while let Some(dir) = curr_cwd {
            if dir.join("vendor/composer/autoload_classmap.php").exists()
                || dir.join("vendor/composer/autoload_psr4.php").exists()
            {
                return Some(dir.to_string_lossy().to_string());
            }
            if dir.join("laravel-crud/vendor/composer/autoload_classmap.php").exists() {
                return Some(dir.join("laravel-crud").to_string_lossy().to_string());
            }
            curr_cwd = dir.parent();
        }
    }
    None
}

fn load_classmap_for_project(base_dir: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let classmap_path = format!("{}/vendor/composer/autoload_classmap.php", base_dir);
    if let Ok(content) = std::fs::read_to_string(&classmap_path) {
        for line in content.lines() {
            if !line.contains("=>") {
                continue;
            }
            let parts: Vec<&str> = line.split("=>").collect();
            if parts.len() < 2 {
                continue;
            }
            let key_part = parts[0];
            let key = match extract_quoted_string(key_part) {
                Some(k) => k.replace("\\\\", "\\"),
                None => continue,
            };

            let val_part = parts[1];
            let base_path = if val_part.contains("$vendorDir") {
                format!("{}/vendor", base_dir)
            } else if val_part.contains("$baseDir") {
                format!("{}", base_dir)
            } else {
                "".to_string()
            };

            if let Some(sub_path) = extract_quoted_string(val_part) {
                let full_path = if base_path.is_empty() {
                    sub_path
                } else {
                    format!("{}{}", base_path, sub_path)
                };
                m.insert(key.to_lowercase(), full_path);
            }
        }
    }
    m
}

fn load_psr4_for_project(base_dir: &str) -> Vec<(String, Vec<String>)> {
    let mut psr4_list = Vec::new();
    let psr4_path = format!("{}/vendor/composer/autoload_psr4.php", base_dir);
    if let Ok(content) = std::fs::read_to_string(&psr4_path) {
        let vendor_dir = format!("{}/vendor", base_dir);
        for line in content.lines() {
            if !line.contains("=>") {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, "=>").collect();
            if parts.len() < 2 {
                continue;
            }
            let prefix = match extract_quoted_string(parts[0]) {
                Some(k) => k.replace("\\\\", "\\"),
                None => continue,
            };
            let val_part = parts[1];
            let mut dirs = Vec::new();
            let mut pos = 0;
            while let Some(q_start) = val_part[pos..].find('\'').or_else(|| val_part[pos..].find('"')) {
                let quote_char = val_part[pos..].as_bytes()[q_start] as char;
                let actual_start = pos + q_start;
                if let Some(q_end) = val_part[actual_start + 1..].find(quote_char) {
                    let actual_end = actual_start + 1 + q_end;
                    let sub = &val_part[actual_start + 1..actual_end];
                    let prefix_slice = &val_part[pos..actual_start];
                    let full_dir = if prefix_slice.contains("$vendorDir") {
                        format!("{}{}", vendor_dir, sub)
                    } else if prefix_slice.contains("$baseDir") {
                        format!("{}{}", base_dir, sub)
                    } else if sub.starts_with('/') {
                        sub.to_string()
                    } else {
                        format!("{}/{}", base_dir, sub)
                    };
                    dirs.push(full_dir);
                    pos = actual_end + 1;
                } else {
                    break;
                }
            }
            if !dirs.is_empty() {
                psr4_list.push((prefix, dirs));
            }
        }
    }
    psr4_list.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    psr4_list
}

fn get_class_path(class_name: &str, file_path: &str) -> Option<String> {
    let root = find_project_root_from_path(file_path)?;
    let cache = PROJECT_AUTOLOAD.get_or_init(|| std::sync::RwLock::new(HashMap::new()));
    
    let autoload = {
        let read = cache.read().unwrap();
        if let Some(a) = read.get(&root) {
            a.clone()
        } else {
            drop(read);
            let mut write = cache.write().unwrap();
            let a = ProjectAutoload {
                classmap: load_classmap_for_project(&root),
                psr4: load_psr4_for_project(&root),
            };
            write.insert(root.clone(), a.clone());
            a
        }
    };
    
    let norm = class_name.trim_start_matches('\\');
    let norm_lower = norm.to_lowercase();
    
    // 1. Try classmap
    if let Some(p) = autoload.classmap.get(&norm_lower) {
        if std::path::Path::new(p).exists() {
            return Some(p.clone());
        }
    }
    
    // 2. Try PSR-4
    for (prefix, dirs) in &autoload.psr4 {
        let prefix_clean = prefix.trim_start_matches('\\');
        if norm_lower.starts_with(&prefix_clean.to_lowercase()) {
            let rel = &norm[prefix_clean.len()..];
            let rel_path = rel.replace('\\', "/");
            for d in dirs {
                let candidate = format!("{}/{}.php", d.trim_end_matches('/'), rel_path.trim_start_matches('/'));
                if std::path::Path::new(&candidate).exists() {
                    return Some(candidate);
                }
            }
        }
    }
    
    None
}

fn extract_quoted_string(s: &str) -> Option<String> {
    let mut chars = s.chars().enumerate();
    let mut start_idx = None;
    let mut quote_char = None;

    while let Some((idx, c)) = chars.next() {
        if c == '\'' || c == '"' {
            if quote_char.is_none() {
                quote_char = Some(c);
                start_idx = Some(idx + 1);
            } else if quote_char == Some(c) {
                let start = start_idx.unwrap();
                return Some(s[start..idx].to_string());
            }
        }
    }
    None
}

impl Compiler {
    pub fn try_compile_external_class(&mut self, class_name: &str) -> bool {
        let normalized = class_name.to_lowercase();
        let is_already_compiling = COMPILING_CLASSES.with(|cell| {
            let mut set = cell.borrow_mut();
            if set.contains(&normalized) {
                true
            } else {
                set.insert(normalized.clone());
                false
            }
        });
        if is_already_compiling {
            if class_name.contains("Date") || class_name.contains("Macro") {
                hyperion_core::hyp_debug!("DEBUG COMPILER: {} already compiling", class_name);
            }
            return false;
        }

        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                COMPILING_CLASSES.with(|cell| {
                    cell.borrow_mut().remove(&self.0);
                });
            }
        }
        let _cleanup = Cleanup(normalized.clone());

        // Find class path
        let path = match get_class_path(class_name, &self.file_path) {
            Some(p) => p,
            None => return false,
        };

        // Read file
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        // Parse
        hyperion_core::hyp_debug!("Compiling file: {}", path);
        let lexer = hyperion_parser::lexer::Lexer::new(&content);
        let mut parser = hyperion_parser::parser::Parser::new(lexer);
        let program = parser.parse_program();

        // Create compiler with current registry state
        let mut sub_compiler = Compiler::new(path.clone());
        sub_compiler.class_registry = self.class_registry.clone();
        sub_compiler.traits_registry = self.traits_registry.clone();
        sub_compiler.interfaces_registry = self.interfaces_registry.clone();

        // Compile
        let result = sub_compiler.compile(program);

        if let Some(trait_def) = result.traits_registry.get(class_name) {
            hyperion_core::hyp_debug!("DEBUG COMPILER: register_trait for {} produced {} methods", class_name, trait_def.methods.len());
            for m in &trait_def.methods {
                if let Stmt::Function { name, .. } = m {
                    hyperion_core::hyp_debug!("DEBUG COMPILER: trait {} has method {}", class_name, name);
                }
            }
        }

        // Merge only traits, interfaces, and constants needed at compile time
        // DO NOT merge full compiled classes or functions from external files
        // into this file's output, as they must be compiled and loaded on-demand
        // by the autoloader/require at runtime!
        self.traits_registry.extend(result.traits_registry);
        self.interfaces_registry.extend(result.interfaces_registry);
        self.interface_constants.extend(result.interface_constants);
        for c in result.classes {
            for (prop_name, val, _) in c.static_properties {
                self.interface_constants.push((c.name.clone(), prop_name, val));
            }
        }

        true
    }
}
