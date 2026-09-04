pub struct Local {
    pub name: String,
    // Note: Since we are using Function-Scope, `depth` represents whether the variable 
    // is in the global scope (0) or function scope (1). For PHP, local variables within 
    // a function all share the same scope, so we won't need to pop them off when leaving a block.
    pub depth: usize,
}

pub struct SymbolTable {
    pub locals: Vec<Local>,
    pub scope_depth: usize,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    pub fn locals_count(&self) -> usize {
        self.locals.len()
    }

    /// Resolves a variable name to its index in the local stack array.
    /// Returns the index if found, or None if it hasn't been declared yet.
    pub fn resolve_local(&self, name: &str) -> Option<u8> {
        // Search backwards to ensure we get the most recently declared variable
        // (Though in function-scope this isn't strictly necessary for shadowing,
        // it's a good practice for when we eventually support closures or scope nuances).
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u8);
            }
        }
        None
    }

    /// Adds a new local variable to the symbol table and returns its index.
    /// Panics if the maximum number of locals (256) is exceeded.
    pub fn add_local(&mut self, name: &str) -> u8 {
        if self.locals.len() >= 256 {
            panic!("Too many local variables in function (maximum is 256).");
        }

        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
        });

        (self.locals.len() - 1) as u8
    }

    /// Helper for entering a new function scope.
    pub fn enter_function_scope(&mut self) {
        self.scope_depth += 1;
        // In the future, this is where we might push the current symbol table 
        // to a stack and start a fresh one for the new function.
    }

    /// Helper for leaving a function scope.
    pub fn leave_function_scope(&mut self) {
        self.scope_depth -= 1;
        // Clear all locals since the function has ended.
        self.locals.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_local() {
        let mut table = SymbolTable::new();
        assert_eq!(table.resolve_local("driver"), None);
        
        let idx = table.add_local("driver");
        assert_eq!(idx, 0);
        assert_eq!(table.resolve_local("driver"), Some(0));

        let idx2 = table.add_local("name");
        assert_eq!(idx2, 1);
        assert_eq!(table.resolve_local("name"), Some(1));
    }

    #[test]
    fn test_function_scope() {
        let mut table = SymbolTable::new();
        table.add_local("global_var");
        
        table.enter_function_scope();
        table.add_local("local_var");
        
        assert_eq!(table.locals.len(), 2);
        assert_eq!(table.resolve_local("local_var"), Some(1));
        
        table.leave_function_scope();
        assert_eq!(table.locals.len(), 0);
        assert_eq!(table.resolve_local("global_var"), None); // Since we cleared everything. 
        // In reality, entering a function in PHP should spawn a fresh table,
        // but this verifies the clear logic.
    }
}
