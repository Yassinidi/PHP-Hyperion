# AI Implementation Plan: Bridge Module
**Goal**: Compatibility with legacy PHP C extensions.

## AI Tasks & Roadmap:
- **Phase 1 (Zval Compat)**: Convert between `NaN Box` memory format and Zend `zval`.
- **Phase 2 (FFI)**: Link and execute C extension `.so`/`.dll` files dynamically.
- **Phase 3 (Native Exts)**: Port or bridge standard heavy extensions like PDO and GD.
