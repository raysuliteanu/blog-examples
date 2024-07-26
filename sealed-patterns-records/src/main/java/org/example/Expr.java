package org.example;

public sealed interface Expr {
    record ConstExpr(int constant) implements Expr {
        @Override
        public String toString() {
            return String.valueOf(constant);
        }
    }

    record ExecutableExpr(Op op) implements Expr {
        @Override
        public String toString() {
            return op.toString();
        }
    }

    record InvalidExpr(String message) implements Expr {
        @Override
        public String toString() {
            return message();
        }
    }
}
