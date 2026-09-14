package ac.backbeat.sdk;

import java.util.Objects;

/**
 * Basic info about the user's backbeat store.
 */
public final class StoreStats {
    private final long charts;
    private final long tables;
    private final long courses;
    private final long packs;
    private final long assetCount;
    private final long assetBytes;
    private final long dbBytes;

    public StoreStats(long charts, long tables, long courses, long packs, long assetCount, long assetBytes, long dbBytes) {
        this.charts = charts; this.tables = tables; this.courses = courses; this.packs = packs;
        this.assetCount = assetCount; this.assetBytes = assetBytes; this.dbBytes = dbBytes;
    }
    public long charts() { return charts; }
    public long tables() { return tables; }
    public long courses() { return courses; }
    public long packs() { return packs; }
    public long assetCount() { return assetCount; }
    public long assetBytes() { return assetBytes; }
    public long dbBytes() { return dbBytes; }

    @Override public boolean equals(Object other) {
        return other instanceof StoreStats value && charts == value.charts && tables == value.tables
                && courses == value.courses && packs == value.packs && assetCount == value.assetCount
                && assetBytes == value.assetBytes && dbBytes == value.dbBytes;
    }
    @Override public int hashCode() { return Objects.hash(charts, tables, courses, packs, assetCount, assetBytes, dbBytes); }
    @Override public String toString() { return "StoreStats[charts=" + charts + ", tables=" + tables + ", courses=" + courses + ", packs=" + packs + ", assetCount=" + assetCount + ", assetBytes=" + assetBytes + ", dbBytes=" + dbBytes + "]"; }
}
