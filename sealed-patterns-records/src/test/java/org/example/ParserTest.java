package org.example;

import java.io.ByteArrayInputStream;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;

class ParserTest {
    @Test
    void parseEmpty() {
        final var executableStatement = new Parser(ByteArrayInputStream.nullInputStream()).parse();
        assertNotNull(executableStatement);
        assertInstanceOf(Expr.InvalidExpr.class, executableStatement.getExpression());
    }

    @Test
    void parseSimpleAdditionExpression() {
        var executableStatement = new Parser(new ByteArrayInputStream("1+1".getBytes())).parse();
        assertNotNull(executableStatement);

        var expression = executableStatement.getExpression();
        if (expression instanceof Expr.ExecutableExpr expr) {
            assertInstanceOf(Op.AddOp.class, expr.op());
        }

        executableStatement = new Parser(new ByteArrayInputStream(" 1  +  1 ".getBytes())).parse();
        assertNotNull(executableStatement);
        expression = executableStatement.getExpression();
        if (expression instanceof Expr.ExecutableExpr expr) {
            assertInstanceOf(Op.AddOp.class, expr.op());
        }

        assertEquals("1 + 1", expression.toString());
    }
}
