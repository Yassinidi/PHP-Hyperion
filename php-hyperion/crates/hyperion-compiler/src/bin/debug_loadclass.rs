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
                if func.name == "loadClass" {
                    println!("--- loadClass constants ---");
                    for (ci, c) in func.chunk.constants.iter().enumerate() {
                        if let Some(s) = c.as_string_ptr() {
                            let str_val = unsafe { &*(s as *const String) };
                            println!("CONST[{}]: String({:?})", ci, str_val);
                        } else {
                            println!("CONST[{}]: {:?}", ci, c);
                        }
                    }

                    println!("--- loadClass code ---");
                    let code = &func.chunk.code;
                    println!("{:?}", code);

                }
            }
        }
    }
}
