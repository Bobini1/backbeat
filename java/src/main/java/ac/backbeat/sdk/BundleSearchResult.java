package ac.backbeat.sdk;

import java.util.List;

/**
 * One page of bundles returned by a store search.
 */
public record BundleSearchResult(List<BundleSearchChart> charts, boolean hasMore, long total) {
    public BundleSearchResult {
        charts = List.copyOf(charts);
    }
}
