# AI Plan: Parser
**Goal**: Build the AST from Tokens.

## AI Implementation Steps:
1. **AST Nodes**: Define `enum Expr` and `enum Stmt`. 
   - `Expr`: BinaryOp, Variable, Call, Array, Closure.
   - `Stmt`: If, While, Return, Echo, ClassDecl, FunctionDecl.
2. **Parsing Strategy**: Implement a Pratt Parser for expressions (to handle operator precedence nicely) and Recursive Descent for statements.
3. **Error Recovery**: AI should implement synchronization to recover from syntax errors instead of crashing immediately.
