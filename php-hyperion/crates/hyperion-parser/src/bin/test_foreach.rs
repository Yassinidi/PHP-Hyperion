use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "foreach ($array as $v) { }";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    println!("{:#?}", ast);
}
