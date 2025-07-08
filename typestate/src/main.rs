use crate::compiler::CompilerBuilder;

mod compiler;

fn main() {
    let compiler = CompilerBuilder::new()
        .print_tokens()
        .with_source("hello world this is a type state example")
        .print_ast()
        .build();

    compiler.scan().parse().evaluate().finish();

    let scanner = compiler.scan();
    let parser = scanner.parse();
    let ev = parser.evaluate();
    ev.finish();
}
