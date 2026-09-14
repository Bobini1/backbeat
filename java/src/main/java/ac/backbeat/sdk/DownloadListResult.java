package ac.backbeat.sdk;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

public final class DownloadListResult {
    private final long total;
    private final long queued;
    private final long running;
    private final long failed;
    private final long done;
    private final long cancelled;
    private final Optional<String> firstError;
    private final List<DownloadSnapshot> downloads;
    private final boolean hasMore;

    public DownloadListResult(long total, long queued, long running, long failed, long done, long cancelled,
                              Optional<String> firstError, List<DownloadSnapshot> downloads, boolean hasMore) {
        this.total = total; this.queued = queued; this.running = running; this.failed = failed;
        this.done = done; this.cancelled = cancelled;
        this.firstError = Objects.requireNonNull(firstError, "firstError");
        this.downloads = List.copyOf(downloads);
        this.hasMore = hasMore;
    }

    public long total() { return total; }
    public long queued() { return queued; }
    public long running() { return running; }
    public long failed() { return failed; }
    public long done() { return done; }
    public long cancelled() { return cancelled; }
    public Optional<String> firstError() { return firstError; }
    public List<DownloadSnapshot> downloads() { return downloads; }
    public boolean hasMore() { return hasMore; }

    @Override public boolean equals(Object other) {
        return other instanceof DownloadListResult value && total == value.total && queued == value.queued
                && running == value.running && failed == value.failed && done == value.done
                && cancelled == value.cancelled && hasMore == value.hasMore
                && Objects.equals(firstError, value.firstError) && Objects.equals(downloads, value.downloads);
    }
    @Override public int hashCode() {
        return Objects.hash(total, queued, running, failed, done, cancelled, firstError, downloads, hasMore);
    }
    @Override public String toString() {
        return "DownloadListResult[total=" + total + ", queued=" + queued + ", running=" + running
                + ", failed=" + failed + ", done=" + done + ", cancelled=" + cancelled
                + ", firstError=" + firstError + ", downloads=" + downloads + ", hasMore=" + hasMore + "]";
    }
}
