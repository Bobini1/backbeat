package ac.backbeat.sdk;

import java.util.Objects;

/**
 * Boring three-piece enum. Some APIs in backbeat return either a chart, bundle or asset ID
 * (namely the download APIs).
 */
public final class DataId {
    private final Kind kind;
    private final String value;

    public DataId(Kind kind, String value) {
        this.kind = Objects.requireNonNull(kind, "kind");
        this.value = Objects.requireNonNull(value, "value");
    }

    public Kind kind() { return kind; }
    public String value() { return value; }

    public static DataId chart(String value) { return new DataId(Kind.CHART, value); }
    public static DataId bundle(String value) { return new DataId(Kind.BUNDLE, value); }
    public static DataId asset(String value) { return new DataId(Kind.ASSET, value); }

    @Override public boolean equals(Object other) {
        return other instanceof DataId dataId && kind == dataId.kind && value.equals(dataId.value);
    }
    @Override public int hashCode() { return Objects.hash(kind, value); }
    @Override public String toString() { return "DataId[kind=" + kind + ", value=" + value + "]"; }

    public enum Kind {
        CHART(NativeBindings.DATA_CHART),
        BUNDLE(NativeBindings.DATA_BUNDLE),
        ASSET(NativeBindings.DATA_ASSET);

        private final int nativeValue;
        Kind(int nativeValue) { this.nativeValue = nativeValue; }
        int nativeValue() { return nativeValue; }
        static Kind fromNative(int value) {
            for (Kind kind : values()) if (kind.nativeValue == value) return kind;
            throw new BackbeatException("Unknown Backbeat data kind: " + value, null);
        }
    }
}
