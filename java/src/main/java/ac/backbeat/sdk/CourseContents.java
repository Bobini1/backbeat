package ac.backbeat.sdk;

import java.time.Instant;
import java.util.List;
import java.util.Objects;

/**
 * A course is an ordered list of charts.
 */
public final class CourseContents {
    private final String name;
    private final Instant updated;
    private final String gamemode;
    private final List<Tag> tags;
    private final List<Asset> assets;
    private final List<CourseContentsChart> charts;

    public CourseContents(String name, Instant updated, String gamemode, List<Tag> tags,
                          List<Asset> assets, List<CourseContentsChart> charts) {
        this.name = name; this.updated = updated; this.gamemode = gamemode;
        this.tags = List.copyOf(tags); this.assets = List.copyOf(assets); this.charts = List.copyOf(charts);
    }
    public String name() { return name; }
    public Instant updated() { return updated; }
    public String gamemode() { return gamemode; }
    public List<Tag> tags() { return tags; }
    public List<Asset> assets() { return assets; }
    public List<CourseContentsChart> charts() { return charts; }

    @Override public boolean equals(Object other) { return other instanceof CourseContents value && Objects.equals(name, value.name) && Objects.equals(updated, value.updated) && Objects.equals(gamemode, value.gamemode) && Objects.equals(tags, value.tags) && Objects.equals(assets, value.assets) && Objects.equals(charts, value.charts); }
    @Override public int hashCode() { return Objects.hash(name, updated, gamemode, tags, assets, charts); }
    @Override public String toString() { return "CourseContents[name=" + name + ", updated=" + updated + ", gamemode=" + gamemode + ", tags=" + tags + ", assets=" + assets + ", charts=" + charts + "]"; }
}
