package ac.backbeat.sdk;

import java.util.List;
import java.util.Objects;

/**
 * A bundle entry in an installed pack.
 */
public final class PackContentsBundle {
    private final String id;
    private final String desc;
    private final List<Tag> tags;
    private final boolean installed;

    public PackContentsBundle(String id, String desc, List<Tag> tags, boolean installed) {
        this.id = id; this.desc = desc; this.tags = List.copyOf(tags); this.installed = installed;
    }
    public String id() { return id; }
    public String desc() { return desc; }
    public List<Tag> tags() { return tags; }
    public boolean installed() { return installed; }

    @Override public boolean equals(Object other) { return other instanceof PackContentsBundle value && Objects.equals(id, value.id) && Objects.equals(desc, value.desc) && Objects.equals(tags, value.tags) && installed == value.installed; }
    @Override public int hashCode() { return Objects.hash(id, desc, tags, installed); }
    @Override public String toString() { return "PackContentsBundle[id=" + id + ", desc=" + desc + ", tags=" + tags + ", installed=" + installed + "]"; }
}
