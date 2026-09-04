use hyperion_parser::lexer::Lexer;
use hyperion_parser::parser::Parser;
use hyperion_compiler::compiler::Compiler;

fn main() {
    let file_path = std::env::args().nth(1).expect("Missing file argument");
    let source = std::fs::read_to_string(&file_path).unwrap_or_else(|_| panic!("Cannot read file: {}", file_path));
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    
    let compiler = Compiler::new(file_path);
    let _compiled = compiler.compile(program);
}
