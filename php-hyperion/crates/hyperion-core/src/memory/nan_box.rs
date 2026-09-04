//! NaN Boxing Implementation
//! 
//! IEEE 754 Double-Precision (64-bit) floats have the following layout:
//! S EEEEEEEEEEE FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
//! 1   11 bits                       52 bits
//! 
//! When all Exponent bits are 1, it's a NaN.
//! Quiet NaN (QNaN) has the highest Fraction bit as 1.
//! This leaves 51 bits of the Fraction for payload.
//! 
//! We use the 3 bits immediately after the QNaN bit for tagging.
//! This leaves 48 bits for payload, perfectly aligning with x86_64 / ARM64 virtual address space.

use std::fmt;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Array,
    Object,
    Closure,
    Resource,
    Yield,
}

/// Base mask for Quiet NaN (Sign=0, Exp=all 1s, highest mantissa bit=1)
pub const QNAN: u64 = 0x7FF8000000000000;

// Type Tags. QNAN already occupies bit 51 (0x0008...), so only bits 48-50 are
// free below the sign bit — tag values 0x0001..=0x0007. Anything that sets
// 0x0008 ORs down into an existing tag (0x0009 == 0x0001 == NULL), so the
// internal tokens live in the sign-bit space instead.
pub const TAG_NULL: u64    = 0x0001000000000000;
pub const TAG_FALSE: u64   = 0x0002000000000000;
pub const TAG_TRUE: u64    = 0x0003000000000000;
pub const TAG_INT: u64     = 0x0004000000000000;
pub const TAG_STR: u64     = 0x0005000000000000; // Pointer
pub const TAG_ARR: u64     = 0x0006000000000000; // Pointer
pub const TAG_OBJ: u64     = 0x0007000000000000; // Pointer
pub const TAG_CLOSURE: u64 = 0x8001000000000000; // Pointer (Uses sign bit)
pub const TAG_YIELD: u64   = 0x8002000000000000; // Internal yield token
pub const TAG_UNPACK_MARKER: u64 = 0x8003000000000000; // Internal arg-list sentinel
pub const TAG_REF: u64     = 0x8004000000000000; // Pointer to a PhpRef cell
pub const TAG_RESOURCE: u64 = 0x8005000000000000; // Pointer to a PhpResource
pub const TAG_NAMED_ARG_MARKER: u64 = 0x8006000000000000; // Internal named arg sentinel

pub const POINTER_MASK: u64 = 0x0000FFFFFFFFFFFF; // 48 bits

/// Represents any PHP Value in 64 bits.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Value(pub u64);

impl Value {
    #[inline]
    pub fn null() -> Self {
        Value(QNAN | TAG_NULL)
    }

    #[inline]
    pub fn new_bool(b: bool) -> Self {
        if b {
            Value(QNAN | TAG_TRUE)
        } else {
            Value(QNAN | TAG_FALSE)
        }
    }

    #[inline]
    pub fn new_int(i: i32) -> Self {
        // Cast to u32 to zero-extend, then u64
        Value(QNAN | TAG_INT | (i as u32 as u64))
    }

    #[inline]
    pub fn new_float(f: f64) -> Self {
        Value(f.to_bits())
    }
    
