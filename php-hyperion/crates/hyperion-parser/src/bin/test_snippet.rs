use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: test_snippet <php_code>");
        return;
    }
    let source = &args[1];
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    
    println!("{:#?}", ast);
}
