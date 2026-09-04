use hyperion_parser::parser::Parser;
use hyperion_parser::lexer::Lexer;

fn main() {
    let source = "
    class Foo {
        public function __construct($vendorDir = null)
        {
            $this->vendorDir = $vendorDir;
        }

        public function getPrefixes() {
            echo 'hello';
        }
    }
    ";
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program();
    
    println!("{:#?}", ast);
}
