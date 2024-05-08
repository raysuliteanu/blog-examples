package org.example;

import java.io.IOException;
import java.io.InputStream;
import java.io.PushbackInputStream;

import static java.lang.Character.isDigit;
import static java.lang.Character.isWhitespace;

/**
 * Simple parser for a grammar to specify addition/subtraction/multiplication/division/modulus.
 * <p>
 * Grammar:
 * <pre>
 *  ExecutableStatement: ConstExpr WS OpExpr WS ConstExpr
 *  ConstExpr: '[0-9]'+
 *  OpExpr: '+' | '-' | '*' | '/' | '%'
 *  WS: '[ ]'+
 * </pre>
 */
public class Parser {
    private final PushbackInputStream inputStream;

    public Parser(final InputStream inputStream) {
        assert inputStream != null;
        this.inputStream = new PushbackInputStream(inputStream);
    }

    public ExecutableStatement parse() {
        ExecutableStatement executableStatement;

        try {
            Expr.ConstExpr left = readConstant();
            char op = parseOperator();
            Expr.ConstExpr right = readConstant();

            Op opExpr = switch (op) {
                case '+' -> new Op.AddOp(left, right);
                case '-' -> new Op.SubOp(left, right);
                case '*' -> new Op.MulOp(left, right);
                case '/' -> new Op.DivOp(left, right);
                case '%' -> new Op.ModOp(left, right);
                default -> throw new ParseException("unknown operator: " + op);
            };

            executableStatement = ExecutableStatement.of(new Expr.ExecutableExpr(opExpr));
        }
        catch (NumberFormatException | IOException | ParseException e) {
            executableStatement = ExecutableStatement.of(new Expr.InvalidExpr(e.getMessage()));
        }

        return executableStatement;
    }

    private char parseOperator() throws IOException {
        skipWhitespace();

        int token = inputStream.read();

        isValidOp(token);

        return (char) token;
    }

    private Expr.ConstExpr readConstant() throws IOException {
        skipWhitespace();

        StringBuilder buffer = new StringBuilder();
        int token;
        while ((token = inputStream.read()) != -1) {
            if (isDigit(token)) {
                buffer.append((char) token);
            }
            else {
                inputStream.unread(token);
                break;
            }
        }

        if (buffer.isEmpty()) {
            throw new ParseException("constant missing");
        }

        return new Expr.ConstExpr(Integer.parseInt(buffer.toString()));
    }

    private void isValidOp(final int token) {
        switch (token) {
            case '+':
            case '-':
            case '*':
            case '/':
            case '%':
                break;
            default:
                throw new ParseException("unexpected token: " + (char) token);
        }
    }

    private void skipWhitespace() throws IOException {
        int token;
        while ((token = inputStream.read()) != -1) {
            if (!isWhitespace(token)) {
                inputStream.unread(token);
                break;
            }
        }

        checkEndOfInput(token);
    }

    private void checkEndOfInput(final int token) {
        if (token == -1) {
            throw new ParseException("unexpected end of input");
        }
    }
}
