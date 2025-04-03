use std::default::Default;

#[derive(Default, Clone)]
pub struct Token;
#[derive(Default, Clone)]
pub struct Ast;

#[derive(Default, Clone)]
pub struct Scanner(&'static str);
#[derive(Default, Clone)]
pub struct Parser(Vec<Token>);
#[derive(Default, Clone)]
pub struct Evaluater(Vec<Ast>);
#[derive(Default, Clone)]
pub struct CompilerResult;

#[derive(Default, Clone)]
pub struct Compiler<S> {
    stage: S,
}

impl Compiler<Scanner> {
    pub fn new(source: &'static str) -> Self {
        Compiler {
            stage: Scanner(source),
        }
    }

    pub fn scan(&self) -> Compiler<Parser> {
        let _ = self.stage.0;
        Compiler {
            stage: Parser(Vec::new()),
        }
    }
}

impl Compiler<Parser> {
    pub fn parse(&self) -> Compiler<Evaluater> {
        for _t in &self.stage.0 {}
        Compiler {
            stage: Evaluater(Vec::new()),
        }
    }
}

impl Compiler<Evaluater> {
    pub fn evaluate(&self) -> Compiler<CompilerResult> {
        for _t in &self.stage.0 {}
        Compiler {
            stage: CompilerResult,
        }
    }
}

fn main() {
    let _compiler = Compiler::new("hello").scan().parse().evaluate();
}
