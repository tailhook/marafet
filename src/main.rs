extern crate argparse;

extern crate marafet_parser as parser;
extern crate marafet_css as css;
extern crate marafet_es5citojs as es5citojs;

use std::fs::File;
use std::io::{Read, Write, BufWriter};
use std::io::{stdin, stdout, stderr};
use std::io::Error as IoError;
use std::path::{PathBuf, Path};
use std::process::exit;

use argparse::{ArgumentParser, Parse, ParseOption, Collect, StoreTrue};


fn read_file<R: Read>(f: Result<R, IoError>) -> Result<String, IoError> {
    let mut buf = Vec::new();
    match f.and_then(|mut f| f.read_to_end(&mut buf)) {
        Ok(_) => Ok(String::from_utf8(buf).unwrap()),
        Err(e) => Err(e),
    }
}


fn main() {
    let mut source = PathBuf::new();
    let mut use_amd = false;
    let mut amd_name = None::<String>;
    let mut output_js = None::<PathBuf>;
    let mut output_css = None::<PathBuf>;
    let mut block_name = None::<String>;
    let mut vars = Vec::<String>::new();
    let mut print_ast = false;
    let mut css_load = false;
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
        ap.refer(&mut use_amd)
            .add_option(&["--amd"], StoreTrue, "Output AMD module");
        ap.refer(&mut amd_name)
            .add_option(&["--amd-name"], ParseOption,
                "Set AMD name for module");
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
        ap.refer(&mut css_load)
            .add_option(&["--auto-load-css"], StoreTrue,
                "Insert css load code to the Javascript code");
        ap.parse_args_or_exit();
    }

    let block_name = block_name.unwrap_or(
            String::from(source.file_stem().unwrap().to_str().unwrap()));

    let fileresult = if Path::new(&source) == Path::new("-") {
        read_file(Ok(stdin()))
    } else {
        read_file(File::open(&source))
    };
    let body = match fileresult {
        Ok(data) => data,
        Err(e) => {
            writeln!(&mut stderr(), "Error reading file {:?}: {}", source, e)
                .unwrap();
            exit(1);
        }
    };

    let ast = match parser::parse_string(&body[..]) {
        Ok(ast) => ast,
        Err(e) => {
            println!("Error parsing file {:?}: {}", source, e);
            exit(1);
        }
    };

    if print_ast {
        println!("--- Ast ---");
        println!("{:?}", ast);
    }

    // println!("--- Css ---");
    // css::generate(&mut stdout(), &ast, &css::Settings {
    //     block_name: &block_name,
    //     vars: &Default::default(),
    //     }).unwrap();

    let css_text = if css_load {
        let mut buf = Vec::new();
        css::generate(&mut buf, &ast, &css::Settings {
            block_name: &block_name,
            vars: &Default::default(),
            }).unwrap();
        let string = String::from_utf8(buf).unwrap();
        if print_ast {
            println!("--- CSS ---");
            println!("{}", string);
        }
        Some(string)
    } else {
        None
    };
    if let Some(filename) = output_js {
        let sourcepath = source.with_extension("");
        let settings = es5citojs::Settings {
            block_name: &block_name[..],
            css_text: css_text.as_ref().map(|x| &x[..]),
            use_amd: use_amd,
            amd_name: amd_name.as_ref().map(|x| &x[..]).unwrap_or(
                sourcepath.to_str().unwrap()),
        };
        let res;
        if Path::new(&filename) == Path::new("-") {
            res = es5citojs::generate(&mut BufWriter::new(stdout()),
                                      &ast, &settings);
        } else {
            let mut file = match File::create(&filename).map(BufWriter::new) {
                Ok(f) => f,
                Err(e) => {
                    println!("Error opening file {:?}: {}", filename, e);
                    exit(2);
                }
            };
            res = es5citojs::generate(&mut file, &ast, &settings);
        }
        if let Err(err) = res {
            println!("{}", err);
            exit(1);
        }
    }
}

