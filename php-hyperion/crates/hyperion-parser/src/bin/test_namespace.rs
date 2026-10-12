use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "namespace Laravel\\AgentDetector; class AgentDetector {}";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    println!("{:#?}", ast);
}
