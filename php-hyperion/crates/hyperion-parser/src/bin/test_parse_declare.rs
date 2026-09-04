use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "php declare(strict_types=1); namespace Laravel\\AgentDetector;";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    for stmt in ast {
        println!("{:#?}", stmt);
    }
}
