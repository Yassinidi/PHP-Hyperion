# AI Plan: Memory (NaN Boxing)
**Goal**: Efficient 8-byte variable storage representing all PHP scalar types and pointers.

## AI Implementation Steps:
1. **Struct Definition**: Create `pub struct Value(u64);` in `nan_box.rs`.
2. **Constants**: Define bitmasks for `TAG_INT`, `TAG_BOOL`, `TAG_NULL`, `TAG_OBJ`, `TAG_STR`, `TAG_ARR`.
3. **Constructors**: Implement `Value::new_int(i32)`, `Value::new_float(f64)`, `Value::new_ptr(*mut T)`.
4. **Extraction**: Implement `is_int()`, `as_int()`, `is_ptr()`, `as_ptr()`.
5. **Testing**: Write exhaustive tests in `mod.rs` to ensure bits don't overlap and endianness is handled.
