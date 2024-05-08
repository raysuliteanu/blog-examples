package org.example;

public class ParseException extends RuntimeException {
    public ParseException() {
        super();
    }

    public ParseException(final String message) {
        super(message);
    }

    public ParseException(final String message, final Throwable cause) {
        super(message, cause);
    }

    public ParseException(final Throwable cause) {
        super(cause);
    }

    protected ParseException(final String message, final Throwable cause, final boolean enableSuppression, final boolean writableStackTrace) {
        super(message, cause, enableSuppression, writableStackTrace);
    }
}
