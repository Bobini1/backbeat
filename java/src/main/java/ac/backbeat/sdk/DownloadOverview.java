package ac.backbeat.sdk;

import java.util.Objects;
import java.util.Optional;

public final class DownloadOverview {
    private final long total;
    private final long queued;
    private final long running;
    private final long failed;
    private final long done;
    private final long cancelled;
    private final Optional<String> firstError;

    public DownloadOverview(long total, long queued, long running, long failed, long done, long cancelled,
                            Optional<String> firstError) {
        this.total = total; this.queued = queued; this.running = running; this.failed = failed;
        this.done = done; this.cancelled = cancelled;
        this.firstError = Objects.requireNonNull(firstError, "firstError");
    }

    public long total() { return total; }
    public long queued() { return queued; }
    public long running() { return running; }
    public long failed() { return failed; }
    public long done() { return done; }
    public long cancelled() { return cancelled; }
    public Optional<String> firstError() { return firstError; }

    @Override public boolean equals(Object other) {
        return other instanceof DownloadOverview value && total == value.total && queued == value.queued
                && running == value.running && failed == value.failed && done == value.done
                && cancelled == value.cancelled && Objects.equals(firstError, value.firstError);
    }
    @Override public int hashCode() {
        return Objects.hash(total, queued, running, failed, done, cancelled, firstError);
    }
    @Override public String toString() {
        return "DownloadOverview[total=" + total + ", queued=" + queued + ", running=" + running
                + ", failed=" + failed + ", done=" + done + ", cancelled=" + cancelled
                + ", firstError=" + firstError + "]";
    }
}
