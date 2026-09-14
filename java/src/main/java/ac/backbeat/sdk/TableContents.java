package ac.backbeat.sdk;

import java.time.Instant;
import java.util.List;
import java.util.Objects;

/**
 * A table (Difficulty Table) is a mapping of charts to levels.
 */
public final class TableContents {
    private final String name;
    private final String symbol;
    private final String gamemode;
    private final Instant updated;
    private final List<Tag> tags;
    private final List<Asset> assets;
    private final List<TableContentsLevel> levels;
    private final List<TableContentsFolder> folders;

    public TableContents(String name, String symbol, String gamemode, Instant updated, List<Tag> tags,
                         List<Asset> assets, List<TableContentsLevel> levels, List<TableContentsFolder> folders) {
        this.name = name; this.symbol = symbol; this.gamemode = gamemode; this.updated = updated;
        this.tags = List.copyOf(tags); this.assets = List.copyOf(assets);
        this.levels = List.copyOf(levels); this.folders = List.copyOf(folders);
    }
    public String name() { return name; }
    public String symbol() { return symbol; }
    public String gamemode() { return gamemode; }
    public Instant updated() { return updated; }
    public List<Tag> tags() { return tags; }
    public List<Asset> assets() { return assets; }
    public List<TableContentsLevel> levels() { return levels; }
    public List<TableContentsFolder> folders() { return folders; }
    public long chartCount() {
        long count = 0;
        for (TableContentsLevel level : levels) count += level.charts().size();
        return count;
    }

    @Override public boolean equals(Object other) { return other instanceof TableContents value && Objects.equals(name, value.name) && Objects.equals(symbol, value.symbol) && Objects.equals(gamemode, value.gamemode) && Objects.equals(updated, value.updated) && Objects.equals(tags, value.tags) && Objects.equals(assets, value.assets) && Objects.equals(levels, value.levels) && Objects.equals(folders, value.folders); }
    @Override public int hashCode() { return Objects.hash(name, symbol, gamemode, updated, tags, assets, levels, folders); }
    @Override public String toString() { return "TableContents[name=" + name + ", symbol=" + symbol + ", gamemode=" + gamemode + ", updated=" + updated + ", tags=" + tags + ", assets=" + assets + ", levels=" + levels + ", folders=" + folders + "]"; }
}
