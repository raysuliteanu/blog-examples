use std::{default::Default, error::Error, marker::PhantomData};

#[derive(Default, Clone)]
pub struct Token<'c>(&'c str);
#[derive(Default, Clone)]
pub struct Ast<'c>(&'c str);

#[derive(Default, Clone)]
pub struct Scanner<'c>(&'c str);
#[derive(Default, Clone)]
pub struct Parser<'c>(Vec<Token<'c>>);
#[derive(Default, Clone)]
pub struct Evaluater<'c>(Vec<Ast<'c>>);
#[derive(Default, Clone)]
pub struct CompilerResult;

#[derive(Default, Clone)]
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
            .collect::<Vec<Token<'_>>>();

        Compiler {
            stage: Parser(tokens),
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
        }
    }
}

impl Compiler<Parser<'_>> {
    pub fn parse(&self) -> Compiler<Evaluater> {
        eprintln!("parse");
        let ast = self
            .stage
            .0
            .iter()
            .map(|t| Ast(t.0))
            .collect::<Vec<Ast<'_>>>();

        Compiler {
            stage: Evaluater(ast),
            print_tokens: self.print_tokens,
            print_ast: self.print_ast,
        }
    }
}

impl Compiler<Evaluater<'_>> {
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
struct BuilderInit;
#[derive(Default)]
struct BuilderSource;

#[derive(Default)]
struct CompilerBuilder<'b, T> {
    source: Option<&'b str>,
    print_tokens: bool,
    print_ast: bool,
    marker: PhantomData<T>,
}

impl<'b> CompilerBuilder<'b, BuilderInit> {
    pub fn new() -> Self {
        CompilerBuilder::default()
    }
}

impl CompilerBuilder<'_, BuilderInit> {
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
            source: self.source,
            print_tokens: true,
            print_ast: self.print_ast,
            marker: PhantomData,
        }
    }

    pub fn print_ast(self) -> Self {
        CompilerBuilder {
            source: self.source,
            print_tokens: self.print_tokens,
            print_ast: true,
            marker: PhantomData,
        }
    }
}

impl<'b> CompilerBuilder<'b, BuilderSource> {
    pub fn build(self) -> Result<Compiler<Scanner<'b>>, Box<dyn Error>> {
        if let Some(source) = self.source {
            Ok(Compiler::new(source, self.print_tokens, self.print_ast))
        } else {
            todo!("missing source");
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let compiler = CompilerBuilder::new()
        .print_tokens()
        .with_source("hello world this is a type state example")
        .print_ast()
        .build()?;

    compiler.scan().parse().evaluate().finish();

    Ok(())
}
