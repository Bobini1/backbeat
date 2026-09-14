package ac.backbeat.sdk;

import java.util.List;
import java.util.Objects;

/**
 * Tables can (unlike in prior art in bms) specify extra folders in a language called
 * [tinyfilter](https://github.com/zkldi/tinyfilter). This allows for things like "Level 18, Marathons only".
 */
public final class TableContentsFolder {
    private final String name;
    private final String query;
    private final List<Tag> tags;
    private final List<TableContentsChart> charts;

    public TableContentsFolder(String name, String query, List<Tag> tags, List<TableContentsChart> charts) {
        this.name = name; this.query = query; this.tags = List.copyOf(tags); this.charts = List.copyOf(charts);
    }
    public String name() { return name; }
    public String query() { return query; }
    public List<Tag> tags() { return tags; }
    public List<TableContentsChart> charts() { return charts; }

    @Override public boolean equals(Object other) { return other instanceof TableContentsFolder value && Objects.equals(name, value.name) && Objects.equals(query, value.query) && Objects.equals(tags, value.tags) && Objects.equals(charts, value.charts); }
    @Override public int hashCode() { return Objects.hash(name, query, tags, charts); }
    @Override public String toString() { return "TableContentsFolder[name=" + name + ", query=" + query + ", tags=" + tags + ", charts=" + charts + "]"; }
}
