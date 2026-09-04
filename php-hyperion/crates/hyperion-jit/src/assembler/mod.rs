use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use std::mem;
use std::collections::HashMap;

#[no_mangle]
pub extern "C" fn debug_guard_type(expected: u64, actual: u64, src: u64) {
    if std::env::var("HYPERION_JIT_DEBUG").is_ok() {
        hyperion_core::hyp_debug!("[JIT DEOPT] Guard Type Failed at src {}. Expected: {:016x}, Actual Full: {:016x}", src, expected, actual);
    }
}

#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct JitReturn {
    pub tag: u64,        // 0 = Finished, 1 = Deoptimized
    pub payload_ip: u64, // The IP to resume at
    pub stack_top: u64,  // The stack_top at the moment of bailout
}

pub enum JitResult {
    Finished,
    Deoptimized(usize, usize), // (IP, stack_top)
}

/// Represents an executable machine code buffer and a function pointer to it
pub struct CompiledTrace {
    pub buffer: dynasmrt::ExecutableBuffer,
    pub execute_fn: extern "C" fn(*mut hyperion_core::memory::nan_box::Value, *mut JitReturn),
}

// Ensure CompiledTrace can be shared across threads
unsafe impl Send for CompiledTrace {}
unsafe impl Sync for CompiledTrace {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VmLocation {
    Local(usize),
    Stack(usize),
}

/// Tracks which VM variables are currently residing in which hardware registers.
pub struct RegisterMap {
    // Maps a VM Location to a hardware register index (e.g. 0 for x0, 1 for x1)
    pub map: HashMap<VmLocation, u8>,
    pub next_free_reg: u8,
}

impl Default for RegisterMap {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            next_free_reg: 1, // x0 is usually reserved for the Fibre pointer or return value
        }
    }

    pub fn allocate(&mut self, loc: VmLocation) -> u8 {
        if let Some(&reg) = self.map.get(&loc) {
            return reg;
        }
        let reg = self.next_free_reg;
        self.next_free_reg += 1;
        self.map.insert(loc, reg);
        reg
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn load_gp_slot(
    ops: &mut dynasmrt::aarch64::Assembler,
    slot: usize,
    target: u8,
    reg_alloc: &crate::tracer::RegisterAllocation,
) {
    if let Some(&r) = reg_alloc.gp_regs.get(&slot) {
        match (r, target) {
            (21, 1) => dynasm!(ops; mov x1, x21),
            (21, 2) => dynasm!(ops; mov x2, x21),
            (22, 1) => dynasm!(ops; mov x1, x22),
            (22, 2) => dynasm!(ops; mov x2, x22),
            (23, 1) => dynasm!(ops; mov x1, x23),
            (23, 2) => dynasm!(ops; mov x2, x23),
            (24, 1) => dynasm!(ops; mov x1, x24),
            (24, 2) => dynasm!(ops; mov x2, x24),
            (25, 1) => dynasm!(ops; mov x1, x25),
            (25, 2) => dynasm!(ops; mov x2, x25),
            (26, 1) => dynasm!(ops; mov x1, x26),
            (26, 2) => dynasm!(ops; mov x2, x26),
            _ => {
                let slot_val = slot as u32;
                if target == 1 {
                    dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x1, [x19, x8]);
                } else {
                    dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x2, [x19, x8]);
                }
            }
        }
    } else {
        let slot_val = slot as u32;
        if target == 1 {
            dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x1, [x19, x8]);
        } else {
            dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x2, [x19, x8]);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn store_gp_slot(
    ops: &mut dynasmrt::aarch64::Assembler,
    slot: usize,
    _src_reg: u8,
    reg_alloc: &crate::tracer::RegisterAllocation,
) {
    let slot_val = slot as u32;
    dynasm!(ops
        ; mov w8, slot_val
        ; lsl x8, x8, 3
        ; str x1, [x19, x8]
    );
    if let Some(&r) = reg_alloc.gp_regs.get(&slot) {
        match r {
            21 => dynasm!(ops; mov x21, x1),
            22 => dynasm!(ops; mov x22, x1),
            23 => dynasm!(ops; mov x23, x1),
            24 => dynasm!(ops; mov x24, x1),
            25 => dynasm!(ops; mov x25, x1),
            26 => dynasm!(ops; mov x26, x1),
            _ => {}
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn load_fp_slot(
    ops: &mut dynasmrt::aarch64::Assembler,
    slot: usize,
    target: u8,
    reg_alloc: &crate::tracer::RegisterAllocation,
) {
    if let Some(&r) = reg_alloc.fp_regs.get(&slot) {
        match (r, target) {
            (8, 0) => dynasm!(ops; fmov d0, d8),
            (8, 1) => dynasm!(ops; fmov d1, d8),
            (9, 0) => dynasm!(ops; fmov d0, d9),
            (9, 1) => dynasm!(ops; fmov d1, d9),
            (10, 0) => dynasm!(ops; fmov d0, d10),
            (10, 1) => dynasm!(ops; fmov d1, d10),
            (11, 0) => dynasm!(ops; fmov d0, d11),
            (11, 1) => dynasm!(ops; fmov d1, d11),
            (12, 0) => dynasm!(ops; fmov d0, d12),
            (12, 1) => dynasm!(ops; fmov d1, d12),
            (13, 0) => dynasm!(ops; fmov d0, d13),
            (13, 1) => dynasm!(ops; fmov d1, d13),
            _ => {
                let slot_val = slot as u32;
                if target == 0 {
                    dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d0, [x19, x8]);
                } else {
                    dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d1, [x19, x8]);
                }
            }
        }
    } else {
        let slot_val = slot as u32;
        if target == 0 {
            dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d0, [x19, x8]);
        } else {
            dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d1, [x19, x8]);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn store_fp_slot(
    ops: &mut dynasmrt::aarch64::Assembler,
    slot: usize,
    _src_reg: u8,
    reg_alloc: &crate::tracer::RegisterAllocation,
) {
    let slot_val = slot as u32;
    dynasm!(ops
        ; mov w8, slot_val
        ; lsl x8, x8, 3
        ; str d0, [x19, x8]
    );
    if let Some(&r) = reg_alloc.fp_regs.get(&slot) {
        match r {
            8 => dynasm!(ops; fmov d8, d0),
            9 => dynasm!(ops; fmov d9, d0),
            10 => dynasm!(ops; fmov d10, d0),
            11 => dynasm!(ops; fmov d11, d0),
            12 => dynasm!(ops; fmov d12, d0),
            13 => dynasm!(ops; fmov d13, d0),
            _ => {}
        }
    }
}

#[cfg(target_arch = "aarch64")]
fn emit_ir(
    ops: &mut dynasmrt::aarch64::Assembler,
    ir: &crate::tracer::TraceIR,
    bailouts: &mut Vec<(dynasmrt::DynamicLabel, u32, u32)>,
    start_label: dynasmrt::DynamicLabel,
    reg_alloc: &crate::tracer::RegisterAllocation,
) {
    match ir {
            crate::tracer::TraceIR::GetLocal { dest, src } => {
                if reg_alloc.fp_regs.contains_key(src) || reg_alloc.fp_regs.contains_key(dest) {
                    load_fp_slot(ops, *src, 0, reg_alloc);
                    store_fp_slot(ops, *dest, 0, reg_alloc);
                } else {
                    load_gp_slot(ops, *src, 1, reg_alloc);
                    store_gp_slot(ops, *dest, 1, reg_alloc);
                }
            }
            crate::tracer::TraceIR::SetLocal { dest, src } => {
                if reg_alloc.fp_regs.contains_key(src) || reg_alloc.fp_regs.contains_key(dest) {
                    load_fp_slot(ops, *src, 0, reg_alloc);
                    store_fp_slot(ops, *dest, 0, reg_alloc);
                } else {
                    load_gp_slot(ops, *src, 1, reg_alloc);
                    store_gp_slot(ops, *dest, 1, reg_alloc);
                }
            }
            crate::tracer::TraceIR::FetchArrayElement { dest, array, key, fn_ptr } => {
                let dest_val = *dest as u32;
                let array_val = *array as u32;
                let key_val = *key as u32;
                let fn_val = *fn_ptr;
                
                let val0 = (fn_val & 0xFFFF) as u32;
                let val1 = ((fn_val >> 16) & 0xFFFF) as u32;
                let val2 = ((fn_val >> 32) & 0xFFFF) as u32;
                let val3 = ((fn_val >> 48) & 0xFFFF) as u32;

                dynasm!(ops
                    // Load array into x0 (Arg 0)
                    ; mov w8, array_val
                    ; lsl x8, x8, 3
                    ; ldr x0, [x19, x8]
                    
                    // Load key into x1 (Arg 1)
                    ; mov w8, key_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8]
                    
                    // Load function pointer into x2
                    ; movz x2, val0
                    ; movk x2, val1, lsl 16
                    ; movk x2, val2, lsl 32
                    ; movk x2, val3, lsl 48
                    
                    // Call the function!
                    ; blr x2
                    
                    // Result is in x0. Store to dest.
                    ; mov w8, dest_val
                    ; lsl x8, x8, 3
                    ; str x0, [x19, x8]
                );
            }
            crate::tracer::TraceIR::ArrayGetPacked { dest, array, key, bailout_ip, bailout_stack_top } => {
                let dest_val = *dest as u32;
                let array_val = *array as u32;
                let key_val = *key as u32;
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));

                dynasm!(ops
                    // 1. Load array into x0
                    ; mov w8, array_val
                    ; lsl x8, x8, 3
                    ; ldr x0, [x19, x8]

                    // Mask with POINTER_MASK (0x0000FFFFFFFFFFFF) to extract *mut PhpArray
                    ; movz x2, 0xFFFF
                    ; movk x2, 0xFFFF, lsl 16
                    ; movk x2, 0xFFFF, lsl 32
                    ; and x0, x0, x2

                    // 2. Check if pointer is non-null
                    ; cbz x0, =>bailout_label

                    // 3. Load key into w1 (zero-extend to x1)
                    ; mov w8, key_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8]
                    ; mov w1, w1

                    // 4. Check is_packed at offset 24
                    ; ldrb w2, [x0, 24]
                    ; cbz w2, =>bailout_label

                    // 5. Load packed.ptr (offset 0) and packed.len (offset 16)
                    ; ldr x3, [x0, 0]
                    ; ldr x4, [x0, 16]

                    // 6. Bounds check: key < len (unsigned)
                    ; cmp x1, x4
                    ; b.hs =>bailout_label

                    // 7. Direct buffer indexing: buffer[key]
                    ; lsl x5, x1, 3
                    ; ldr x6, [x3, x5]

                    // 8. Store to dest
                    ; mov w8, dest_val
                    ; lsl x8, x8, 3
                    ; str x6, [x19, x8]
                );
            }
            crate::tracer::TraceIR::ArraySetPacked { array, key, val, bailout_ip, bailout_stack_top } => {
                let array_val = *array as u32;
                let key_val = *key as u32;
                let val_val = *val as u32;
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));

                dynasm!(ops
                    // 1. Load array into x0
                    ; mov w8, array_val
                    ; lsl x8, x8, 3
                    ; ldr x0, [x19, x8]

                    // Mask with POINTER_MASK (0x0000FFFFFFFFFFFF) to extract *mut PhpArray
                    ; movz x2, 0xFFFF
                    ; movk x2, 0xFFFF, lsl 16
                    ; movk x2, 0xFFFF, lsl 32
                    ; and x0, x0, x2

                    // 2. Check if pointer is non-null
                    ; cbz x0, =>bailout_label

                    // 3. Load key into w1 (zero-extend to x1)
                    ; mov w8, key_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8]
                    ; mov w1, w1

                    // 4. Load val into x6
                    ; mov w8, val_val
                    ; lsl x8, x8, 3
                    ; ldr x6, [x19, x8]

                    // 5. Check is_packed at offset 24
                    ; ldrb w2, [x0, 24]
                    ; cbz w2, =>bailout_label

                    // 6. Load packed.ptr (offset 0) and packed.len (offset 16)
                    ; ldr x3, [x0, 0]
                    ; ldr x4, [x0, 16]

                    // 7. Bounds check: key < len (unsigned)
                    ; cmp x1, x4
                    ; b.hs =>bailout_label

                    // 8. Direct in-place store: buffer[key] = val
                    ; lsl x5, x1, 3
                    ; str x6, [x3, x5]
                );
            }
            crate::tracer::TraceIR::Constant { dest, value } => {
                let val0 = (*value & 0xFFFF) as u32;
                let val1 = ((*value >> 16) & 0xFFFF) as u32;
                let val2 = ((*value >> 32) & 0xFFFF) as u32;
                let val3 = ((*value >> 48) & 0xFFFF) as u32;
                dynasm!(ops
                    ; movz x1, val0
                    ; movk x1, val1, lsl 16
                    ; movk x1, val2, lsl 32
                    ; movk x1, val3, lsl 48
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::GuardType { src, expected_type_tag, bailout_ip, bailout_stack_top } => {
                let src_val = *src as u32;
                let tag_val = *expected_type_tag;
                
                let tag0 = (tag_val & 0xFFFF) as u32;
                let tag1 = ((tag_val >> 16) & 0xFFFF) as u32;
                let tag2 = ((tag_val >> 32) & 0xFFFF) as u32;
                let tag3 = ((tag_val >> 48) & 0xFFFF) as u32;

                let debug_fn_val = debug_guard_type as *const () as u64;
                let fn0 = (debug_fn_val & 0xFFFF) as u32;
                let fn1 = ((debug_fn_val >> 16) & 0xFFFF) as u32;
                let fn2 = ((debug_fn_val >> 32) & 0xFFFF) as u32;
                let fn3 = ((debug_fn_val >> 48) & 0xFFFF) as u32;
                
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));
                
                dynasm!(ops
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[src]
                    
                    ; movz x2, 0 // bottom 16
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0xFFFF, lsl 48 // mask out the bottom 48 bits, keep top 16 bits!
                    // wait, TAG_INT is 0x0004000000000000, which has bits in the 48-63 range.
                    // QNAN is 0x7FF8000000000000, which also has bits in 48-63.
                    // So we mask x1 with 0xFFFF000000000000
                    ; and x1, x1, x2 // x1 = tag
                    
                    ; movz x2, tag0
                    ; movk x2, tag1, lsl 16
                    ; movk x2, tag2, lsl 32
                    ; movk x2, tag3, lsl 48 // x2 = expected_tag
                    
                    ; cmp x1, x2
                    ; b.eq >pass
                    
                    ; mov x0, x2 // arg0: expected
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // arg1: actual_full
                    ; mov x2, src_val as u64 // arg2: src
                    
                    ; movz x10, fn0
                    ; movk x10, fn1, lsl 16
                    ; movk x10, fn2, lsl 32
                    ; movk x10, fn3, lsl 48
                    
                    ; blr x10
                    
                    ; b =>bailout_label
                    ; pass:
                );
            }
            crate::tracer::TraceIR::GuardFunction { src, expected_fn_ptr, bailout_ip, bailout_stack_top } => {
                let src_val = *src as u32;
                let fn_val = *expected_fn_ptr;
                
                let fn0 = (fn_val & 0xFFFF) as u32;
                let fn1 = ((fn_val >> 16) & 0xFFFF) as u32;
                let fn2 = ((fn_val >> 32) & 0xFFFF) as u32;
                let fn3 = ((fn_val >> 48) & 0xFFFF) as u32;
                
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));
                
                dynasm!(ops
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[src]
                    
                    ; movz x2, fn0
                    ; movk x2, fn1, lsl 16
                    ; movk x2, fn2, lsl 32
                    ; movk x2, fn3, lsl 48 // x2 = expected_fn_val
                    
                    ; cmp x1, x2
                    ; b.ne =>bailout_label
                );
            }
            crate::tracer::TraceIR::GuardClosure { src, expected_closure_fn_ptr, bailout_ip, bailout_stack_top } => {
                let src_val = *src as u32;
                let fn_val = *expected_closure_fn_ptr;
                
                let fn0 = (fn_val & 0xFFFF) as u32;
                let fn1 = ((fn_val >> 16) & 0xFFFF) as u32;
                let fn2 = ((fn_val >> 32) & 0xFFFF) as u32;
                let fn3 = ((fn_val >> 48) & 0xFFFF) as u32;
                
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));
                
                dynasm!(ops
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[src] (closure Value)
                    
                    // Extract closure object pointer (mask 0x0000FFFFFFFFFFFF)
                    ; movz x2, 0xFFFF
                    ; movk x2, 0xFFFF, lsl 16
                    ; movk x2, 0xFFFF, lsl 32
                    ; and x0, x1, x2
                    ; cbz x0, =>bailout_label
                    
                    // Load closure.function_ptr at offset 0
                    ; ldr x3, [x0, 0]
                    
                    ; movz x2, fn0
                    ; movk x2, fn1, lsl 16
                    ; movk x2, fn2, lsl 32
                    ; movk x2, fn3, lsl 48 // x2 = expected_function_ptr
                    
                    ; cmp x3, x2
                    ; b.ne =>bailout_label
                );
            }
            crate::tracer::TraceIR::Add { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; add w1, w1, w2
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFC, lsl 48
                    
                    ; orr x1, x1, x2
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::LessThan { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp w1, w2
                    ; cset w1, lt
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::LessThanOrEqual { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp w1, w2
                    ; cset w1, le
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::GreaterThanOrEqual { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp w1, w2
                    ; cset w1, ge
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::GuardCondition { src, expected_bool, bailout_ip, bailout_stack_top } => {
                let src_val = *src as u32;
                let expected_raw: u64 = if *expected_bool {
                    hyperion_core::memory::nan_box::QNAN | hyperion_core::memory::nan_box::TAG_TRUE
                } else {
                    hyperion_core::memory::nan_box::QNAN | hyperion_core::memory::nan_box::TAG_FALSE
                };
                
                let e0 = (expected_raw & 0xFFFF) as u32;
                let e1 = ((expected_raw >> 16) & 0xFFFF) as u32;
                let e2 = ((expected_raw >> 32) & 0xFFFF) as u32;
                let e3 = ((expected_raw >> 48) & 0xFFFF) as u32;
                
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));
                
                dynasm!(ops
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[src]
                    
                    ; movz x2, e0
                    ; movk x2, e1, lsl 16
                    ; movk x2, e2, lsl 32
                    ; movk x2, e3, lsl 48
                    
                    ; cmp x1, x2
                    ; b.ne =>bailout_label
                );
            }

            crate::tracer::TraceIR::Loop { target_ip: _ } => {
                // Native self-branching loop!
                // We jump back to start_label, executing the loop again natively.
                dynasm!(ops
                    ; b =>start_label
                );
            }
            crate::tracer::TraceIR::FetchObjectProperty { dest, obj, name_idx, chunk_ptr, fn_ptr } => {
                let dest_val = *dest as u32;
                let obj_val = *obj as u32;
                let name_val = *name_idx as u32;
                
                let chunk0 = (*chunk_ptr & 0xFFFF) as u32;
                let chunk1 = ((*chunk_ptr >> 16) & 0xFFFF) as u32;
                let chunk2 = ((*chunk_ptr >> 32) & 0xFFFF) as u32;
                let chunk3 = ((*chunk_ptr >> 48) & 0xFFFF) as u32;
                
                let fn0 = (*fn_ptr & 0xFFFF) as u32;
                let fn1 = ((*fn_ptr >> 16) & 0xFFFF) as u32;
                let fn2 = ((*fn_ptr >> 32) & 0xFFFF) as u32;
                let fn3 = ((*fn_ptr >> 48) & 0xFFFF) as u32;
                
                dynasm!(ops
                    ; mov w8, obj_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[obj]
                    
                    // Extract object pointer (mask with POINTER_MASK 0x0000FFFFFFFFFFFF)
                    ; movz x2, 0xFFFF
                    ; movk x2, 0xFFFF, lsl 16
                    ; movk x2, 0xFFFF, lsl 32
                    ; and x0, x1, x2 // x0 = obj_ptr (arg0)
                    
                    ; mov w1, name_val // x1 = name_idx (arg1)
                    
                    ; movz x2, chunk0
                    ; movk x2, chunk1, lsl 16
                    ; movk x2, chunk2, lsl 32
                    ; movk x2, chunk3, lsl 48 // x2 = chunk_ptr (arg2)
                    
                    ; movz x10, fn0
                    ; movk x10, fn1, lsl 16
                    ; movk x10, fn2, lsl 32
                    ; movk x10, fn3, lsl 48
                    
                    ; blr x10 // call jit_fetch_object_property
                    
                    ; mov w8, dest_val
                    ; lsl x8, x8, 3
                    ; str x0, [x19, x8] // stack[dest] = returned Value (x0)
                );
            }
            crate::tracer::TraceIR::SetObjectProperty { dest, obj, val, name_idx, chunk_ptr, fn_ptr } => {
                let dest_val = *dest as u32;
                let obj_val = *obj as u32;
                let val_val = *val as u32;
                let name_val = *name_idx as u32;
                
                let chunk0 = (*chunk_ptr & 0xFFFF) as u32;
                let chunk1 = ((*chunk_ptr >> 16) & 0xFFFF) as u32;
                let chunk2 = ((*chunk_ptr >> 32) & 0xFFFF) as u32;
                let chunk3 = ((*chunk_ptr >> 48) & 0xFFFF) as u32;
                
                let fn0 = (*fn_ptr & 0xFFFF) as u32;
                let fn1 = ((*fn_ptr >> 16) & 0xFFFF) as u32;
                let fn2 = ((*fn_ptr >> 32) & 0xFFFF) as u32;
                let fn3 = ((*fn_ptr >> 48) & 0xFFFF) as u32;
                
                dynasm!(ops
                    ; mov w8, obj_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[obj]
                    
                    // Extract object pointer (mask with POINTER_MASK)
                    ; movz x2, 0xFFFF
                    ; movk x2, 0xFFFF, lsl 16
                    ; movk x2, 0xFFFF, lsl 32
                    ; and x0, x1, x2 // x0 = obj_ptr (arg0)
                    
                    ; mov w8, val_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = val_bits (arg1)
                    
                    ; mov w2, name_val // w2 = name_idx (arg2)
                    
                    ; movz x3, chunk0
                    ; movk x3, chunk1, lsl 16
                    ; movk x3, chunk2, lsl 32
                    ; movk x3, chunk3, lsl 48 // x3 = chunk_ptr (arg3)
                    
                    ; movz x10, fn0
                    ; movk x10, fn1, lsl 16
                    ; movk x10, fn2, lsl 32
                    ; movk x10, fn3, lsl 48
                    
                    ; blr x10 // call jit_set_object_property
                    
                    // Restore val_bits from the original location and write it to dest
                    ; mov w8, val_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = val_bits
                    
                    ; mov w8, dest_val
                    ; lsl x8, x8, 3
                    ; str x1, [x19, x8] // stack[dest] = val_bits
                );
            }
            crate::tracer::TraceIR::Bailout { bailout_ip, bailout_stack_top } => {
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));
                dynasm!(ops
                    ; b =>bailout_label
                );
            }
            crate::tracer::TraceIR::Subtract { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; sub w1, w1, w2
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFC, lsl 48
                    
                    ; orr x1, x1, x2
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::Multiply { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; mul w1, w1, w2
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFC, lsl 48
                    
                    ; orr x1, x1, x2
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::Divide { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; sdiv w1, w1, w2
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFC, lsl 48
                    
                    ; orr x1, x1, x2
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::GreaterThan { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp w1, w2
                    ; cset w1, gt
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::Equals { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp x1, x2 // compare full 64-bit bits
                    ; cset w1, eq
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::NotEquals { dest, left, right } => {
                load_gp_slot(ops, *left, 1, reg_alloc);
                load_gp_slot(ops, *right, 2, reg_alloc);
                dynasm!(ops
                    ; cmp x1, x2
                    ; cset w1, ne
                    
                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::Concat { dest, left, right, fn_ptr } => {
                let dest_val = *dest as u32;
                let left_val = *left as u32;
                let right_val = *right as u32;
                let fn_val = *fn_ptr;
                
                let val0 = (fn_val & 0xFFFF) as u32;
                let val1 = ((fn_val >> 16) & 0xFFFF) as u32;
                let val2 = ((fn_val >> 32) & 0xFFFF) as u32;
                let val3 = ((fn_val >> 48) & 0xFFFF) as u32;

                dynasm!(ops
                    ; mov w8, left_val
                    ; lsl x8, x8, 3
                    ; ldr x0, [x19, x8]
                    
                    ; mov w8, right_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8]
                    
                    ; movz x2, val0
                    ; movk x2, val1, lsl 16
                    ; movk x2, val2, lsl 32
                    ; movk x2, val3, lsl 48
                    
                    ; blr x2

                    ; mov w8, dest_val
                    ; lsl x8, x8, 3
                    ; str x0, [x19, x8]
                );
            }
            crate::tracer::TraceIR::GuardFloat { src, bailout_ip, bailout_stack_top } => {
                let src_val = *src as u32;
                let bailout_label = ops.new_dynamic_label();
                bailouts.push((bailout_label, *bailout_ip as u32, *bailout_stack_top as u32));

                dynasm!(ops
                    ; mov w8, src_val
                    ; lsl x8, x8, 3
                    ; ldr x1, [x19, x8] // x1 = stack[src]

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FF8, lsl 48 // x2 = QNAN mask

                    ; and x3, x1, x2
                    ; cmp x3, x2
                    ; b.eq =>bailout_label // If (val & QNAN) == QNAN, it's not a float -> bailout!
                );
            }
            crate::tracer::TraceIR::FloatAdd { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fadd d0, d0, d1
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::FloatSubtract { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fsub d0, d0, d1
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::FloatMultiply { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fmul d0, d0, d1
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::FloatDivide { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fdiv d0, d0, d1
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::FloatNegate { dest, src } => {
                load_fp_slot(ops, *src, 0, reg_alloc);
                dynasm!(ops
                    ; fneg d0, d0
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::IntToFloat { dest, src } => {
                load_gp_slot(ops, *src, 1, reg_alloc);
                dynasm!(ops
                    ; sxtw x1, w1 // sign-extend the 32-bit int payload
                    ; scvtf d0, x1 // d0 = (double)x1
                );
                store_fp_slot(ops, *dest, 0, reg_alloc);
            }
            crate::tracer::TraceIR::FloatToInt { dest, src } => {
                load_fp_slot(ops, *src, 0, reg_alloc);
                dynasm!(ops
                    ; fcvtzs w1, d0 // w1 = (int32)d0

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFC, lsl 48 // TAG_INT mask

                    ; orr x1, x1, x2
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatLessThan { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, mi

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatGreaterThan { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, gt

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatLessThanOrEqual { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, ls

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatGreaterThanOrEqual { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, ge

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatEquals { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, eq

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            crate::tracer::TraceIR::FloatNotEquals { dest, left, right } => {
                load_fp_slot(ops, *left, 0, reg_alloc);
                load_fp_slot(ops, *right, 1, reg_alloc);
                dynasm!(ops
                    ; fcmp d0, d1
                    ; cset w1, ne

                    ; movz x2, 0
                    ; movk x2, 0, lsl 16
                    ; movk x2, 0, lsl 32
                    ; movk x2, 0x7FFA, lsl 48
                    ; and w3, w1, 1
                    ; lsl x3, x3, 48
                    ; add x1, x2, x3
                );
                store_gp_slot(ops, *dest, 1, reg_alloc);
            }
            _ => {}
        }
    }

#[cfg(target_arch = "aarch64")]
pub fn compile_trace(trace: &crate::tracer::OptimizedTrace) -> CompiledTrace {
    let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();
    
    // We assume the C ABI: x0 = pointer to VM stack (Value array)
    // Return: x0 = Tag, x1 = Payload (IP)
    dynasm!(ops
        ; .arch aarch64
    );

    let mut bailouts = Vec::new();
    let start_label = ops.new_dynamic_label();

    dynasm!(ops
        // PROLOGUE: Save callee-saved registers x19-x28 and d8-d15 (160 bytes aligned)
        ; sub sp, sp, 160
        ; stp x19, x20, [sp, 0]
        ; stp x21, x22, [sp, 16]
        ; stp x23, x24, [sp, 32]
        ; stp x25, x26, [sp, 48]
        ; stp x27, x28, [sp, 64]
        ; str x30, [sp, 80]
        ; stp d8, d9, [sp, 96]
        ; stp d10, d11, [sp, 112]
        ; stp d12, d13, [sp, 128]
        ; stp d14, d15, [sp, 144]

        // Move VM stack pointer to x19, out_result pointer to x20
        ; mov x19, x0
        ; mov x20, x1
    );

    // 1. PRE-HEADER: Hoisted invariant guards (executed once on loop entry)
    for ir in &trace.preheader {
        emit_ir(&mut ops, ir, &mut bailouts, start_label, &trace.reg_alloc);
    }

    // 2. PRE-HEADER: Warm up allocated registers from VM stack
    for (&slot, &reg) in &trace.reg_alloc.gp_regs {
        let slot_val = slot as u32;
        match reg {
            21 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x21, [x19, x8]),
            22 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x22, [x19, x8]),
            23 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x23, [x19, x8]),
            24 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x24, [x19, x8]),
            25 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x25, [x19, x8]),
            26 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr x26, [x19, x8]),
            _ => {}
        }
    }

    for (&slot, &reg) in &trace.reg_alloc.fp_regs {
        let slot_val = slot as u32;
        match reg {
            8 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d8, [x19, x8]),
            9 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d9, [x19, x8]),
            10 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d10, [x19, x8]),
            11 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d11, [x19, x8]),
            12 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d12, [x19, x8]),
            13 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; ldr d13, [x19, x8]),
            _ => {}
        }
    }

    // 3. LOOP HEADER LABEL
    dynasm!(ops
        ; =>start_label
    );

    // 4. LOOP BODY: Executed repeatedly
    for ir in &trace.body {
        emit_ir(&mut ops, ir, &mut bailouts, start_label, &trace.reg_alloc);
    }

    for (label, ip, stack_top) in bailouts {
        let ip0 = ip & 0xFFFF;
        let ip1 = (ip >> 16) & 0xFFFF;
        let st0 = stack_top & 0xFFFF;
        let st1 = (stack_top >> 16) & 0xFFFF;
        
        dynasm!(ops
            ; =>label
        );

        // Flush all allocated GP registers back to VM stack before bailout
        for (&slot, &reg) in &trace.reg_alloc.gp_regs {
            let slot_val = slot as u32;
            match reg {
                21 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x21, [x19, x8]),
                22 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x22, [x19, x8]),
                23 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x23, [x19, x8]),
                24 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x24, [x19, x8]),
                25 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x25, [x19, x8]),
                26 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str x26, [x19, x8]),
                _ => {}
            }
        }

        // Flush all allocated FP registers back to VM stack before bailout
        for (&slot, &reg) in &trace.reg_alloc.fp_regs {
            let slot_val = slot as u32;
            match reg {
                8 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d8, [x19, x8]),
                9 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d9, [x19, x8]),
                10 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d10, [x19, x8]),
                11 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d11, [x19, x8]),
                12 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d12, [x19, x8]),
                13 => dynasm!(ops; mov w8, slot_val; lsl x8, x8, 3; str d13, [x19, x8]),
                _ => {}
            }
        }

        dynasm!(ops
            ; mov x8, 1 // Tag 1 = Deoptimized
            ; str x8, [x20]
            ; movz x8, ip0
            ; movk x8, ip1, lsl 16
            ; str x8, [x20, 8] // payload_ip
            ; movz x8, st0
            ; movk x8, st1, lsl 16
            ; str x8, [x20, 16] // stack_top
            
            // EPILOGUE: Restore callee-saved registers
            ; ldp d14, d15, [sp, 144]
            ; ldp d12, d13, [sp, 128]
            ; ldp d10, d11, [sp, 112]
            ; ldp d8, d9, [sp, 96]
            ; ldr x30, [sp, 80]
            ; ldp x27, x28, [sp, 64]
            ; ldp x25, x26, [sp, 48]
            ; ldp x23, x24, [sp, 32]
            ; ldp x21, x22, [sp, 16]
            ; ldp x19, x20, [sp, 0]
            ; add sp, sp, 160
            
            ; ret
        );
    }

    // Default Fallback Return if Trace finishes linearly without looping
    dynasm!(ops
        ; str xzr, [x20] // Tag 0 = Finished
        ; ldp d14, d15, [sp, 144]
        ; ldp d12, d13, [sp, 128]
        ; ldp d10, d11, [sp, 112]
        ; ldp d8, d9, [sp, 96]
        ; ldr x30, [sp, 80]
        ; ldp x27, x28, [sp, 64]
        ; ldp x25, x26, [sp, 48]
        ; ldp x23, x24, [sp, 32]
        ; ldp x21, x22, [sp, 16]
        ; ldp x19, x20, [sp, 0]
        ; add sp, sp, 160
        ; ret
    );

    let buf = ops.finalize().unwrap();
    let execute_fn: extern "C" fn(*mut hyperion_core::memory::nan_box::Value, *mut JitReturn) = unsafe {
        mem::transmute(buf.ptr(dynasmrt::AssemblyOffset(0)))
    };

    CompiledTrace {
        buffer: buf,
        execute_fn,
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn compile_trace(_trace: &crate::tracer::OptimizedTrace) -> CompiledTrace {
    // Dummy implementation for non-aarch64 platforms (x86_64, etc.)
    extern "C" fn dummy_exec(_stack: *mut hyperion_core::memory::nan_box::Value, out_result: *mut JitReturn) {
        if !out_result.is_null() {
            unsafe {
                (*out_result).tag = 1;
                (*out_result).payload_ip = 0;
                (*out_result).stack_top = 0;
            }
        }
    }

    let mut ops = dynasmrt::x64::Assembler::new().unwrap();
    let buf = ops.finalize().unwrap();

    CompiledTrace {
        buffer: buf,
        execute_fn: dummy_exec,
    }
}

