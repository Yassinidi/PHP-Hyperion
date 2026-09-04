use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "php declare(strict_types=1); namespace Laravel\\AgentDetector;";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    
    while parser.peek().is_some() {
        if parser.peek().unwrap().token == hyperion_parser::lexer::Token::Eof { break; }
        println!("PEEK: {:?}", parser.peek().unwrap().token);
        let _stmt = parser.parse_program(); // Wait, I can't call parse_statement because it's private.
        break;
    }
}
