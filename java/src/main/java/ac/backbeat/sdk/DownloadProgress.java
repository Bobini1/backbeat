package ac.backbeat.sdk;

import java.util.OptionalLong;
import java.util.Objects;

public final class DownloadProgress {
    private final long bytes;
    private final OptionalLong total;
    private final long itemsDone;
    private final OptionalLong itemsTotal;
    private final DownloadState state;

    public DownloadProgress(long bytes, OptionalLong total, long itemsDone, OptionalLong itemsTotal, DownloadState state) {
        this.bytes = bytes; this.total = total; this.itemsDone = itemsDone; this.itemsTotal = itemsTotal; this.state = state;
    }
    public long bytes() { return bytes; }
    public OptionalLong total() { return total; }
    public long itemsDone() { return itemsDone; }
    public OptionalLong itemsTotal() { return itemsTotal; }
    public DownloadState state() { return state; }

    @Override public boolean equals(Object other) {
        return other instanceof DownloadProgress value && bytes == value.bytes && itemsDone == value.itemsDone
                && Objects.equals(total, value.total) && Objects.equals(itemsTotal, value.itemsTotal) && state == value.state;
    }
    @Override public int hashCode() { return Objects.hash(bytes, total, itemsDone, itemsTotal, state); }
    @Override public String toString() { return "DownloadProgress[bytes=" + bytes + ", total=" + total + ", itemsDone=" + itemsDone + ", itemsTotal=" + itemsTotal + ", state=" + state + "]"; }
}