    // Pointers for complex types
    // Using raw pointers `*mut ()` for simplicity before implementing GC arenas
    #[inline]
    pub fn new_string_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_STR | (ptr as u64 & POINTER_MASK))
    }
    
    #[inline]
    pub fn new_array_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_ARR | (ptr as u64 & POINTER_MASK))
    }
    
    #[inline]
    pub fn new_object_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_OBJ | (ptr as u64 & POINTER_MASK))
    }

    #[inline]
    pub fn new_closure_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_CLOSURE | (ptr as u64 & POINTER_MASK))
    }

    #[inline]
    pub fn new_unpack_marker() -> Self {
        Value(QNAN | TAG_UNPACK_MARKER)
    }

    #[inline]
    pub fn new_named_arg_marker() -> Self {
        Value(QNAN | TAG_NAMED_ARG_MARKER)
    }

    #[inline]
    pub fn new_yield(id: u64) -> Self {
        Value(QNAN | TAG_YIELD | (id & POINTER_MASK))
    }

    /// Wrap a pointer to a `PhpRef` cell. A ref Value is an *indirection*:
    /// every read must go through `deref` and every write through the cell,
    /// so all aliases observe the same slot. PHP creates these for `&$x`,
    /// by-reference parameters, and `foreach ($a as &$v)`.
    #[inline]
    pub fn new_ref_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_REF | (ptr as u64 & POINTER_MASK))
    }

    /// Wrap a pointer to a `PhpResource`. A resource is its own type in PHP:
    /// `is_object()` is false, `gettype()` says `"resource"`, and there is no
    /// class behind it, so it cannot be modelled as an object.
    #[inline]
    pub fn new_resource_ptr(ptr: *mut ()) -> Self {
        Value(QNAN | TAG_RESOURCE | (ptr as u64 & POINTER_MASK))
    }

    // --- Type Checkers ---

    #[inline]
    pub fn is_float(&self) -> bool {
        // If it's not a NaN, or it is a standard IEEE float NaN (all fraction bits 0 except what makes it NaN)
        // A simple check: if the upper 16 bits are not exactly our QNAN masked tags
        // Actually, any value below QNAN is a float. Any value that is standard NaN is float.
        // It's a float if it doesn't have our QNAN mask AND at least one of our tag bits.
        (self.0 & QNAN) != QNAN
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == (QNAN | TAG_NULL)
    }

    #[inline]
    pub fn is_bool(&self) -> bool {
        self.0 == (QNAN | TAG_TRUE) || self.0 == (QNAN | TAG_FALSE)
    }

    #[inline]
    pub fn is_int(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_INT)
    }

    #[inline]
    pub fn is_string(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_STR)
    }

    #[inline]
    pub fn is_array(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_ARR)
    }

    #[inline]
    pub fn is_object(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_OBJ)
    }

    #[inline]
    pub fn is_closure(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_CLOSURE)
    }

    #[inline]
    pub fn is_unpack_marker(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_UNPACK_MARKER)
    }

    #[inline]
    pub fn is_named_arg_marker(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_NAMED_ARG_MARKER)
    }

    #[inline]
    pub fn is_yield(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_YIELD)
    }

    #[inline]
    pub fn is_ref(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_REF)
    }

    #[inline]
    pub fn is_resource(&self) -> bool {
        (self.0 & 0xFFFF000000000000) == (QNAN | TAG_RESOURCE)
    }

    /// The wrapped `PhpResource`, or `None` for any other type.
    ///
    /// A closed resource still returns its pointer here — `fclose` flips the
    /// kind to `Closed` rather than freeing, so `is_resource()` must consult
    /// `PhpResource::is_open` and not merely this tag.
    #[inline]
    pub fn as_resource_ptr(&self) -> Option<*mut ()> {
        if self.is_resource() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    #[inline]
    pub fn as_ref_ptr(&self) -> Option<*mut ()> {
        if self.is_ref() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    /// Read through a reference cell to the value it holds.
    ///
    /// Refs never nest in PHP — binding a ref to a ref shares the same cell —
    /// so a single hop is always enough. Non-ref values return themselves,
    /// making this safe to call on any Value before inspecting its type.
    #[inline]
    pub fn deref(&self) -> Value {
        if let Some(ptr) = self.as_ref_ptr() {
            unsafe { (*(ptr as *const crate::types::reference::PhpRef)).value }
        } else {
            *self
        }
    }

    // --- Extractors ---

    #[inline]
    pub fn to_int_coerced(&self) -> i64 {
        let val = self.deref();
        if let Some(i) = val.as_int() {
            i as i64
        } else if let Some(b) = val.as_bool() {
            if b { 1 } else { 0 }
        } else if let Some(f) = val.as_float() {
            f as i64
        } else if val.is_null() {
            0
        } else if let Some(s_ptr) = val.as_string_ptr() {
            let s = unsafe { &*(s_ptr as *const String) };
            s.trim().parse::<i64>().unwrap_or(0)
        } else {
            0
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<i32> {
        if self.is_int() {
            Some((self.0 & 0xFFFFFFFF) as i32)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.0 == (QNAN | TAG_TRUE) {
            Some(true)
        } else if self.0 == (QNAN | TAG_FALSE) {
            Some(false)
        } else {
            None
        }
    }
    
    #[inline]
    pub fn as_string_ptr(&self) -> Option<*mut ()> {
        if self.is_string() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    #[inline]
    pub fn as_array_ptr(&self) -> Option<*mut ()> {
        if self.is_array() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    #[inline]
    pub fn as_object_ptr(&self) -> Option<*mut ()> {
        if self.is_object() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    #[inline]
    pub fn as_closure_ptr(&self) -> Option<*mut ()> {
        if self.is_closure() {
            Some((self.0 & POINTER_MASK) as *mut ())
        } else {
            None
        }
    }

    #[inline]
    pub fn as_yield_id(&self) -> Option<u64> {
        if self.is_yield() {
            Some(self.0 & POINTER_MASK)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_type(&self) -> ValueType {
        // A ref is transparent to userland: gettype()/is_*() report the type
        // of the referenced value, never the cell itself.
        if self.is_ref() { return self.deref().get_type(); }
        if self.is_int() { return ValueType::Int; }
        if self.is_float() { return ValueType::Float; }
        if self.is_bool() { return ValueType::Bool; }
        if self.is_null() { return ValueType::Null; }
        if self.is_string() { return ValueType::String; }
        if self.is_array() { return ValueType::Array; }
        if self.is_object() { return ValueType::Object; }
        if self.is_closure() { return ValueType::Closure; }
        if self.is_resource() { return ValueType::Resource; }
        if self.is_yield() { return ValueType::Yield; }
        ValueType::Float // Fallback, shouldn't happen unless raw float
    }

    #[inline]
    pub fn strict_equals(&self, other: &Self) -> bool {
        self.strict_equals_depth(other, 0)
    }

    /// Depth cap for recursive comparison. PHP raises a fatal error on a
    /// self-referential compare; bail out rather than overflow the stack.
    const MAX_COMPARE_DEPTH: usize = 64;

    fn strict_equals_depth(&self, other: &Self, depth: usize) -> bool {
        // Comparisons see through refs: `$a === $b` compares pointees.
        if self.is_ref() || other.is_ref() {
            return self.deref().strict_equals_depth(&other.deref(), depth);
        }
        if self.is_int() && other.is_int() {
            self.as_int() == other.as_int()
        } else if self.is_float() && other.is_float() {
            self.as_float() == other.as_float()
        } else if self.is_null() && other.is_null() {
            true
        } else if self.is_bool() && other.is_bool() {
            self.as_bool() == other.as_bool()
        } else if self.is_string() && other.is_string() {
            let ls = unsafe { &*(self.as_string_ptr().unwrap() as *const String) };
            let rs = unsafe { &*(other.as_string_ptr().unwrap() as *const String) };
            ls == rs
        } else if self.is_array() && other.is_array() {
            // `===` on arrays is by content: same keys in the same order, with
            // strictly-equal values. Comparing the handles would make every
            // distinct-but-identical array unequal.
            let (Some(ap), Some(bp)) = (self.as_array_ptr(), other.as_array_ptr()) else {
                return false;
            };
            if ap == bp {
                return true;
            }
            if depth > Self::MAX_COMPARE_DEPTH {
                return false;
            }
            let (aa, ba) = unsafe {
                (
                    &*(ap as *const crate::types::array::PhpArray),
                    &*(bp as *const crate::types::array::PhpArray),
                )
            };
            if aa.elements.len() != ba.elements.len() {
                return false;
            }
            aa.elements
                .iter()
                .zip(ba.elements.iter())
                .all(|((ak, av), (bk, bv))| ak == bk && av.strict_equals_depth(bv, depth + 1))
        } else if self.is_object() && other.is_object() {
            if self.as_object_ptr() == other.as_object_ptr() {
                return true;
            }
            // In PHP, Enum cases with the same class name and case name are identical
            let (Some(ap), Some(bp)) = (self.as_object_ptr(), other.as_object_ptr()) else {
                return false;
            };
            let (ao, bo) = unsafe {
                (
                    &*(ap as *const crate::types::object::PhpObject),
                    &*(bp as *const crate::types::object::PhpObject),
                )
            };
            let same_class = (ao.class_id == bo.class_id) || match (&ao.class_name, &bo.class_name) {
                (Some(a_cn), Some(b_cn)) => a_cn.eq_ignore_ascii_case(b_cn),
                _ => false,
            };
            if same_class {
                if let (Some(a_name_val), Some(b_name_val)) = (ao.properties.get("name"), bo.properties.get("name")) {
                    if a_name_val.strict_equals_depth(b_name_val, depth + 1) {
                        return true;
                    }
                }
            }
            false
        } else if self.is_closure() && other.is_closure() {
            self.as_closure_ptr() == other.as_closure_ptr()
        } else if self.is_resource() && other.is_resource() {
            // Two handles are identical only if they are the same stream.
            self.as_resource_ptr() == other.as_resource_ptr()
        } else {
            false
        }
    }

    #[inline]
    pub fn equals(&self, other: &Self) -> bool {
        // Loose equality (for now just strict equality since type coercion is complex)
        self.strict_equals(other)
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        if self.is_ref() {
            return self.deref().is_truthy();
        }
        if let Some(b) = self.as_bool() {
            b
        } else if let Some(i) = self.as_int() {
            i != 0
        } else if let Some(f) = self.as_float() {
            f != 0.0
        } else if self.is_null() {
            false
        } else if self.is_string() {
            let s = unsafe { &*(self.as_string_ptr().unwrap() as *const String) };
            // PHP: "" and "0" are falsy
            !s.is_empty() && s != "0"
        } else if self.is_array() {
            let arr = unsafe {
                &*(self.as_array_ptr().unwrap() as *const crate::types::array::PhpArray)
            };
            !arr.elements.is_empty()
        } else {
            // Objects, closures → truthy
            true
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_null() {
            serializer.serialize_none()
        } else if self.is_bool() {
            serializer.serialize_bool(self.as_bool().unwrap())
        } else if self.is_int() {
            serializer.serialize_i32(self.as_int().unwrap())
        } else if self.is_float() {
            serializer.serialize_f64(self.as_float().unwrap())
        } else if self.is_string() {
            let ptr = self.as_string_ptr().unwrap() as *const String;
            let s = unsafe { &*ptr };
            serializer.serialize_str(s)
        } else if self.is_object() {
            let ptr = self.as_object_ptr().unwrap() as *const crate::types::object::PhpObject;
            let obj = unsafe { &*ptr };
            obj.serialize(serializer)
        } else {
            serializer.serialize_unit()
        }
    }
}


impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "NULL")
        } else if let Some(b) = self.as_bool() {
            write!(f, "Bool({})", b)
        } else if let Some(i) = self.as_int() {
            write!(f, "Int({})", i)
        } else if let Some(fl) = self.as_float() {
            write!(f, "Float({})", fl)
        } else if self.is_string() {
            write!(f, "StringPtr({:#x})", self.0 & POINTER_MASK)
        } else if self.is_array() {
            write!(f, "ArrayPtr({:#x})", self.0 & POINTER_MASK)
        } else if self.is_object() {
            write!(f, "ObjectPtr({:#x})", self.0 & POINTER_MASK)
        } else if self.is_ref() {
            write!(f, "Ref({:#x} -> {:?})", self.0 & POINTER_MASK, self.deref())
        } else if self.is_resource() {
            let r = unsafe { &*((self.0 & POINTER_MASK) as *const crate::types::resource::PhpResource) };
            write!(f, "Resource(id #{}, {})", r.id, r.uri)
        } else {
            write!(f, "Unknown({:#018x})", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let v = Value::null();
        assert!(v.is_null());
        assert!(!v.is_int());
        assert!(!v.is_float());
    }

    #[test]
    fn test_bool() {
        let t = Value::new_bool(true);
        let f = Value::new_bool(false);
        assert!(t.is_bool());
        assert!(f.is_bool());
        assert_eq!(t.as_bool(), Some(true));
        assert_eq!(f.as_bool(), Some(false));
    }

    #[test]
    fn test_int() {
        let i = Value::new_int(42);
        assert!(i.is_int());
        assert!(!i.is_float());
        assert_eq!(i.as_int(), Some(42));
        
        let i_neg = Value::new_int(-42);
        assert!(i_neg.is_int());
        assert_eq!(i_neg.as_int(), Some(-42));
    }

    #[test]
    fn test_float() {
        let f = Value::new_float(3.1415);
        assert!(f.is_float());
        assert!(!f.is_int());
        assert_eq!(f.as_float(), Some(3.1415));
        
        // Zero float
        let f_zero = Value::new_float(0.0);
        assert!(f_zero.is_float());
        assert_eq!(f_zero.as_float(), Some(0.0));
    }
    
    #[test]
    fn test_pointers() {
        let dummy_ptr = 0x123456789ABC as *mut ();
        let str_val = Value::new_string_ptr(dummy_ptr);
        
        assert!(str_val.is_string());
        assert!(!str_val.is_int());
        assert_eq!(str_val.as_string_ptr(), Some(dummy_ptr));
    }

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Value>(), 8);
    }

    #[test]
    fn test_ref_tag_is_distinct() {
        // A ref cell must not be mistaken for any other type — the whole
        // value model depends on these tags staying disjoint.
        let cell = Box::into_raw(Box::new(crate::types::reference::PhpRef::new(
            Value::new_int(7),
        )));
        let r = Value::new_ref_ptr(cell as *mut ());

        assert!(r.is_ref());
        assert!(!r.is_null());
        assert!(!r.is_int());
        assert!(!r.is_float());
        assert!(!r.is_bool());
        assert!(!r.is_string());
        assert!(!r.is_array());
        assert!(!r.is_object());
        assert!(!r.is_closure());
        assert!(!r.is_yield());
        assert!(!r.is_unpack_marker());
        assert!(!r.is_named_arg_marker());
        assert_eq!(r.as_ref_ptr(), Some(cell as *mut ()));

        unsafe { drop(Box::from_raw(cell)) };
    }

    #[test]
    fn test_ref_is_transparent() {
        // Reads through a ref observe the pointee, so userland type checks
        // and truthiness behave as if the ref were not there.
        let cell = Box::into_raw(Box::new(crate::types::reference::PhpRef::new(
            Value::new_int(42),
        )));
        let r = Value::new_ref_ptr(cell as *mut ());

        assert_eq!(r.deref().as_int(), Some(42));
        assert_eq!(r.get_type(), ValueType::Int);
        assert!(r.is_truthy());
        assert!(r.strict_equals(&Value::new_int(42)));
        assert!(Value::new_int(42).strict_equals(&r));

        // Writing through the cell is visible to every alias.
        unsafe { (*cell).set(Value::new_int(0)) };
        assert_eq!(r.deref().as_int(), Some(0));
        assert!(!r.is_truthy());

        unsafe { drop(Box::from_raw(cell)) };
    }

    #[test]
    fn test_refs_do_not_nest() {
        // Binding a ref to a ref shares the cell rather than wrapping it,
        // so a single deref hop always reaches a concrete value.
        let inner = Box::into_raw(Box::new(crate::types::reference::PhpRef::new(
            Value::new_int(5),
        )));
        let r = Value::new_ref_ptr(inner as *mut ());

        let outer = crate::types::reference::PhpRef::new(r);
        assert!(!outer.get().is_ref());
        assert_eq!(outer.get().as_int(), Some(5));

        unsafe { drop(Box::from_raw(inner)) };
    }

    #[test]
    fn test_resource_tag_is_distinct() {
        // The resource tag shares the sign-bit space with closures, yields,
        // unpack markers and refs — a collision there would silently turn a
        // stream handle into one of those.
        let res = Box::into_raw(Box::new(crate::types::resource::PhpResource::new(
            crate::types::resource::ResourceKind::Stderr,
            "php://stderr",
        )));
        let v = Value::new_resource_ptr(res as *mut ());

        assert!(v.is_resource());
        assert_eq!(v.get_type(), ValueType::Resource);
        assert!(!v.is_null());
        assert!(!v.is_int());
        assert!(!v.is_float());
        assert!(!v.is_bool());
        assert!(!v.is_string());
        assert!(!v.is_array());
        assert!(!v.is_object());
        assert!(!v.is_closure());
        assert!(!v.is_ref());
        assert!(!v.is_yield());
        assert!(!v.is_unpack_marker());
        assert!(!v.is_named_arg_marker());
        assert_eq!(v.as_resource_ptr(), Some(res as *mut ()));
        // Resources are always truthy in PHP, open or not.
        assert!(v.is_truthy());
        // Identity, not content: a second handle to the same kind differs.
        assert!(v.strict_equals(&v));

        unsafe { drop(Box::from_raw(res)) };
    }

    #[test]
    fn test_deref_on_non_ref_is_identity() {
        // deref() is called on the hot path before type inspection, so it
        // must be a no-op for ordinary values.
        for v in [
            Value::null(),
            Value::new_bool(true),
            Value::new_int(-3),
            Value::new_float(1.5),
        ] {
            assert_eq!(v.deref(), v);
        }
    }
}
