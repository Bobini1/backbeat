package ac.backbeat.sdk;

import java.util.Optional;

public enum ErrorCode {
    DB(NativeBindings.ERR_DB),
    CORRUPT(NativeBindings.ERR_CORRUPT),
    MIGRATE(NativeBindings.ERR_MIGRATE),
    IO(NativeBindings.ERR_IO),
    CONFIG(NativeBindings.ERR_CONFIG),
    JSON(NativeBindings.ERR_JSON),
    INVALID_COLLECTION_HEADER(NativeBindings.ERR_INVALID_COLLECTION_HEADER),
    INVALID_BUNDLE(NativeBindings.ERR_INVALID_BUNDLE),
    TOO_LARGE(NativeBindings.ERR_TOO_LARGE),
    ZIP(NativeBindings.ERR_ZIP),
    NESTED_BBZIP(NativeBindings.ERR_NESTED_BBZIP),
    PARSE(NativeBindings.ERR_PARSE),
    NOT_FOUND(NativeBindings.NOT_FOUND),
    HASH_MISMATCH(NativeBindings.ERR_HASH_MISMATCH),
    INVALID_PRECOMBINED_ASSETS(NativeBindings.ERR_INVALID_PRECOMBINED_ASSETS),
    INVALID_ASSET_PATH(NativeBindings.ERR_INVALID_ASSET_PATH),
    ID_MISMATCH(NativeBindings.ERR_ID_MISMATCH),
    NO_SERVERS(NativeBindings.ERR_NO_SERVERS),
    INVALID_URL(NativeBindings.ERR_INVALID_URL),
    NETWORK(NativeBindings.ERR_NETWORK),
    UNKNOWN_ALGORITHM(NativeBindings.ERR_UNKNOWN_ALGORITHM),
    PERMISSION_DENIED(NativeBindings.ERR_PERMISSION_DENIED),
    CANCELLED(NativeBindings.ERR_CANCELLED),
    REMOTE_BAD_STATUS_CODE(NativeBindings.ERR_REMOTE_BAD_STATUS_CODE),
    DOWNLOAD_TASK_FAILED(NativeBindings.ERR_DOWNLOAD_TASK_FAILED),
    NULL_ARG(NativeBindings.ERR_NULL_ARG),
    PANIC(NativeBindings.ERR_PANIC),
    INVALID_STRING(NativeBindings.ERR_INVALID_STRING),
    INCOMPATIBLE_SQLITE(NativeBindings.ERR_INCOMPATIBLE_SQLITE);

    private final int code;

    ErrorCode(int code) {
        this.code = code;
    }

    public int code() {
        return code;
    }

    public static Optional<ErrorCode> fromCode(int code) {
        for (ErrorCode value : values()) {
            if (value.code == code) return Optional.of(value);
        }
        return Optional.empty();
    }
}
