use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "declare(strict_types=1);";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let expr = parser.parse_program();
    println!("{:#?}", expr);
}
