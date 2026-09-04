use hyperion_core::memory::nan_box::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    Constant,
    Add,
    Subtract,
    Multiply,
    Divide,
    Echo,
    SetLocal,
    GetLocal,
    Return,
    Yield,
    JumpBack,
    CallNative,
    New,
    GetProperty,
    SetProperty,
    JumpIfFalse,
    Jump,
    Loop,
    Call,
    ReturnValue,
    MethodCall,
    Concat,
    Reserve,
    Pop,
    MakeClosure,
    Dup,
    CallConstruct,
    IterInit,
    IterNext,
    GetSuperglobal,
    ArrayGet,
    ArraySet,
    SwitchTable,
    Equals,
    StrictEquals,
    Include,
    IncludeOnce,
    Throw,
    CallIntrinsic,
    
    // Static Opcodes
    GetStatic,
    SetStatic,
    StaticMethodCall,
    JumpIfNotNull,
    JumpIfNull,
    PackVariadic,
    CompareSpaceship,
    CallNamed,
    LateStaticMethodCall,
    DynamicStaticMethodCall,
    LateStaticPropertyGet,
    FetchConstant,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    NotEquals,
    StrictNotEquals,
    LogicalAnd,
    LogicalOr,
    Modulo,
    Not,
    CreateGenerator,
    LateStaticPropertySet,
    Clone,
    GetPropertyDynamic,
    SetPropertyDynamic,
    InstanceOf,
    NewDynamic,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    Power,
    GetLocalQuiet,
    ArrayDelete,
    Inc,
    Dec,
    Eval,
    GetLocalDynamic,
    Dup2,
    EndFinally,
    CastInt,
    CastString,
    CastBool,
    CastFloat,
    CastArray,
    PushUnpackMarker,
    UnpackArray,
    CallUnpacked,
    CallConstructUnpacked,
    MethodCallUnpacked,
    StaticMethodCallUnpacked,

    // References (`&$x`).
    //
    // MakeRefLocal converts a local slot into a shared reference cell in
    // place, then pushes the ref — so the original variable and every future
    // alias observe the same cell. MakeRefElement does the same for an array
    // element. BindRefLocal stores a ref Value into a local slot *without*
    // dereferencing it, which is what distinguishes `$a = &$b` from `$a = $b`.
    MakeRefLocal,
    MakeRefElement,
    BindRefLocal,

    /// `unset($obj->prop)` — pops the property name and the object, and removes
    /// the property from the instance. `ArrayDelete` covers the `$x[$k]` form.
    PropertyDelete,

    /// `const NAME = expr;` at file or namespace scope. Takes a name index and
    /// pops the value, writing it into the same global table `define()` and
    /// `Opcode::FetchConstant` use.
    ///
    /// A dedicated opcode rather than a lowering to `define()`: `const` is a
    /// declaration, so it must not depend on the function registry being
    /// populated, and unlike `define()` it is not a call that userland can
    /// intercept.
    DeclareConst,

    /// `ArrayGetForWrite`: like `ArrayGet` but for the intermediate fetch in a
    /// nested array assignment such as `$a[$k1][$k2] = $v`. If the element at
    /// `$key` does not exist (or is null), a new empty array is created, stored
    /// back into the parent array at that key, and the new array's handle is
    /// pushed. This ensures that `$this->routes['GET']['uri'] = $r` auto-
    /// vivifies `$this->routes['GET']` instead of discarding the write.
    ArrayGetForWrite,

    ArrayIsset,

    DynamicStaticMethodCallUnpacked,
    CastObject,
    Swap,
    CallNativeMethodForward,
    EnsureLocalArray,
    GetPropertyForWrite,
    GetPropertyDynamicForWrite,
    SetLocalDynamic,
    CallNativeForward,
    PropertyIsset,
    PropertyIssetDynamic,
    SetSuperglobal,
    GetStaticVar,
    InitStaticVar,
}

