use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let file_path = std::env::args().nth(1).expect("Missing file argument");
    let source = std::fs::read_to_string(&file_path).unwrap_or_else(|_| panic!("Cannot read file: {}", file_path));
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let _ = parser.parse_program();
}
