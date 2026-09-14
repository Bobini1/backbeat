package ac.backbeat.sdk;

import java.time.Instant;

import java.util.Objects;

/**
 * Basic information about a collection.
 */
public final class CollectionMetadata {
    private final String url;
    private final String name;
    private final String gamemode;
    private final Instant updated;
    private final long installed;
    private final long outOf;

    public CollectionMetadata(String url, String name, String gamemode, Instant updated, long installed, long outOf) {
        this.url = url; this.name = name; this.gamemode = gamemode; this.updated = updated; this.installed = installed; this.outOf = outOf;
    }
    public String url() { return url; }
    public String name() { return name; }
    public String gamemode() { return gamemode; }
    public Instant updated() { return updated; }
    public long installed() { return installed; }
    public long outOf() { return outOf; }

    @Override public boolean equals(Object other) {
        return other instanceof CollectionMetadata value && Objects.equals(url, value.url)
                && Objects.equals(name, value.name) && Objects.equals(gamemode, value.gamemode)
                && Objects.equals(updated, value.updated)
                && installed == value.installed && outOf == value.outOf;
    }
    @Override public int hashCode() { return Objects.hash(url, name, gamemode, updated, installed, outOf); }
    @Override public String toString() { return "CollectionMetadata[url=" + url + ", name=" + name + ", gamemode=" + gamemode + ", updated=" + updated + ", installed=" + installed + ", outOf=" + outOf + "]"; }
}
