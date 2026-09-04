use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;
use hyperion_compiler::compiler::Compiler;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    
    let compiler = Compiler::new("debug.php".to_string());
    let result = compiler.compile(ast);
    
    for cls in result.classes {
        if cls.name.contains("ClassLoader") {
            for func in cls.methods {
                if func.name == "findFile" {
                    println!("--- findFile ---");
                    let code = &func.chunk.code;
                    let mut i = 0;
                    while i < code.len() {
                        if let Ok(op) = hyperion_bytecode::Opcode::try_from(code[i]) {
                            println!("{}: {:?}", i, op);
                        } else {
                            println!("{}: {}", i, code[i]);
                        }
                        i += 1;
                    }
                }
            }
        }
    }
}
