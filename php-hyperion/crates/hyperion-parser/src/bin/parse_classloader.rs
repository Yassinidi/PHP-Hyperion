use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("../example-app/vendor/composer/ClassLoader.php").unwrap();
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    for stmt in ast {
        if let hyperion_parser::ast::Stmt::Class { name, methods, .. } = &stmt {
            if name == "ClassLoader" {
                for m in methods {
                    if let hyperion_parser::ast::Stmt::Function { name: m_name, body, .. } = m {
                        if m_name == "loadClass" {
                            println!("{:#?}", body);
                        }
                    }
                }
            }
        }
    }
}
