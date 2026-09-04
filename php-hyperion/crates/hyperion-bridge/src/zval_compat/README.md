# AI Plan: Zval Compatibility
**Goal**: Translate our fast `Value` (NaN boxed) to Zend's `zval` (C struct).

## AI Implementation Steps:
1. **C Struct Representation**: Define `#[repr(C)] struct zval` in Rust matching PHP 8's memory layout.
2. **Conversion Logic**: Write `to_zval(v: Value) -> zval` and `from_zval(z: &zval) -> Value`.
3. **Memory Ownership**: Guarantee that the Rust GC doesn't free strings or arrays while they are being manipulated by C extensions.
