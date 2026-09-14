package ac.backbeat.sdk;

import java.time.Instant;
import java.util.List;
import java.util.Objects;

/**
 * The contents of an installed pack.
 */
public final class PackContents {
    private final String name;
    private final String gamemode;
    private final Instant updated;
    private final List<Tag> tags;
    private final List<Asset> assets;
    private final List<PackContentsBundle> bundles;

    public PackContents(String name, String gamemode, Instant updated, List<Tag> tags, List<Asset> assets, List<PackContentsBundle> bundles) {
        this.name = name; this.gamemode = gamemode; this.updated = updated;
        this.tags = List.copyOf(tags); this.assets = List.copyOf(assets); this.bundles = List.copyOf(bundles);
    }
    public String name() { return name; }
    public String gamemode() { return gamemode; }
    public Instant updated() { return updated; }
    public List<Tag> tags() { return tags; }
    public List<Asset> assets() { return assets; }
    public List<PackContentsBundle> bundles() { return bundles; }

    @Override public boolean equals(Object other) { return other instanceof PackContents value && Objects.equals(name, value.name) && Objects.equals(gamemode, value.gamemode) && Objects.equals(updated, value.updated) && Objects.equals(tags, value.tags) && Objects.equals(assets, value.assets) && Objects.equals(bundles, value.bundles); }
    @Override public int hashCode() { return Objects.hash(name, gamemode, updated, tags, assets, bundles); }
    @Override public String toString() { return "PackContents[name=" + name + ", gamemode=" + gamemode + ", updated=" + updated + ", tags=" + tags + ", assets=" + assets + ", bundles=" + bundles + "]"; }
}
