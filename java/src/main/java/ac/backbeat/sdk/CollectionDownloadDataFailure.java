package ac.backbeat.sdk;

import java.util.Objects;

public final class CollectionDownloadDataFailure {
    private final DataId item;
    private final String message;

    public CollectionDownloadDataFailure(DataId item, String message) { this.item = item; this.message = message; }
    public DataId item() { return item; }
    public String message() { return message; }

    @Override public boolean equals(Object other) {
        return other instanceof CollectionDownloadDataFailure value && Objects.equals(item, value.item)
                && Objects.equals(message, value.message);
    }
    @Override public int hashCode() { return Objects.hash(item, message); }
    @Override public String toString() { return "CollectionDownloadDataFailure[item=" + item + ", message=" + message + "]"; }
}
