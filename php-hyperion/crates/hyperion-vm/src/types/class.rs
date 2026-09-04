use std::collections::HashMap;
use hyperion_core::memory::nan_box::Value;
use crate::types::function::FunctionPtr;

#[derive(Clone, Debug)]
pub struct ClassAttribute {
    pub name: String,
    pub args: Vec<(Option<String>, Value)>,
}

#[derive(Clone, Debug)]
pub struct PhpClass {
    pub id: usize,
    pub name: String,
    pub methods: HashMap<String, FunctionPtr>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub traits: Vec<String>,
    /// Ordered, because `foreach` over a plain object visits properties in
    /// declaration order and object construction seeds them from here.
    pub default_properties:
        indexmap::IndexMap<String, (Value, hyperion_parser::parser::ast::Visibility)>,
    pub static_properties:
        indexmap::IndexMap<String, (Value, hyperion_parser::parser::ast::Visibility)>,
    pub attributes: Vec<ClassAttribute>,
    pub filename: Option<String>,
    pub is_interface: bool,
    pub is_trait: bool,
    pub is_enum: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_readonly: bool,
}

impl PhpClass {
    pub fn new(id: usize, name: String) -> Self {
        Self {
            id,
            name,
            methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
            traits: Vec::new(),
            default_properties: indexmap::IndexMap::new(),
            static_properties: indexmap::IndexMap::new(),
            attributes: Vec::new(),
            filename: None,
            is_interface: false,
            is_trait: false,
            is_enum: false,
            is_abstract: false,
            is_final: false,
            is_readonly: false,
        }
    }

    pub fn add_method(&mut self, name: String, method: FunctionPtr) {
        let norm = crate::vm::normalize_name(&name);
        self.methods.insert(name, method);
        if !self.methods.contains_key(&norm) {
            self.methods.insert(norm, method);
        }
    }
    
    pub fn add_native_method(&mut self, name: String, arity: usize, native_fn_name: String) {
        let mut chunk = hyperion_bytecode::Chunk::new();
        chunk.write_opcode(hyperion_bytecode::Opcode::CallNativeMethodForward);
        chunk.write_byte(native_fn_name.len() as u8);
        for b in native_fn_name.bytes() {
            chunk.write_byte(b);
        }
        
        let func = crate::types::function::PhpFunction::new_with_params(
            name.clone(), 
            arity, 
            chunk, 
            Vec::new(),
            true,
            false,
            hyperion_parser::parser::ast::Visibility::Public,
            0,
        );
        let ptr = crate::types::function::FunctionPtr(Box::into_raw(Box::new(func)));
        self.add_method(name, ptr);
    }
    
    pub fn add_native_static_method(&mut self, name: String, arity: usize, native_fn_name: String) {
        let mut chunk = hyperion_bytecode::Chunk::new();
        for i in 0..arity {
            chunk.write_opcode(hyperion_bytecode::Opcode::GetLocal);
            chunk.write_byte(i as u8);
        }
        chunk.write_opcode(hyperion_bytecode::Opcode::CallNative);
        chunk.write_byte(native_fn_name.len() as u8);
        for b in native_fn_name.bytes() {
            chunk.write_byte(b);
        }
        chunk.write_byte(arity as u8);
        chunk.write_opcode(hyperion_bytecode::Opcode::ReturnValue);
        
        let mut params = Vec::new();
        for i in 0..arity {
            params.push(hyperion_compiler::compiler::CompiledParam {
                name: format!("arg{}", i),
                type_hint: None,
                has_default: false,
                default_value: None,
                by_ref: false,
                is_variadic: false,
            });
        }
        
        let func = crate::types::function::PhpFunction::new_with_params(
            name.clone(), 
            arity, 
            chunk, 
            params,
            true,
            true,
            hyperion_parser::parser::ast::Visibility::Public,
            arity.max(1)
        );
        let ptr = crate::types::function::FunctionPtr(Box::into_raw(Box::new(func)));
        self.methods.insert(name, ptr);
    }
    
    pub fn extends(&mut self, parent: &str) {
        self.extends = Some(parent.to_string());
    }

    pub fn implements(&mut self, interface: &str) {
        self.implements.push(interface.to_string());
    }

    pub fn add_property(&mut self, name: &str, default: Value, visibility: hyperion_parser::parser::ast::Visibility) {
        self.default_properties.insert(name.to_string(), (default, visibility));
    }
}
