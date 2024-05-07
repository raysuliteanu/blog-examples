package org.example;

public sealed interface Expr {
    record ConstExpr(int constant) implements Expr {
    }

    record ExecutableExpr(Op op) implements Expr {
    }

    record InvalidExpr() implements Expr {
    }
}
