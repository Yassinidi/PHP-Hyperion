use hyperion_core::memory::nan_box::Value;
use crate::types::function::FunctionPtr;

pub struct PhpClosure {
    pub function_ptr: FunctionPtr,
    pub upvalues: Vec<Value>,
    pub called_class_id: usize,
    pub this_val: Option<Value>,
}

impl PhpClosure {
    pub fn new(function_ptr: FunctionPtr, upvalues: Vec<Value>, called_class_id: usize, this_val: Option<Value>) -> Self {
        Self {
            function_ptr,
            upvalues,
            called_class_id,
            this_val,
        }
    }
}
