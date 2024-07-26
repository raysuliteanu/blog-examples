package org.example;

public sealed interface Op {
    record AddOp(Expr left, Expr right) implements Op {
        @Override
        public String toString() {
            return left + " + " + right;
        }
    }

    record SubOp(Expr left, Expr right) implements Op {
        @Override
        public String toString() {
            return left + " - " + right;
        }
    }

    record MulOp(Expr left, Expr right) implements Op {
        @Override
        public String toString() {
            return left + " * " + right;
        }
    }

    record DivOp(Expr left, Expr right) implements Op {
        @Override
        public String toString() {
            return left + " / " + right;
        }
    }

    record ModOp(Expr left, Expr right) implements Op {
        @Override
        public String toString() {
            return left + " % " + right;
        }
    }
}
