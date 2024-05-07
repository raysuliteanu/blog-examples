package org.example;

public class ExecutableStatement {
    private final Expr expression;

    public static ExecutableStatement of(Expr expression) {
        return new ExecutableStatement(expression);
    }

    private ExecutableStatement(final Expr expression) {
        this.expression = expression;
    }

    public Expr getExpression() {
        return expression;
    }

    public void execute() {
        // todo!
    }

    @Override
    public String toString() {
        return expression.toString();
    }
}
