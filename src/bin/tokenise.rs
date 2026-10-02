use choreo::parser::{Lexer, Parser};

fn main() {
    let lexer = Lexer::from_file("./demo.csf").unwrap();
    dbg!(lexer.collect::<Result<Vec<_>, _>>().unwrap());

    // let parser = Parser::new(lexer);
    // let script = parser.parse().unwrap();
    // dbg!(script);
}
