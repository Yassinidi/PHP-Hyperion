
/// Represents a single recorded operation inside a hot trace.
/// This acts as the JIT's Intermediate Representation (IR).
#[derive(Debug, Clone, PartialEq)]
pub enum TraceIR {
    // Math Operations (Integer)
    Add { dest: usize, left: usize, right: usize },
    Subtract { dest: usize, left: usize, right: usize },
    Multiply { dest: usize, left: usize, right: usize },
    Divide { dest: usize, left: usize, right: usize },

    // Math Operations (Float / Double Precision)
    FloatAdd { dest: usize, left: usize, right: usize },
    FloatSubtract { dest: usize, left: usize, right: usize },
    FloatMultiply { dest: usize, left: usize, right: usize },
    FloatDivide { dest: usize, left: usize, right: usize },
    FloatNegate { dest: usize, src: usize },
    
    // Comparisons (Integer)
    LessThan { dest: usize, left: usize, right: usize },
    GreaterThan { dest: usize, left: usize, right: usize },
    LessThanOrEqual { dest: usize, left: usize, right: usize },
    GreaterThanOrEqual { dest: usize, left: usize, right: usize },
    Equals { dest: usize, left: usize, right: usize },
    NotEquals { dest: usize, left: usize, right: usize },

    // Comparisons (Float)
    FloatLessThan { dest: usize, left: usize, right: usize },
    FloatGreaterThan { dest: usize, left: usize, right: usize },
    FloatLessThanOrEqual { dest: usize, left: usize, right: usize },
    FloatGreaterThanOrEqual { dest: usize, left: usize, right: usize },
    FloatEquals { dest: usize, left: usize, right: usize },
    FloatNotEquals { dest: usize, left: usize, right: usize },

    // Type Conversions
    IntToFloat { dest: usize, src: usize },
    FloatToInt { dest: usize, src: usize },
    
    // Locals Management
    GetLocal { dest: usize, src: usize },
    SetLocal { dest: usize, src: usize },
    
    // Arrays
    FetchArrayElement { dest: usize, array: usize, key: usize, fn_ptr: u64 },
    ArrayGetPacked { dest: usize, array: usize, key: usize, bailout_ip: usize, bailout_stack_top: usize },
    ArraySetPacked { array: usize, key: usize, val: usize, bailout_ip: usize, bailout_stack_top: usize },
    
    // Objects
    FetchObjectProperty { dest: usize, obj: usize, name_idx: u16, chunk_ptr: u64, fn_ptr: u64 },
    SetObjectProperty { dest: usize, obj: usize, val: usize, name_idx: u16, chunk_ptr: u64, fn_ptr: u64 },
    
    // Constants
    Constant { dest: usize, value: u64 },
    
    // Guards (Type Check / Bailout)
    GuardType { src: usize, expected_type_tag: u64, bailout_ip: usize, bailout_stack_top: usize },
    GuardFloat { src: usize, bailout_ip: usize, bailout_stack_top: usize },
    GuardFunction { src: usize, expected_fn_ptr: u64, bailout_ip: usize, bailout_stack_top: usize },
    GuardClosure { src: usize, expected_closure_fn_ptr: u64, bailout_ip: usize, bailout_stack_top: usize },
    
    // Conditional Jumps
    GuardCondition { src: usize, expected_bool: bool, bailout_ip: usize, bailout_stack_top: usize },
    
    // String
    Concat { dest: usize, left: usize, right: usize, fn_ptr: u64 },
    
    // Explicit Bailout (side-exit for unhandled opcodes like MethodCall)
    Bailout { bailout_ip: usize, bailout_stack_top: usize },
    
    // Trace Completion
    Loop { target_ip: usize },
    Return { src: usize },
}

#[derive(Debug, Clone, Default)]
pub struct RegisterAllocation {
    /// Maps stack slot index -> ARM64 hardware register ID (21..26 for GP, 8..13 for FP)
    pub gp_regs: std::collections::HashMap<usize, u8>,
    pub fp_regs: std::collections::HashMap<usize, u8>,
}

#[derive(Debug, Clone, Default)]
pub struct OptimizedTrace {
    pub preheader: Vec<TraceIR>,
    pub body: Vec<TraceIR>,
    pub reg_alloc: RegisterAllocation,
}

/// Tracks the execution path during a hot loop or function to record a trace.
pub struct Tracer {
    pub is_recording: bool,
    pub start_ip: usize,
    pub trace: Vec<TraceIR>,
    pub current_ip: usize,
    
    // Track simulated stack slots during tracing to map stack ops to virtual registers
    // `simulated_stack_top` acts as the virtual top.
    pub simulated_stack_top: usize,
    pub virtual_registers_assigned: usize,
    pub inline_depth: usize,
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            start_ip: 0,
            trace: Vec::new(),
            current_ip: 0,
            simulated_stack_top: 0,
            virtual_registers_assigned: 0,
            inline_depth: 0,
        }
    }

    pub fn start(&mut self, ip: usize, stack_top: usize) {
        self.is_recording = true;
        self.start_ip = ip;
        self.trace.clear();
        self.simulated_stack_top = stack_top;
        self.virtual_registers_assigned = 0; // Reset virtual register count
        self.inline_depth = 0;
    }

    pub fn stop(&mut self) {
        self.is_recording = false;
    }

    pub fn allocate_vreg(&mut self) -> usize {
        let vreg = self.virtual_registers_assigned;
        self.virtual_registers_assigned += 1;
        vreg
    }

    pub fn record_push(&mut self) -> usize {
        let vreg = self.allocate_vreg();
        self.simulated_stack_top += 1;
        vreg
    }

    pub fn record_pop(&mut self) -> usize {
        self.simulated_stack_top -= 1;
        // In a real tracer, we'd need to know which vreg maps to this stack slot.
        // For now, we will simplify by just returning a pseudo-vreg based on the stack slot.
        self.simulated_stack_top
    }
}
