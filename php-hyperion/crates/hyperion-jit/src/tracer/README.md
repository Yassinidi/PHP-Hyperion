# AI Plan: JIT Tracer
**Goal**: Record operations when the Baseline Interpreter detects a hot loop.

## AI Implementation Steps:
1. **Trace IR**: Create a `TraceIR` enum which is lower-level than standard Opcodes and tied to specific observed types.
2. **Recording State**: Implement `Tracer::record(opcode, types)` which is called by the VM.
3. **Guard Insertion**: Automatically insert Guard IR nodes where type assumptions are made (e.g., `GuardIsInt(reg1)`).
