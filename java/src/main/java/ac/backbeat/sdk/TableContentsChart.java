package ac.backbeat.sdk;

import java.util.List;
import java.util.Optional;
import java.util.Objects;

/**
 * A chart in a table level.
 */
public final class TableContentsChart {
    private final String id;
    private final String desc;
    private final List<Tag> tags;
    private final Optional<String> bundleId;

    public TableContentsChart(String id, String desc, List<Tag> tags, Optional<String> bundleId) {
        this.id = id; this.desc = desc; this.tags = List.copyOf(tags); this.bundleId = bundleId;
    }
    public String id() { return id; }
    public String desc() { return desc; }
    public List<Tag> tags() { return tags; }
    public Optional<String> bundleId() { return bundleId; }

    @Override public boolean equals(Object other) { return other instanceof TableContentsChart value && Objects.equals(id, value.id) && Objects.equals(desc, value.desc) && Objects.equals(tags, value.tags) && Objects.equals(bundleId, value.bundleId); }
    @Override public int hashCode() { return Objects.hash(id, desc, tags, bundleId); }
    @Override public String toString() { return "TableContentsChart[id=" + id + ", desc=" + desc + ", tags=" + tags + ", bundleId=" + bundleId + "]"; }
}
