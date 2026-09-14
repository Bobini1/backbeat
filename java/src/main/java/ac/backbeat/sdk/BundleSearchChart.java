package ac.backbeat.sdk;

import java.util.Optional;

/**
 * One bundle returned by a store search.
 */
public record BundleSearchChart(String bundleId, String description, Optional<String> extension) {}
