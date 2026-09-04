#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    
    #[test]
    fn test_ast() {
        let source = "<?php echo 'Hello'; ?>";
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();
        assert_eq!(program.len(), 1);
    }
}
