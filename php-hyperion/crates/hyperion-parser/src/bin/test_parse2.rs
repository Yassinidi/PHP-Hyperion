use hyperion_parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "<?php (new ($this->getPermissionClass())())->newInstance([], true);";
    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        if tok.token == hyperion_parser::lexer::Token::Eof { break; }
        tokens.push(tok);
    }
    let mut parser = hyperion_parser::Parser::new(tokens.into_iter());
    let stmts = parser.parse_program();
    for stmt in &stmts {
        println!("{:#?}", stmt);
    }
}
