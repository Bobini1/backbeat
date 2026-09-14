package ac.backbeat.sdk;

import java.util.Optional;

public final class BackbeatException extends RuntimeException {
    private final int code;

    BackbeatException(int code, String message) {
        super(message);
        this.code = code;
    }

    BackbeatException(String message, Throwable cause) {
        super(message, cause);
        // idk lol
        this.code = -1;
    }

    public int code() {
        return code;
    }

    public Optional<ErrorCode> errorCode() {
        return ErrorCode.fromCode(code);
    }
}
