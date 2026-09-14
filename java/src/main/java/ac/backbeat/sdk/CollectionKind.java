package ac.backbeat.sdk;

/**
 * Some backbeat APIs return collection kinds - like collection metadata and whatnot.
 * This is just a three piece enum - TABLE or COURSE or PACK.
 */
public enum CollectionKind {
    TABLE(NativeBindings.COLLECTION_TABLE),
    COURSE(NativeBindings.COLLECTION_COURSE),
    PACK(NativeBindings.COLLECTION_PACK);

    private final int nativeValue;
    CollectionKind(int nativeValue) { this.nativeValue = nativeValue; }
    static CollectionKind fromNative(int value) {
        for (CollectionKind kind : values()) if (kind.nativeValue == value) return kind;
        throw new BackbeatException("Unknown Backbeat collection kind: " + value, null);
    }
}
