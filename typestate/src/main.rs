use std::{default::Default, marker::PhantomData};

#[derive(Default, Debug)]
pub struct Token<'c>(&'c str);
#[derive(Default, Debug)]
pub struct Ast<'c>(&'c str);

#[derive(Default)]
pub struct Scanner<'c>(&'c str);
#[derive(Default)]
pub struct Parser<'c>(Vec<Token<'c>>);
#[derive(Default)]
pub struct Evaluator<'c>(Vec<Ast<'c>>);
#[derive(Default)]
pub struct CompilerResult;

#[derive(Default)]
pub struct Compiler<S> {
    stage: S,
    print_tokens: bool,
    print_ast: bool,
}

impl<'compiler> Compiler<Scanner<'compiler>> {
    pub fn new(source: &'compiler str, print_tokens: bool, print_ast: bool) -> Self {
        Compiler {
            stage: Scanner(source),
            print_tokens,
            print_ast,
        }
    }

    pub fn scan(&self) -> Compiler<Parser> {
        let source = self.stage.0;
        eprintln!("scan");
        let tokens = source
            .split_whitespace()
            .map(Token)
            .inspect(|t| {
                if self.print_tokens {
                    println!("{t:?}");
                }
            })
            .collect::<Vec<Token<'_>>>();

        Compiler {
            stage: Parser(tokens),
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
        }
    }
}

impl Compiler<Parser<'_>> {
    pub fn parse(&self) -> Compiler<Evaluator> {
        eprintln!("parse");
        let ast = self
            .stage
            .0
            .iter()
            .map(|t| Ast(t.0))
            .inspect(|a| {
                if self.print_ast {
                    println!("{a:?}");
                }
            })
            .collect::<Vec<Ast<'_>>>();

        Compiler {
            stage: Evaluator(ast),
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
        }
    }
}

impl Compiler<Evaluator<'_>> {
    pub fn evaluate(&self) -> Compiler<CompilerResult> {
        eprintln!("evaluate");
        self.stage
            .0
            .iter()
            .map(|a| a.0)
            .for_each(|s| println!("{s}"));

        Compiler {
            stage: CompilerResult,
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
        }
    }
}

impl Compiler<CompilerResult> {
    pub fn finish(&self) {
        eprintln!("done");
    }
}

#[derive(Default)]
struct BuilderNoSource;
#[derive(Default)]
struct BuilderSource;

#[derive(Default)]
struct CompilerBuilder<'b, T> {
    source: Option<&'b str>,
    print_tokens: bool,
    print_ast: bool,
    marker: PhantomData<T>,
}

impl<'b> CompilerBuilder<'b, BuilderNoSource> {
    pub fn new() -> CompilerBuilder<'b, BuilderNoSource> {
        CompilerBuilder::default()
    }

    pub fn with_source(self, source: &str) -> CompilerBuilder<BuilderSource> {
        CompilerBuilder {
            source: Some(source),
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
            marker: PhantomData,
        }
    }
}

impl<'b, T> CompilerBuilder<'b, T> {
    pub fn print_tokens(self) -> Self {
        CompilerBuilder {
            print_tokens: true,
            ..self
        }
    }

    pub fn print_ast(self) -> Self {
        CompilerBuilder {
            print_ast: true,
            ..self
        }
    }
}

impl<'b> CompilerBuilder<'b, BuilderSource> {
    pub fn build(self) -> Compiler<Scanner<'b>> {
        Compiler::new(
            // SAFETY: by type state pattern, can only call build() if `self.source` has been set
            unsafe { self.source.unwrap_unchecked() },
            self.print_tokens,
            self.print_ast,
        )
    }
}

fn main() {
    CompilerBuilder::new()
        .print_tokens()
        .with_source("hello world this is a type state example")
        .print_ast()
        .build()
        .scan()
        .parse()
        .evaluate()
        .finish();
}