impl TryFrom<u8> for Opcode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Opcode::Constant),
            1 => Ok(Opcode::Add),
            2 => Ok(Opcode::Subtract),
            3 => Ok(Opcode::Multiply),
            4 => Ok(Opcode::Divide),
            5 => Ok(Opcode::Echo),
            6 => Ok(Opcode::SetLocal),
            7 => Ok(Opcode::GetLocal),
            8 => Ok(Opcode::Return),
            9 => Ok(Opcode::Yield),
            10 => Ok(Opcode::JumpBack),
            11 => Ok(Opcode::CallNative),
            12 => Ok(Opcode::New),
            13 => Ok(Opcode::GetProperty),
            14 => Ok(Opcode::SetProperty),
            15 => Ok(Opcode::JumpIfFalse),
            16 => Ok(Opcode::Jump),
            17 => Ok(Opcode::Loop),
            18 => Ok(Opcode::Call),
            19 => Ok(Opcode::ReturnValue),
            20 => Ok(Opcode::MethodCall),
            21 => Ok(Opcode::Concat),
            22 => Ok(Opcode::Reserve),
            23 => Ok(Opcode::Pop),
            24 => Ok(Opcode::MakeClosure),
            25 => Ok(Opcode::Dup),
            26 => Ok(Opcode::CallConstruct),
            27 => Ok(Opcode::IterInit),
            28 => Ok(Opcode::IterNext),
            29 => Ok(Opcode::GetSuperglobal),
            30 => Ok(Opcode::ArrayGet),
            31 => Ok(Opcode::ArraySet),
            32 => Ok(Opcode::SwitchTable),
            33 => Ok(Opcode::Equals),
            34 => Ok(Opcode::StrictEquals),
            35 => Ok(Opcode::Include),
            36 => Ok(Opcode::IncludeOnce),
            37 => Ok(Opcode::Throw),
            38 => Ok(Opcode::CallIntrinsic),
            39 => Ok(Opcode::GetStatic),
            40 => Ok(Opcode::SetStatic),
            41 => Ok(Opcode::StaticMethodCall),
            42 => Ok(Opcode::JumpIfNotNull),
            43 => Ok(Opcode::JumpIfNull),
            44 => Ok(Opcode::PackVariadic),
            45 => Ok(Opcode::CompareSpaceship),
            46 => Ok(Opcode::CallNamed),
            47 => Ok(Opcode::LateStaticMethodCall),
            48 => Ok(Opcode::DynamicStaticMethodCall),
            49 => Ok(Opcode::LateStaticPropertyGet),
            50 => Ok(Opcode::FetchConstant),
            51 => Ok(Opcode::LessThan),
            52 => Ok(Opcode::GreaterThan),
            53 => Ok(Opcode::LessThanOrEqual),
            54 => Ok(Opcode::GreaterThanOrEqual),
            55 => Ok(Opcode::NotEquals),
            56 => Ok(Opcode::StrictNotEquals),
            57 => Ok(Opcode::LogicalAnd),
            58 => Ok(Opcode::LogicalOr),
            59 => Ok(Opcode::Modulo),
            60 => Ok(Opcode::Not),
            61 => Ok(Opcode::CreateGenerator),
            62 => Ok(Opcode::LateStaticPropertySet),
            63 => Ok(Opcode::Clone),
            64 => Ok(Opcode::GetPropertyDynamic),
            65 => Ok(Opcode::SetPropertyDynamic),
            66 => Ok(Opcode::InstanceOf),
            67 => Ok(Opcode::NewDynamic),
            68 => Ok(Opcode::BitwiseOr),
            69 => Ok(Opcode::BitwiseAnd),
            70 => Ok(Opcode::BitwiseXor),
            71 => Ok(Opcode::ShiftLeft),
            72 => Ok(Opcode::ShiftRight),
            73 => Ok(Opcode::Power),
            74 => Ok(Opcode::GetLocalQuiet),
            75 => Ok(Opcode::ArrayDelete),
            76 => Ok(Opcode::Inc),
            77 => Ok(Opcode::Dec),
            78 => Ok(Opcode::Eval),
            79 => Ok(Opcode::GetLocalDynamic),
            80 => Ok(Opcode::Dup2),
            81 => Ok(Opcode::EndFinally),
            82 => Ok(Opcode::CastInt),
            83 => Ok(Opcode::CastString),
            84 => Ok(Opcode::CastBool),
            85 => Ok(Opcode::CastFloat),
            86 => Ok(Opcode::CastArray),
            87 => Ok(Opcode::PushUnpackMarker),
            88 => Ok(Opcode::UnpackArray),
            89 => Ok(Opcode::CallUnpacked),
            90 => Ok(Opcode::CallConstructUnpacked),
            91 => Ok(Opcode::MethodCallUnpacked),
            92 => Ok(Opcode::StaticMethodCallUnpacked),
            93 => Ok(Opcode::MakeRefLocal),
            94 => Ok(Opcode::MakeRefElement),
            95 => Ok(Opcode::BindRefLocal),
            96 => Ok(Opcode::PropertyDelete),
            97 => Ok(Opcode::DeclareConst),
            98 => Ok(Opcode::ArrayGetForWrite),
            99 => Ok(Opcode::ArrayIsset),
            100 => Ok(Opcode::DynamicStaticMethodCallUnpacked),
            101 => Ok(Opcode::CastObject),
            102 => Ok(Opcode::Swap),
            103 => Ok(Opcode::CallNativeMethodForward),
            104 => Ok(Opcode::EnsureLocalArray),
            105 => Ok(Opcode::GetPropertyForWrite),
            106 => Ok(Opcode::GetPropertyDynamicForWrite),
            107 => Ok(Opcode::SetLocalDynamic),
            108 => Ok(Opcode::CallNativeForward),
            109 => Ok(Opcode::PropertyIsset),
            110 => Ok(Opcode::PropertyIssetDynamic),
            111 => Ok(Opcode::SetSuperglobal),
            112 => Ok(Opcode::GetStaticVar),
            113 => Ok(Opcode::InitStaticVar),
            _ => Err(format!("Invalid Opcode: {}", value)),
        }
    }
}

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ExceptionHandler {
    pub start_ip: usize,
    pub end_ip: usize,
    pub catch_ip: usize,
    pub catch_var: String,
    pub catch_class: String,
    pub stack_depth: usize,
}

