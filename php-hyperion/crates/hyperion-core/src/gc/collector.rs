use rustc_hash::FxHashSet;
use crate::memory::nan_box::Value;
use crate::types::array::PhpArray;
use crate::types::object::PhpObject;
use crate::types::reference::PhpRef;
use crate::gc::arena::GcArena;

use std::sync::atomic::{AtomicPtr, Ordering};

pub type ClosureVisitor = fn(*mut (), &mut dyn FnMut(&Value));
static CLOSURE_VISITOR: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

pub fn set_closure_visitor(visitor: ClosureVisitor) {
    CLOSURE_VISITOR.store(visitor as *mut (), Ordering::SeqCst);
}

pub struct GcCollector;

impl GcCollector {
    pub fn mark_value(val: &Value, marked: &mut FxHashSet<*mut ()>) {
        if let Some(ptr) = val.as_array_ptr() {
            if marked.insert(ptr) {
                let arr = unsafe { &*(ptr as *const PhpArray) };
                for v in &arr.packed {
                    Self::mark_value(v, marked);
                }
                for v in arr.elements.values() {
                    Self::mark_value(v, marked);
                }
            }
        } else if let Some(ptr) = val.as_object_ptr() {
            if marked.insert(ptr) {
                let obj = unsafe { &*(ptr as *const PhpObject) };
                for v in obj.properties.values() {
                    Self::mark_value(v, marked);
                }
            }
        } else if let Some(ptr) = val.as_string_ptr() {
            marked.insert(ptr);
        } else if let Some(ptr) = val.as_ref_ptr() {
            if marked.insert(ptr) {
                let r = unsafe { &*(ptr as *const PhpRef) };
                Self::mark_value(&r.get(), marked);
            }
        } else if let Some(ptr) = val.as_closure_ptr() {
            if marked.insert(ptr) {
                let visitor_ptr = CLOSURE_VISITOR.load(Ordering::Relaxed);
                if !visitor_ptr.is_null() {
                    let visitor: ClosureVisitor = unsafe { std::mem::transmute(visitor_ptr) };
                    visitor(ptr, &mut |child| {
                        Self::mark_value(child, marked);
                    });
                }
            }
        }
    }

    pub fn collect(arena: &GcArena, from_idx: usize, roots: &[&[Value]]) -> usize {
        let mut marked = FxHashSet::default();
        for root_slice in roots {
            for val in *root_slice {
                Self::mark_value(val, &mut marked);
            }
        }

        arena.sweep(from_idx, &marked)
    }
}
