use std::{env, fs};
use std::error::Error;
use std::path::Path;

use lazy_static::lazy_static;
use tree_sitter_highlight::{Highlight, Highlighter, HtmlRenderer};
use tree_sitter_highlight::HighlightConfiguration;

lazy_static!(
static ref  HIGHLIGHT_NAMES: Vec<&'static str> = vec![
        "attribute",
        "constant",
        "function.builtin",
        "function",
        "keyword",
        "number",
        "operator",
        "property",
        "punctuation",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "string.special",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ];
);

fn main() -> Result<(), Box<dyn Error>> {
    let args : Vec<_> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage: {} some-rust-file", args.get(0).unwrap());
    }

    let source = fs::read_to_string(Path::new(args.get(1).unwrap()))?;

    let mut highlighter = Highlighter::new();
    let rust_lang = tree_sitter_rust::language();

    let mut rust_config = HighlightConfiguration::new(
        rust_lang,
        "rust",
        tree_sitter_rust::HIGHLIGHT_QUERY,
        "",
        "",
    ).unwrap();

    rust_config.configure(&HIGHLIGHT_NAMES);

    let highlights = highlighter.highlight(
        &rust_config,
        source.as_bytes(),
        None,
        |_| None,
    ).unwrap();

    let mut renderer = HtmlRenderer::new();

    match renderer.render(highlights, source.as_bytes(), &attr_callback) {
        Ok(_) => {
            for line in renderer.lines() {
                println!("{line}");
            }
            Ok(())
        },
        Err(e) => {
            eprintln!("{e}");
            Err(Box::try_from(e).unwrap())
        }
    }
}

fn attr_callback<'a>(h: Highlight) -> &'a [u8] {
    let highlight_type = dbg!(HIGHLIGHT_NAMES[h.0]);
    match highlight_type {
        "keyword" => "color='blue'",
        "constant" => "color='red'",
        "punctuation" | "punctuation.delimiter" | "punctuation.bracket" => "color='yellow'",
        &_ => ""
    }.as_bytes()
}
