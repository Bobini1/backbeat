package ac.backbeat.sdk;

import java.util.List;
import java.util.Objects;

/**
 * A level in a difficulty table.
 */
public final class TableContentsLevel {
    private final String level;
    private final List<Tag> tags;
    private final List<TableContentsChart> charts;

    public TableContentsLevel(String level, List<Tag> tags, List<TableContentsChart> charts) {
        this.level = level; this.tags = List.copyOf(tags); this.charts = List.copyOf(charts);
    }
    public String level() { return level; }
    public List<Tag> tags() { return tags; }
    public List<TableContentsChart> charts() { return charts; }

    @Override public boolean equals(Object other) { return other instanceof TableContentsLevel value && Objects.equals(level, value.level) && Objects.equals(tags, value.tags) && Objects.equals(charts, value.charts); }
    @Override public int hashCode() { return Objects.hash(level, tags, charts); }
    @Override public String toString() { return "TableContentsLevel[level=" + level + ", tags=" + tags + ", charts=" + charts + "]"; }
}
