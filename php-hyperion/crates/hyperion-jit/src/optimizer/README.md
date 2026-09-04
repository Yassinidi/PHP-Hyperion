# AI Plan: JIT Optimizer
**Goal**: Optimize TraceIR before assembling.

## AI Implementation Steps:
1. **Constant Folding**: Pre-calculate static expressions in the trace.
2. **Dead Code Elimination**: Remove unused assignments or redundant guard checks.
3. **Type Specialization**: Convert heavy dynamic PHP operators (which handle multiple types) into pure, fast native integer/float CPU instructions based on the Guards.
