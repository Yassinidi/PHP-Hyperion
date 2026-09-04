# AI Plan: JIT Assembler
**Goal**: Convert TraceIR into executable CPU instructions in memory.

## AI Implementation Steps:
1. **mmap Allocation**: AI must allocate RWX (Read/Write/Execute) memory pages using `libc::mmap`.
2. **x86_64 Encoding**: Write basic instruction encoders for standard operations (MOV, ADD, CMP, JMP).
3. **Register Allocation**: Map IR virtual registers to physical CPU registers (RAX, RBX, etc.).
4. **Execution**: Provide a function pointer cast `unsafe { mem::transmute::<_, fn()>(ptr) }` to jump into the generated machine code.
