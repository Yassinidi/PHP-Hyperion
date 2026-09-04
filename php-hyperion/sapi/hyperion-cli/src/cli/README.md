# AI Plan: CLI
**Goal**: Execute PHP scripts from the terminal.

## AI Implementation Steps:
1. **Argument Parsing**: Parse commands like `php-h script.php`.
2. **Execution Flow**: Read file -> `compiler::lexer` -> `parser` -> `opcode` -> `interpreter::VM`.
3. **STDOUT Hook**: Pipe PHP's `echo` and `print` outputs directly to the terminal's standard output.
