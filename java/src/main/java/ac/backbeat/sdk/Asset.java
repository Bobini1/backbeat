package ac.backbeat.sdk;

import java.util.Objects;

/**
 * Maps a filepath to the sha256 of that asset.
 */
public final class Asset {
    private final String path;
    private final String id;

    public Asset(String path, String id) { this.path = path; this.id = id; }
    public String path() { return path; }
    public String id() { return id; }

    @Override public boolean equals(Object other) {
        return other instanceof Asset value && Objects.equals(path, value.path) && Objects.equals(id, value.id);
    }
    @Override public int hashCode() { return Objects.hash(path, id); }
    @Override public String toString() { return "Asset[path=" + path + ", id=" + id + "]"; }
}
