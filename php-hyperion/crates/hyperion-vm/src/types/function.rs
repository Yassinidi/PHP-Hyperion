use hyperion_bytecode::Chunk;
use hyperion_compiler::compiler::CompiledParam;

pub struct PhpFunction {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub params: Vec<CompiledParam>,
    pub is_method: bool,
    pub is_static: bool,
    pub visibility: hyperion_parser::parser::ast::Visibility,
    pub num_locals: usize,
    pub attributes: Vec<crate::types::class::ClassAttribute>,
    pub jit_entry: Option<hyperion_jit::assembler::CompiledTrace>,
}

impl PhpFunction {
    pub fn new(name: String, arity: usize, chunk: Chunk) -> Self {
        let num_locals = chunk.local_names.len();
        Self { name, arity, chunk, params: Vec::new(), is_method: false, is_static: false, visibility: hyperion_parser::parser::ast::Visibility::Public, num_locals, attributes: Vec::new(), jit_entry: None }
    }

    pub fn new_with_params(name: String, arity: usize, chunk: Chunk, params: Vec<CompiledParam>, is_method: bool, is_static: bool, visibility: hyperion_parser::parser::ast::Visibility, num_locals: usize) -> Self {
        Self { name, arity, chunk, params, is_method, is_static, visibility, num_locals, attributes: Vec::new(), jit_entry: None }
    }

    pub fn new_with_attributes(name: String, arity: usize, chunk: Chunk, params: Vec<CompiledParam>, is_method: bool, is_static: bool, visibility: hyperion_parser::parser::ast::Visibility, num_locals: usize, attributes: Vec<crate::types::class::ClassAttribute>) -> Self {
        Self { name, arity, chunk, params, is_method, is_static, visibility, num_locals, attributes, jit_entry: None }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FunctionPtr(pub *const PhpFunction);

unsafe impl Send for FunctionPtr {}
unsafe impl Sync for FunctionPtr {}
