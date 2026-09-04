# AI Plan: Lexer
**Goal**: Tokenize raw PHP source code.

## AI Implementation Steps:
1. **Token Enum**: Define `enum Token` covering all PHP keywords, operators, and literals (T_ECHO, T_IF, T_STRING, T_VARIABLE).
2. **Lexer Struct**: Create `Lexer<'a>` holding the source string and current cursor.
3. **HTML vs PHP Mode**: The AI must implement state switching between inline HTML and `<?php ... ?>`.
4. **String Interpolation**: Handle double-quoted strings with variables `"Hello $name"`.
