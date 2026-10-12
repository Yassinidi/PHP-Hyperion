use std::fs;
use hyperion_parser::Parser;

fn main() {
    let content = fs::read_to_string("test_create.php").unwrap();
    let lexer = hyperion_parser::lexer::Lexer::new(&content);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    println!("{:#?}", program);
}
