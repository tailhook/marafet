extern crate argparse;
extern crate parser_combinators;
extern crate unicode_segmentation;

use std::fs::File;
use std::io::{Read, stdout};
use std::path::PathBuf;

use argparse::{ArgumentParser, Parse, ParseOption, Collect, StoreTrue};
use parser_combinators::{parser, Parser, ParserExt, from_iter};

mod grammar;
mod visitors;
mod js;
mod css;
mod util;

fn main() {
    let mut source = PathBuf::new();
    let mut output_js = None::<PathBuf>;
    let mut output_css = None::<PathBuf>;
    let mut block_name = None::<String>;
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
        ap.refer(&mut block_name)
            .add_option(&["--block-name"], ParseOption,
                "Set block name (this is a name of CSS class that will be \
                 prepended to every classname in CSS and HTML to limit scope \
                 of style for this block only). By default derived from file \
                 name");
        ap.refer(&mut vars)
            .add_option(&["--css-var"], Collect, "Set CSS variable");
        ap.refer(&mut print_ast)
            .add_option(&["--print-ast"], StoreTrue, "Print AST to stdout");
        ap.parse_args_or_exit();
    }

    let mut buf = Vec::new();
    File::open(&source).and_then(|mut f| f.read_to_end(&mut buf)).unwrap();
    let body = String::from_utf8(buf).unwrap();
    let (ast, _) = parser(grammar::body)
        .parse(from_iter(grammar::Tokenizer::new(&body[..])))
        .unwrap();  // TODO(tailhook) should check tail?

    println!("--- Initial Ast ---");
    println!("{:?}", ast);

    let ast = visitors::add_block_name(ast,
        &block_name.unwrap_or(
            String::from(source.file_stem().unwrap().to_str().unwrap())
        )[..]);

    println!("--- Transformed Ast --");
    println!("{:?}", ast);

    for blk in ast.blocks.iter() {
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

