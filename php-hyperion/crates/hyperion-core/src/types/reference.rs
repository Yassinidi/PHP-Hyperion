//! PHP reference cells (`&$x`).
//!
//! PHP variables are slots holding values. A reference makes two or more slots
//! point at *one* shared slot: writing through any alias is visible to all of
//! them. That indirection is what this cell provides.
//!
//! A `PhpRef` is bump-allocated in the request arena like every other heap
//! type. It holds a plain `Value`, so it needs no `Drop` — the arena reclaims
//! it wholesale at request end.
//!
//! References deliberately do NOT nest: binding a reference to a reference
//! shares the existing cell rather than wrapping it again, so a single
//! `deref` hop always reaches the value.

use crate::memory::nan_box::Value;

#[derive(Debug)]
pub struct PhpRef {
    pub value: Value,
}

impl PhpRef {
    #[inline]
    pub fn new(value: Value) -> Self {
        // Storing a ref inside a ref would create a chain that `deref`'s
        // single hop could not resolve; collapse to the pointee instead.
        Self {
            value: value.deref(),
        }
    }

    #[inline]
    pub fn get(&self) -> Value {
        self.value
    }

    #[inline]
    pub fn set(&mut self, value: Value) {
        self.value = value.deref();
    }
}
