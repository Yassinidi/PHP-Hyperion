use hyperion_parser::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("../example-app/vendor/laravel/agent-detector/src/AgentDetector.php").unwrap();
    let mut lexer = Lexer::new(&source);
    for _ in 0..10 {
        let tok = lexer.next_token();
        if tok.token == hyperion_parser::lexer::Token::Eof { break; }
        println!("{:?}", tok.token);
    }
}
