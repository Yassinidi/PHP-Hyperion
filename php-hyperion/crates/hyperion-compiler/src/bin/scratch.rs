use hyperion_parser::{lexer::Lexer, parser::Parser};

fn main() {
    let source = "<?php echo 'ok';";
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(tokens.into_iter());
    let ast = parser.parse_program();
    println!("{:#?}", ast);
}
