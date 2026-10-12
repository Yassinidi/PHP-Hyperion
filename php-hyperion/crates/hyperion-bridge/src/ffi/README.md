# AI Plan: FFI Engine
**Goal**: Call C functions directly from Rust/PHP.

## AI Implementation Steps:
1. **Dynamic Loading**: Use the `libloading` crate to load compiled C extensions.
2. **Symbol Resolution**: Look up Zend extension structs and initialization functions.
3. **Function Hooking**: Allow loaded C functions to register themselves into the PHP-H `VM` function table.
