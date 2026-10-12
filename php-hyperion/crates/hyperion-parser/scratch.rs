use hyperion_parser::{lexer::Lexer, parser::Parser};
fn main() {
    let content = std::fs::read_to_string("../test_empty_array_elvis.php").unwrap();
    let lexer = Lexer::new(&content);
    let mut parser = Parser::new(lexer);
    println!("{:#?}", parser.parse_program());
}