#[derive(Clone, Debug)]
pub struct FinallyHandler {
    pub start_ip: usize,
    pub end_ip: usize,
    pub finally_ip: usize,
    pub stack_depth: usize,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub jump_tables: Vec<HashMap<Value, u16>>,
    pub exception_handlers: Vec<ExceptionHandler>,
    pub finally_handlers: Vec<FinallyHandler>,
    pub heat_map: HashMap<usize, u32>,
    pub local_names: Vec<String>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            jump_tables: Vec::new(),
            exception_handlers: Vec::new(),
            finally_handlers: Vec::new(),
            heat_map: HashMap::new(),
            local_names: Vec::new(),
        }
    }

    pub fn write_opcode(&mut self, opcode: Opcode) {
        self.code.push(opcode as u8);
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn write_short(&mut self, short: u16) {
        self.code.push(((short >> 8) & 0xff) as u8);
        self.code.push((short & 0xff) as u8);
    }

    pub fn patch_short(&mut self, offset: usize, short: u16) {
        self.code[offset] = ((short >> 8) & 0xff) as u8;
        self.code[offset + 1] = (short & 0xff) as u8;
    }

    pub fn add_constant(&mut self, value: Value) -> u16 {
        self.constants.push(value);
        (self.constants.len() - 1) as u16
    }

    pub fn add_jump_table(&mut self, table: HashMap<Value, u16>) -> u8 {
        self.jump_tables.push(table);
        (self.jump_tables.len() - 1) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_write() {
        let mut chunk = Chunk::new();
        let val = Value::new_int(42);
        let constant_idx = chunk.add_constant(val);
        
        chunk.write_opcode(Opcode::Constant);
        chunk.write_short(constant_idx);
        chunk.write_opcode(Opcode::Return);
        
        assert_eq!(chunk.code.len(), 4);
        assert_eq!(chunk.constants.len(), 1);
        
        // Read back
        let op = Opcode::try_from(chunk.code[0]).unwrap();
        assert_eq!(op, Opcode::Constant);
    }
}
