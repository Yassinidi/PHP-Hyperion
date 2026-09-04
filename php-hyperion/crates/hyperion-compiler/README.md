# AI Implementation Plan: Compiler Module
**Goal**: Parse PHP code and emit highly optimized bytecodes (Opcodes).

## AI Tasks & Roadmap:
- **Phase 1 (Lexer)**: Read PHP string/file and yield `Token`s. Must handle PHP's HTML mixing (`<?php`).
- **Phase 2 (Parser)**: Recursive Descent or Pratt parser turning `Token`s into an Abstract Syntax Tree (AST).
- **Phase 3 (Opcode)**: Traverse AST and generate a flat array of `Opcode` instructions.
- **Phase 4 (Interpreter)**: Implement the baseline VM loop that reads `Opcode` array and executes it using `core::memory::Value`.
