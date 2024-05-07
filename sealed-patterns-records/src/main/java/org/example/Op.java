package org.example;

public sealed interface Op {
    record AddOp(Expr left, Expr right) implements Op {
    }

    record SubOp(Expr left, Expr right) implements Op {
    }

    record MulOp(Expr left, Expr right) implements Op {
    }

    record DivOp(Expr left, Expr right) implements Op {
    }

    record ModOp(Expr left, Expr right) implements Op {
    }

    record NegOp(Expr expr) implements Op {
    }
}
