# AI Implementation Plan: Runtime Module
**Goal**: M:N green thread scheduler and non-blocking I/O (The "Hyper" in Hyperion).

## AI Tasks & Roadmap:
- **Phase 1 (Fibre)**: Implement stackful coroutines (Hyper-Fibres) that can pause and resume execution.
- **Phase 2 (Scheduler)**: Dispatch millions of Fibres across a small pool of OS threads.
- **Phase 3 (I/O)**: Hook socket/file operations to yield Fibres instead of blocking the OS thread.
