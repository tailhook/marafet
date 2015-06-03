extern crate argparse;
extern crate parser_combinators;

use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

use argparse::{ArgumentParser, Parse, ParseOption, Collect, StoreTrue};
use parser_combinators::{parser, Parser, ParserExt};
//use parser_combinators::combinator::{Try, Or, many, Many, ParserExt, Map};

mod grammar;


fn main() {
    let mut source = PathBuf::new();
    let mut output_js = None::<PathBuf>;
    let mut output_css = None::<PathBuf>;
    let mut vars = Vec::<String>::new();
    let mut print_ast = false;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Compiles .mft file to a CSS and/or JS file");
        ap.refer(&mut source)
            .required()
            .add_option(&["-f", "--file"], Parse, "Input file name");
        ap.refer(&mut output_js)
            .add_option(&["--js"], ParseOption, "Output JS file");
        ap.refer(&mut output_css)
            .add_option(&["--css"], ParseOption, "Output CSS file");
        ap.refer(&mut vars)
            .add_option(&["--css-var"], Collect, "Set CSS variable");
        ap.refer(&mut print_ast)
            .add_option(&["--print-ast"], StoreTrue, "Print AST to stdout");
        ap.parse_args_or_exit();
    }

    let mut buf = Vec::new();
    File::open(source).and_then(|mut f| f.read_to_end(&mut buf)).unwrap();
    let body = String::from_utf8(buf).unwrap();
    let (ast, tail) = parser(grammar::body)
        .parse(grammar::Tokenizer::new(&body[..]))
        .unwrap();  // TODO(tailhook) should check tail?
    if !tail.end_of_file() {
        println!("Tokenizer error: {}", tail.error_message());
    }
    println!("AST");
    println!("{:?}", ast);
}

