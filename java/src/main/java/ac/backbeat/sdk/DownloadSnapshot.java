package ac.backbeat.sdk;

import java.util.Objects;
import java.util.Optional;

public final class DownloadSnapshot {
    private final DataId key;
    private final DownloadProgress progress;
    private final Optional<String> error;

    public DownloadSnapshot(DataId key, DownloadProgress progress, Optional<String> error) {
        this.key = Objects.requireNonNull(key, "key");
        this.progress = Objects.requireNonNull(progress, "progress");
        this.error = Objects.requireNonNull(error, "error");
    }

    public DataId key() { return key; }
    public DownloadProgress progress() { return progress; }
    public Optional<String> error() { return error; }

    @Override public boolean equals(Object other) {
        return other instanceof DownloadSnapshot value && Objects.equals(key, value.key)
                && Objects.equals(progress, value.progress) && Objects.equals(error, value.error);
    }
    @Override public int hashCode() { return Objects.hash(key, progress, error); }
    @Override public String toString() {
        return "DownloadSnapshot[key=" + key + ", progress=" + progress + ", error=" + error + "]";
    }
}
