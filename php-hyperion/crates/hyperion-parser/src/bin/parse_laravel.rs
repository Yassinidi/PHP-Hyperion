use std::fs;
use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let file = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: parse_laravel <path_to_php_file>");
        std::process::exit(1);
    });
    let content = fs::read_to_string(&file).expect("Failed to read PHP file");
    let mut lexer = Lexer::new(&content);
    let mut tokens = Vec::new();
    loop {
        let t = lexer.next_token();
        let eof = t.token == hyperion_parser::lexer::Token::Eof;
        tokens.push(t);
        if eof { break; }
    }
    let mut parser = Parser::new(tokens.into_iter());
    let ast = parser.parse_program();
    for (i, stmt) in ast.iter().enumerate() {
        println!("=== Statement {} ===", i);
        if let hyperion_parser::parser::ast::Stmt::If { condition, then_branch, else_branch } = stmt {
            println!("If cond={:?}", condition);
            println!("  then_branch: {} statements", then_branch.len());
            for (j, s) in then_branch.iter().enumerate() {
                match s {
                    hyperion_parser::parser::ast::Stmt::Echo(e) => {
                        println!("    then[{}] Echo with exprs: {:?}", j, match &e[0] {
                            hyperion_parser::parser::ast::Expr::LiteralString(s) => format!("LiteralString(len={}, prefix={:?})", s.len(), &s[..s.len().min(40)]),
                            other => format!("{:?}", other),
                        });
                    }
                    hyperion_parser::parser::ast::Stmt::If { condition, .. } => {
                        println!("    then[{}] If cond={:?}", j, condition);
                    }
                    other => println!("    then[{}] {:?}", j, other),
                }
            }
            println!("  else_branch: {:?}", else_branch);
        }
    }
}
