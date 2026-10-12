use crate::tracer::{OptimizedTrace, TraceIR};

pub struct Optimizer;

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Optimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, trace: Vec<TraceIR>) -> OptimizedTrace {
        use std::collections::{HashMap, HashSet};

        // Pass 1: Constant Folding
        let mut constants = HashMap::new();
        let mut folded_trace = Vec::new();

        for op in trace {
            match op {
                TraceIR::Constant { dest, value } => {
                    constants.insert(dest, value);
                    folded_trace.push(op);
                }
                TraceIR::Add { dest, left, right } => {
                    if let (Some(&l), Some(&r)) = (constants.get(&left), constants.get(&right)) {
                        let l_val = (l & 0xFFFFFFFF) as i32;
                        let r_val = (r & 0xFFFFFFFF) as i32;
                        let sum = l_val.wrapping_add(r_val);
                        let new_val = 0x0004000000000000 | (sum as u32 as u64); // TAG_INT | sum
                        constants.insert(dest, new_val);
                        folded_trace.push(TraceIR::Constant { dest, value: new_val });
                    } else {
                        constants.remove(&dest);
                        folded_trace.push(op);
                    }
                }
                TraceIR::Subtract { dest, left, right } => {
                    if let (Some(&l), Some(&r)) = (constants.get(&left), constants.get(&right)) {
                        let l_val = (l & 0xFFFFFFFF) as i32;
                        let r_val = (r & 0xFFFFFFFF) as i32;
                        let sum = l_val.wrapping_sub(r_val);
                        let new_val = 0x0004000000000000 | (sum as u32 as u64);
                        constants.insert(dest, new_val);
                        folded_trace.push(TraceIR::Constant { dest, value: new_val });
                    } else {
                        constants.remove(&dest);
                        folded_trace.push(op);
                    }
                }
                TraceIR::Multiply { dest, left, right } => {
                    if let (Some(&l), Some(&r)) = (constants.get(&left), constants.get(&right)) {
                        let l_val = (l & 0xFFFFFFFF) as i32;
                        let r_val = (r & 0xFFFFFFFF) as i32;
                        let sum = l_val.wrapping_mul(r_val);
                        let new_val = 0x0004000000000000 | (sum as u32 as u64);
                        constants.insert(dest, new_val);
                        folded_trace.push(TraceIR::Constant { dest, value: new_val });
                    } else {
                        constants.remove(&dest);
                        folded_trace.push(op);
                    }
                }
                // Invalidations
                TraceIR::SetLocal { dest, .. }
                | TraceIR::GetLocal { dest, .. }
                | TraceIR::Divide { dest, .. }
                | TraceIR::LessThan { dest, .. }
                | TraceIR::GreaterThan { dest, .. }
                | TraceIR::LessThanOrEqual { dest, .. }
                | TraceIR::GreaterThanOrEqual { dest, .. }
                | TraceIR::Equals { dest, .. }
                | TraceIR::NotEquals { dest, .. }
                | TraceIR::FloatAdd { dest, .. }
                | TraceIR::FloatSubtract { dest, .. }
                | TraceIR::FloatMultiply { dest, .. }
                | TraceIR::FloatDivide { dest, .. }
                | TraceIR::FloatNegate { dest, .. }
                | TraceIR::FloatLessThan { dest, .. }
                | TraceIR::FloatGreaterThan { dest, .. }
                | TraceIR::FloatLessThanOrEqual { dest, .. }
                | TraceIR::FloatGreaterThanOrEqual { dest, .. }
                | TraceIR::FloatEquals { dest, .. }
                | TraceIR::FloatNotEquals { dest, .. }
                | TraceIR::IntToFloat { dest, .. }
                | TraceIR::FloatToInt { dest, .. }
                | TraceIR::Concat { dest, .. }
                | TraceIR::FetchArrayElement { dest, .. }
                | TraceIR::ArrayGetPacked { dest, .. }
                | TraceIR::FetchObjectProperty { dest, .. } => {
                    constants.remove(&dest);
                    folded_trace.push(op);
                }
                _ => {
                    folded_trace.push(op);
                }
            }
        }

        // Pass 2: Dead Code Elimination (DCE)
        let mut dce_trace = Vec::new();
        let mut overwritten_slots = HashSet::new();

        for op in folded_trace.into_iter().rev() {
            let is_dead = match &op {
                TraceIR::SetLocal { dest, src } if dest == src => true,
                TraceIR::SetLocal { dest, .. }
                | TraceIR::GetLocal { dest, .. }
                | TraceIR::Add { dest, .. }
                | TraceIR::Subtract { dest, .. }
                | TraceIR::Multiply { dest, .. }
                | TraceIR::Divide { dest, .. }
                | TraceIR::LessThan { dest, .. }
                | TraceIR::GreaterThan { dest, .. }
                | TraceIR::LessThanOrEqual { dest, .. }
                | TraceIR::GreaterThanOrEqual { dest, .. }
                | TraceIR::Equals { dest, .. }
                | TraceIR::NotEquals { dest, .. }
                | TraceIR::FloatAdd { dest, .. }
                | TraceIR::FloatSubtract { dest, .. }
                | TraceIR::FloatMultiply { dest, .. }
                | TraceIR::FloatDivide { dest, .. }
                | TraceIR::FloatNegate { dest, .. }
                | TraceIR::FloatLessThan { dest, .. }
                | TraceIR::FloatGreaterThan { dest, .. }
                | TraceIR::FloatLessThanOrEqual { dest, .. }
                | TraceIR::FloatGreaterThanOrEqual { dest, .. }
                | TraceIR::FloatEquals { dest, .. }
                | TraceIR::FloatNotEquals { dest, .. }
                | TraceIR::IntToFloat { dest, .. }
                | TraceIR::FloatToInt { dest, .. }
                | TraceIR::Concat { dest, .. }
                | TraceIR::FetchArrayElement { dest, .. }
                | TraceIR::ArrayGetPacked { dest, .. }
                | TraceIR::FetchObjectProperty { dest, .. }
                | TraceIR::Constant { dest, .. } => overwritten_slots.contains(dest),
                _ => false,
            };

            if is_dead {
                continue;
            }

            match &op {
                TraceIR::Add { left, right, .. }
                | TraceIR::Subtract { left, right, .. }
                | TraceIR::Multiply { left, right, .. }
                | TraceIR::Divide { left, right, .. }
                | TraceIR::LessThan { left, right, .. }
                | TraceIR::GreaterThan { left, right, .. }
                | TraceIR::LessThanOrEqual { left, right, .. }
                | TraceIR::GreaterThanOrEqual { left, right, .. }
                | TraceIR::Equals { left, right, .. }
                | TraceIR::NotEquals { left, right, .. }
                | TraceIR::FloatAdd { left, right, .. }
                | TraceIR::FloatSubtract { left, right, .. }
                | TraceIR::FloatMultiply { left, right, .. }
                | TraceIR::FloatDivide { left, right, .. }
                | TraceIR::FloatLessThan { left, right, .. }
                | TraceIR::FloatGreaterThan { left, right, .. }
                | TraceIR::FloatLessThanOrEqual { left, right, .. }
                | TraceIR::FloatGreaterThanOrEqual { left, right, .. }
                | TraceIR::FloatEquals { left, right, .. }
                | TraceIR::FloatNotEquals { left, right, .. }
                | TraceIR::Concat { left, right, .. } => {
                    overwritten_slots.remove(left);
                    overwritten_slots.remove(right);
                }
                TraceIR::SetLocal { src, .. }
                | TraceIR::GetLocal { src, .. }
                | TraceIR::FloatNegate { src, .. }
                | TraceIR::IntToFloat { src, .. }
                | TraceIR::FloatToInt { src, .. }
                | TraceIR::GuardType { src, .. }
                | TraceIR::GuardFloat { src, .. }
                | TraceIR::GuardFunction { src, .. }
                | TraceIR::GuardClosure { src, .. }
                | TraceIR::GuardCondition { src, .. }
                | TraceIR::Return { src, .. } => {
                    overwritten_slots.remove(src);
                }
                TraceIR::FetchArrayElement { array, key, .. }
                | TraceIR::ArrayGetPacked { array, key, .. } => {
                    overwritten_slots.remove(array);
                    overwritten_slots.remove(key);
                }
                TraceIR::ArraySetPacked { array, key, val, .. } => {
                    overwritten_slots.remove(array);
                    overwritten_slots.remove(key);
                    overwritten_slots.remove(val);
                }
                TraceIR::FetchObjectProperty { obj, .. } => {
                    overwritten_slots.remove(obj);
                }
                TraceIR::SetObjectProperty { obj, val, .. } => {
                    overwritten_slots.remove(obj);
                    overwritten_slots.remove(val);
                }
                _ => {}
            }

            match &op {
                TraceIR::SetLocal { dest, .. }
                | TraceIR::GetLocal { dest, .. }
                | TraceIR::Add { dest, .. }
                | TraceIR::Subtract { dest, .. }
                | TraceIR::Multiply { dest, .. }
                | TraceIR::Divide { dest, .. }
                | TraceIR::LessThan { dest, .. }
                | TraceIR::GreaterThan { dest, .. }
                | TraceIR::LessThanOrEqual { dest, .. }
                | TraceIR::GreaterThanOrEqual { dest, .. }
                | TraceIR::Equals { dest, .. }
                | TraceIR::NotEquals { dest, .. }
                | TraceIR::FloatAdd { dest, .. }
                | TraceIR::FloatSubtract { dest, .. }
                | TraceIR::FloatMultiply { dest, .. }
                | TraceIR::FloatDivide { dest, .. }
                | TraceIR::FloatNegate { dest, .. }
                | TraceIR::FloatLessThan { dest, .. }
                | TraceIR::FloatGreaterThan { dest, .. }
                | TraceIR::FloatLessThanOrEqual { dest, .. }
                | TraceIR::FloatGreaterThanOrEqual { dest, .. }
                | TraceIR::FloatEquals { dest, .. }
                | TraceIR::FloatNotEquals { dest, .. }
                | TraceIR::IntToFloat { dest, .. }
                | TraceIR::FloatToInt { dest, .. }
                | TraceIR::Concat { dest, .. }
                | TraceIR::FetchArrayElement { dest, .. }
                | TraceIR::ArrayGetPacked { dest, .. }
                | TraceIR::FetchObjectProperty { dest, .. }
                | TraceIR::Constant { dest, .. } => {
                    overwritten_slots.insert(*dest);
                }
                _ => {}
            }

            dce_trace.push(op);
        }

        dce_trace.reverse();

        // Pass 3: Loop-Invariant Code Motion (LICM) & Invariant Guard Hoisting
        let mut modified_slots = HashSet::new();
        for op in &dce_trace {
            match op {
                TraceIR::SetLocal { dest, .. }
                | TraceIR::GetLocal { dest, .. }
                | TraceIR::Add { dest, .. }
                | TraceIR::Subtract { dest, .. }
                | TraceIR::Multiply { dest, .. }
                | TraceIR::Divide { dest, .. }
                | TraceIR::LessThan { dest, .. }
                | TraceIR::GreaterThan { dest, .. }
                | TraceIR::LessThanOrEqual { dest, .. }
                | TraceIR::GreaterThanOrEqual { dest, .. }
                | TraceIR::Equals { dest, .. }
                | TraceIR::NotEquals { dest, .. }
                | TraceIR::FloatAdd { dest, .. }
                | TraceIR::FloatSubtract { dest, .. }
                | TraceIR::FloatMultiply { dest, .. }
                | TraceIR::FloatDivide { dest, .. }
                | TraceIR::FloatNegate { dest, .. }
                | TraceIR::FloatLessThan { dest, .. }
                | TraceIR::FloatGreaterThan { dest, .. }
                | TraceIR::FloatLessThanOrEqual { dest, .. }
                | TraceIR::FloatGreaterThanOrEqual { dest, .. }
                | TraceIR::FloatEquals { dest, .. }
                | TraceIR::FloatNotEquals { dest, .. }
                | TraceIR::IntToFloat { dest, .. }
                | TraceIR::FloatToInt { dest, .. }
                | TraceIR::Concat { dest, .. }
                | TraceIR::FetchArrayElement { dest, .. }
                | TraceIR::ArrayGetPacked { dest, .. }
                | TraceIR::FetchObjectProperty { dest, .. }
                | TraceIR::SetObjectProperty { dest, .. }
                | TraceIR::Constant { dest, .. } => {
                    modified_slots.insert(*dest);
                }
                _ => {}
            }
        }

        let mut preheader = Vec::new();
        let mut body = Vec::new();
        let mut hoisted_guards = HashSet::new();
        let mut active_guards = HashSet::new();

        for op in dce_trace {
            match op {
                TraceIR::GuardType { src, expected_type_tag, bailout_ip, bailout_stack_top } => {
                    if !modified_slots.contains(&src) {
                        if hoisted_guards.insert((src, expected_type_tag)) {
                            preheader.push(TraceIR::GuardType {
                                src,
                                expected_type_tag,
                                bailout_ip,
                                bailout_stack_top,
                            });
                        }
                    } else {
                        if active_guards.insert((src, expected_type_tag)) {
                            body.push(op);
                        }
                    }
                }
                TraceIR::GuardFloat { src, bailout_ip, bailout_stack_top } => {
                    if !modified_slots.contains(&src) {
                        if hoisted_guards.insert((src, 0x7FF8000000000000)) {
                            preheader.push(TraceIR::GuardFloat {
                                src,
                                bailout_ip,
                                bailout_stack_top,
                            });
                        }
                    } else {
                        if active_guards.insert((src, 0x7FF8000000000000)) {
                            body.push(op);
                        }
                    }
                }
                TraceIR::GuardFunction { src, expected_fn_ptr, bailout_ip, bailout_stack_top } => {
                    if !modified_slots.contains(&src) {
                        if hoisted_guards.insert((src, expected_fn_ptr)) {
                            preheader.push(TraceIR::GuardFunction {
                                src,
                                expected_fn_ptr,
                                bailout_ip,
                                bailout_stack_top,
                            });
                        }
                    } else {
                        if active_guards.insert((src, expected_fn_ptr)) {
                            body.push(op);
                        }
                    }
                }
                TraceIR::GuardClosure { src, expected_closure_fn_ptr, bailout_ip, bailout_stack_top } => {
                    if !modified_slots.contains(&src) {
                        if hoisted_guards.insert((src, expected_closure_fn_ptr)) {
                            preheader.push(TraceIR::GuardClosure {
                                src,
                                expected_closure_fn_ptr,
                                bailout_ip,
                                bailout_stack_top,
                            });
                        }
                    } else {
                        if active_guards.insert((src, expected_closure_fn_ptr)) {
                            body.push(op);
                        }
                    }
                }
                _ => {
                    match &op {
                        TraceIR::SetLocal { dest, .. }
                        | TraceIR::GetLocal { dest, .. }
                        | TraceIR::Add { dest, .. }
                        | TraceIR::Subtract { dest, .. }
                        | TraceIR::Multiply { dest, .. }
                        | TraceIR::Divide { dest, .. }
                        | TraceIR::FloatAdd { dest, .. }
                        | TraceIR::FloatSubtract { dest, .. }
                        | TraceIR::FloatMultiply { dest, .. }
                        | TraceIR::FloatDivide { dest, .. }
                        | TraceIR::IntToFloat { dest, .. }
                        | TraceIR::FloatToInt { dest, .. }
                        | TraceIR::Constant { dest, .. } => {
                            active_guards.retain(|&(s, _)| s != *dest);
                        }
                        _ => {}
                    }
                    body.push(op);
                }
            }
        }

        // Pass 4: Linear Scan Register Allocation
        let mut float_slots: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for op in &body {
            match op {
                TraceIR::FloatAdd { dest, left, right }
                | TraceIR::FloatSubtract { dest, left, right }
                | TraceIR::FloatMultiply { dest, left, right }
                | TraceIR::FloatDivide { dest, left, right } => {
                    float_slots.insert(*dest);
                    float_slots.insert(*left);
                    float_slots.insert(*right);
                }
                TraceIR::FloatNegate { dest, src } => {
                    float_slots.insert(*dest);
                    float_slots.insert(*src);
                }
                TraceIR::FloatLessThan { left, right, .. }
                | TraceIR::FloatGreaterThan { left, right, .. }
                | TraceIR::FloatLessThanOrEqual { left, right, .. }
                | TraceIR::FloatGreaterThanOrEqual { left, right, .. }
                | TraceIR::FloatEquals { left, right, .. }
                | TraceIR::FloatNotEquals { left, right, .. } => {
                    float_slots.insert(*left);
                    float_slots.insert(*right);
                }
                TraceIR::IntToFloat { dest, .. } => {
                    float_slots.insert(*dest);
                }
                TraceIR::FloatToInt { src, .. } => {
                    float_slots.insert(*src);
                }
                TraceIR::GuardFloat { src, .. } => {
                    float_slots.insert(*src);
                }
                _ => {}
            }
        }

        // Propagate floatness through GetLocal / SetLocal
        for _ in 0..3 {
            for op in &body {
                match op {
                    TraceIR::GetLocal { dest, src } | TraceIR::SetLocal { dest, src } => {
                        if float_slots.contains(src) || float_slots.contains(dest) {
                            float_slots.insert(*src);
                            float_slots.insert(*dest);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut gp_freq: HashMap<usize, usize> = HashMap::new();
        let mut fp_freq: HashMap<usize, usize> = HashMap::new();

        for op in &body {
            match op {
                TraceIR::FloatAdd { dest, left, right }
                | TraceIR::FloatSubtract { dest, left, right }
                | TraceIR::FloatMultiply { dest, left, right }
                | TraceIR::FloatDivide { dest, left, right } => {
                    *fp_freq.entry(*dest).or_insert(0) += 2;
                    *fp_freq.entry(*left).or_insert(0) += 1;
                    *fp_freq.entry(*right).or_insert(0) += 1;
                }
                TraceIR::FloatNegate { dest, src } => {
                    *fp_freq.entry(*dest).or_insert(0) += 2;
                    *fp_freq.entry(*src).or_insert(0) += 1;
                }
                TraceIR::FloatLessThan { dest, left, right }
                | TraceIR::FloatGreaterThan { dest, left, right }
                | TraceIR::FloatLessThanOrEqual { dest, left, right }
                | TraceIR::FloatGreaterThanOrEqual { dest, left, right }
                | TraceIR::FloatEquals { dest, left, right }
                | TraceIR::FloatNotEquals { dest, left, right } => {
                    *gp_freq.entry(*dest).or_insert(0) += 2;
                    *fp_freq.entry(*left).or_insert(0) += 1;
                    *fp_freq.entry(*right).or_insert(0) += 1;
                }
                TraceIR::IntToFloat { dest, src } => {
                    *fp_freq.entry(*dest).or_insert(0) += 2;
                    *gp_freq.entry(*src).or_insert(0) += 1;
                }
                TraceIR::FloatToInt { dest, src } => {
                    *gp_freq.entry(*dest).or_insert(0) += 2;
                    *fp_freq.entry(*src).or_insert(0) += 1;
                }
                TraceIR::GuardFloat { src, .. } => {
                    *fp_freq.entry(*src).or_insert(0) += 1;
                }
                TraceIR::Add { dest, left, right }
                | TraceIR::Subtract { dest, left, right }
                | TraceIR::Multiply { dest, left, right }
                | TraceIR::Divide { dest, left, right }
                | TraceIR::LessThan { dest, left, right }
                | TraceIR::GreaterThan { dest, left, right }
                | TraceIR::LessThanOrEqual { dest, left, right }
                | TraceIR::GreaterThanOrEqual { dest, left, right }
                | TraceIR::Equals { dest, left, right }
                | TraceIR::NotEquals { dest, left, right } => {
                    *gp_freq.entry(*dest).or_insert(0) += 2;
                    *gp_freq.entry(*left).or_insert(0) += 1;
                    *gp_freq.entry(*right).or_insert(0) += 1;
                }
                TraceIR::GetLocal { dest, src } | TraceIR::SetLocal { dest, src } => {
                    if float_slots.contains(src) || float_slots.contains(dest) {
                        *fp_freq.entry(*dest).or_insert(0) += 2;
                        *fp_freq.entry(*src).or_insert(0) += 1;
                    } else {
                        *gp_freq.entry(*dest).or_insert(0) += 2;
                        *gp_freq.entry(*src).or_insert(0) += 1;
                    }
                }
                TraceIR::Constant { dest, .. } => {
                    if float_slots.contains(dest) {
                        *fp_freq.entry(*dest).or_insert(0) += 1;
                    } else {
                        *gp_freq.entry(*dest).or_insert(0) += 1;
                    }
                }
                TraceIR::GuardType { src, .. }
                | TraceIR::GuardCondition { src, .. }
                | TraceIR::GuardFunction { src, .. }
                | TraceIR::GuardClosure { src, .. } => {
                    *gp_freq.entry(*src).or_insert(0) += 1;
                }
                TraceIR::ArrayGetPacked { dest, array, key, .. } => {
                    *gp_freq.entry(*dest).or_insert(0) += 2;
                    *gp_freq.entry(*array).or_insert(0) += 1;
                    *gp_freq.entry(*key).or_insert(0) += 1;
                }
                TraceIR::ArraySetPacked { array, key, val, .. } => {
                    *gp_freq.entry(*array).or_insert(0) += 1;
                    *gp_freq.entry(*key).or_insert(0) += 1;
                    *gp_freq.entry(*val).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        // Available hardware registers:
        // GP: x21..x26 (IDs: 21..26)
        // FP: d8..d13 (IDs: 8..13)
        let mut gp_sorted: Vec<(usize, usize)> = gp_freq.into_iter().collect();
        gp_sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let mut fp_sorted: Vec<(usize, usize)> = fp_freq.into_iter().collect();
        fp_sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let gp_available = [21u8, 22, 23, 24, 25, 26];
        let fp_available = [8u8, 9, 10, 11, 12, 13];

        let mut fp_regs = HashMap::new();
        for (i, (slot, _count)) in fp_sorted.into_iter().filter(|(s, _)| float_slots.contains(s)).take(fp_available.len()).enumerate() {
            fp_regs.insert(slot, fp_available[i]);
        }

        let mut gp_regs = HashMap::new();
        for (i, (slot, _count)) in gp_sorted.into_iter().filter(|(s, _)| !float_slots.contains(s)).take(gp_available.len()).enumerate() {
            gp_regs.insert(slot, gp_available[i]);
        }

        let reg_alloc = crate::tracer::RegisterAllocation::default();

        OptimizedTrace {
            preheader,
            body,
            reg_alloc,
        }
    }
}
