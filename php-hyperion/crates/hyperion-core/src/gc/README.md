# AI Plan: Garbage Collector
**Goal**: Memory safety without reference counting overhead.

## AI Implementation Steps:
1. **Heap Allocation**: Implement an arena allocator for `PhpString`, `PhpArray`, `PhpObject`.
2. **Roots Identification**: AI needs to write logic to scan the VM stack and globals to find GC roots.
3. **Mark Phase**: Implement tracing algorithms to mark reachable objects.
4. **Sweep/Scavenge**: Implement the cleanup of unmarked objects.
5. **Integration**: Connect the GC to `core/src/memory/nan_box.rs` so `Value` pointers are updated if moved by a compacting GC.
