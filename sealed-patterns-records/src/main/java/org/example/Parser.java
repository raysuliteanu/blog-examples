package org.example;

import java.io.IOException;
import java.io.InputStream;
import java.io.PushbackInputStream;
import java.util.ArrayDeque;
import java.util.Set;
import java.util.function.BiConsumer;
import java.util.function.BinaryOperator;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.stream.Collector;

import static java.lang.Character.isDigit;
import static java.lang.Character.isWhitespace;

public class Parser {
    private final PushbackInputStream inputStream;

    public Parser(final InputStream inputStream) {
        assert inputStream != null;
        this.inputStream = new PushbackInputStream(inputStream);
    }

    public ExecutableStatement parse() {
        try {
            var constExprs = new ArrayDeque<Expr.ConstExpr>();
            var mathExprs = new ArrayDeque<Op>();
            var token = inputStream.read();
            while (token != -1) {
                token = skipWhitespace(token);

                if (isDigit(token)) {
                    constExprs.add(readConstant(token));
                }
                else {
                    switch (token) {
                        case '+':
                            if (constExprs.isEmpty()) {
                                throw new RuntimeException("parse error"); // change to custom ParseException
                            }

                            if ((token = inputStream.read()) != -1) {
                                token = skipWhitespace(token);

                                if (!isDigit(token)) {
                                    throw new RuntimeException("parse error"); // change to custom ParseException
                                }
                                else {
                                    constExprs.add(readConstant(token));
                                }
                            }

                            mathExprs.add(new Op.AddOp(constExprs.poll(), constExprs.poll()));

                            break;
                        case '-':
                        case '*':
                        case '/':
                        case '%':
                    }
                }

                token = inputStream.read();
            }

            return mathExprs.isEmpty() ?
                   ExecutableStatement.of(new Expr.InvalidExpr()) :
                   ExecutableStatement.of(new Expr.ExecutableExpr(mathExprs.poll()));
        }
        catch (IOException e) {
            throw new RuntimeException();
        }

    }

    private Expr.ConstExpr readConstant(int token) throws IOException {
        token = skipWhitespace(token);

        var characters = new ArrayDeque<Character>();
        characters.add((char) token);

        while ((token = inputStream.read()) != -1) {
            if (isDigit(token)) {
                characters.add((char) token);
            }
            else {
                inputStream.unread(token);
                break;
            }
        }

        return makeConstantExpr(characters);

    }

    private int skipWhitespace(int token) throws IOException {
        if (isWhitespace(token)) {
            do {
                token = inputStream.read();
            } while (isWhitespace(token));
        }
        return token;
    }

    private static Expr.ConstExpr makeConstantExpr(final ArrayDeque<Character> characters) {
        final var value = characters.stream().collect(new CharactersToStringCollector());
        characters.clear();
        return new Expr.ConstExpr(Integer.parseInt(value));
    }

    static class CharactersToStringCollector implements Collector<Character, StringBuilder, String> {
        @Override
        public Supplier<StringBuilder> supplier() {
            return StringBuilder::new;
        }

        @Override
        public BiConsumer<StringBuilder, Character> accumulator() {
            return StringBuilder::append;
        }

        @Override
        public BinaryOperator<StringBuilder> combiner() {
            return StringBuilder::append;
        }

        @Override
        public Function<StringBuilder, String> finisher() {
            return StringBuilder::toString;
        }

        @Override
        public Set<Characteristics> characteristics() {
            return Set.of();
        }
    }
}
