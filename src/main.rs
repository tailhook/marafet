extern crate argparse;
extern crate parser_combinators;
extern crate unicode_segmentation;

use std::rc::Rc;
use std::fs::File;
use std::io::{Read, stdout};
use std::path::PathBuf;

use argparse::{ArgumentParser, Parse, ParseOption, Collect, StoreTrue};
use parser_combinators::{parser, Parser, ParserExt, from_iter};

mod grammar;
mod js;
mod css;
mod util;

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
    let (ast, _) = parser(grammar::body)
        .parse(from_iter(grammar::Tokenizer::new(&body[..])))
        .unwrap();  // TODO(tailhook) should check tail?
    println!("AST");
    println!("{:?}", ast);

    for blk in ast.iter() {
        match blk {
            &grammar::Block::Css(_, _) => {
                use std::default::Default;
                println!("------- CSS -------");
                css::generate(&mut stdout(), blk, &Default::default()
                    ).unwrap();
                println!("----- end css -----");
            }
            &grammar::Block::Html(_, _, _) => {
                println!("------- HTML->JS -------");
                js::generate(&mut stdout(), blk).unwrap();
                println!("----- end html->js -----");
            }
        }
    }
}

