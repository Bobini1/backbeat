package ac.backbeat.sdk;

import java.util.List;
import java.util.Objects;

public final class CollectionDownloadDataReport {
    private final long chartsDownloaded;
    private final long chartsSkipped;
    private final long chartsFailed;
    private final long bundlesDownloaded;
    private final long bundlesSkipped;
    private final long bundlesFailed;
    private final long assetsDownloaded;
    private final long assetsSkipped;
    private final long assetsFailed;
    private final List<CollectionDownloadDataFailure> errors;

    public CollectionDownloadDataReport(long chartsDownloaded, long chartsSkipped, long chartsFailed,
                                        long bundlesDownloaded, long bundlesSkipped, long bundlesFailed,
                                        long assetsDownloaded, long assetsSkipped, long assetsFailed,
                                        List<CollectionDownloadDataFailure> errors) {
        this.chartsDownloaded = chartsDownloaded; this.chartsSkipped = chartsSkipped; this.chartsFailed = chartsFailed;
        this.bundlesDownloaded = bundlesDownloaded; this.bundlesSkipped = bundlesSkipped; this.bundlesFailed = bundlesFailed;
        this.assetsDownloaded = assetsDownloaded; this.assetsSkipped = assetsSkipped; this.assetsFailed = assetsFailed;
        this.errors = List.copyOf(errors);
    }
    public long chartsDownloaded() { return chartsDownloaded; }
    public long chartsSkipped() { return chartsSkipped; }
    public long chartsFailed() { return chartsFailed; }
    public long bundlesDownloaded() { return bundlesDownloaded; }
    public long bundlesSkipped() { return bundlesSkipped; }
    public long bundlesFailed() { return bundlesFailed; }
    public long assetsDownloaded() { return assetsDownloaded; }
    public long assetsSkipped() { return assetsSkipped; }
    public long assetsFailed() { return assetsFailed; }
    public List<CollectionDownloadDataFailure> errors() { return errors; }

    // I'd cry if I couldn't laugh
    @Override public boolean equals(Object other) { return other instanceof CollectionDownloadDataReport value && chartsDownloaded == value.chartsDownloaded && chartsSkipped == value.chartsSkipped && chartsFailed == value.chartsFailed && bundlesDownloaded == value.bundlesDownloaded && bundlesSkipped == value.bundlesSkipped && bundlesFailed == value.bundlesFailed && assetsDownloaded == value.assetsDownloaded && assetsSkipped == value.assetsSkipped && assetsFailed == value.assetsFailed && Objects.equals(errors, value.errors); }
    @Override public int hashCode() { return Objects.hash(chartsDownloaded, chartsSkipped, chartsFailed, bundlesDownloaded, bundlesSkipped, bundlesFailed, assetsDownloaded, assetsSkipped, assetsFailed, errors); }
    @Override public String toString() { return "CollectionDownloadDataReport[chartsDownloaded=" + chartsDownloaded + ", chartsSkipped=" + chartsSkipped + ", chartsFailed=" + chartsFailed + ", bundlesDownloaded=" + bundlesDownloaded + ", bundlesSkipped=" + bundlesSkipped + ", bundlesFailed=" + bundlesFailed + ", assetsDownloaded=" + assetsDownloaded + ", assetsSkipped=" + assetsSkipped + ", assetsFailed=" + assetsFailed + ", errors=" + errors + "]"; }
}
