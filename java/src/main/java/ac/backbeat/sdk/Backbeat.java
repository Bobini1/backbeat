package ac.backbeat.sdk;

public final class Backbeat {
    public static final int LIB_VERSION = NativeBindings.LIBVERSION;
    public static final int SQLITE_MIN_VERSION_NUMBER = 3_038_000;

    private Backbeat() {}

    /**
     * Get the version of the backbeat library you're working with.
     */
    public static int libVersion() {
        return NativeBindings.INSTANCE.bkb_libversion();
    }

    /**
     * Get the commit hash for the backbeat library you're working with.
     */
    public static String libCommitHash() {
        return NativeBindings.INSTANCE.bkb_libcommithash().getString(0, "UTF-8");
    }

    /**
     * Return the version number of the SQLite implementation linked into Backbeat.
     */
    public static int sqliteVersionNumber() {
        return NativeBindings.INSTANCE.bkb_sqlite_version_number();
    }

    /**
     * Convert an error code from backbeat_c_sdk into reasonable human text.
     */
    public static String errorString(int code) {
        return NativeBindings.INSTANCE.bkb_error_string(code).getString(0, "UTF-8");
    }

    /**
     * Convert an error code from backbeat_c_sdk into reasonable human text.
     */
    public static String errorString(ErrorCode code) {
        return errorString(code.code());
    }
}
