# AI Plan: PHP Types
**Goal**: Replicate PHP's internal data structures in Rust.

## AI Implementation Steps:
1. **String (`string.rs`)**: 
   - Implement `PhpString` with length, hash cache, and inline buffer for short strings (SSO).
2. **Array (`array.rs`)**: 
   - Implement `PhpArray` using a hybrid of `IndexMap` (Rust) or a custom Robin Hood hashing table that preserves insertion order (crucial for PHP `foreach`).
3. **Object (`object.rs`)**:
   - Implement `PhpObject` linking to a `ClassEntry`.
   - Store properties in a dense array (for declared props) and a dynamic table (for dynamic props).
