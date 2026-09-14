package ac.backbeat.sdk;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.ByteByReference;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.LongByReference;
import com.sun.jna.ptr.PointerByReference;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * Access this user's backbeat store. Read what they've got installed and jam it into your
 * game.
 */
public final class Store implements AutoCloseable {
    private Pointer handle;

    private Store(Pointer handle) { this.handle = handle; }

    /**
     * Open the store.
     */
    public static Store open() {
        PointerByReference result = new PointerByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_open(result), "open store");
        return new Store(result.getValue());
    }

    /**
     * Get where config is stored. Note that this location is immutable and only varies
     * from OS to OS. Users can't edit where their config is stored (this allows games to
     * hook in without littering `config.toml`s in everyones folders).
     */
    public Path configDir() {
        return Path.of(ownedString(value -> NativeBindings.INSTANCE.bkb_store_config_dir(requireOpen(), value),
                "read config directory"));
    }

    /**
     * Get where this user is storing their data. You don't need to access this, but it's provided
     * for completeness' sake. Might be visually useful.
     */
    public Path storeDir() {
        return Path.of(ownedString(value -> NativeBindings.INSTANCE.bkb_store_dir(requireOpen(), value),
                "read store directory"));
    }

    /**
     * Get where backbeat debug logs are being written to.
     */
    public Path logsDir() {
        return Path.of(ownedString(value -> NativeBindings.INSTANCE.bkb_store_logs_dir(requireOpen(), value),
                "read logs directory"));
    }

    /**
     * Get the read-only URL you should connect to, to connect to backbeat's sqlite database.
     *
     * The tables in the backbeat.db file are considered part of the public API, although writing
     * to them directly is **extremely forbidden**, and I have a rogue instance of deepseek programmed
     * to scan github for people doing it (and kill them)
     */
    public String sqliteConnectionUrl() {
        return ownedString(value -> NativeBindings.INSTANCE.bkb_store_sqlite_connection_url(requireOpen(), value),
                "read SQLite connection URL");
    }

    /**
     * Eval this directly on your sqlite connection to "ATTACH $backbeat_url AS backbeat".
     *
     * This makes all of the backbeat tables read-only accessible under e.g. `backbeat.bundles`.
     */
    public String sqliteAttachCommand() {
        return ownedString(value -> NativeBindings.INSTANCE.bkb_store_sqlite_attach_command(requireOpen(), value),
                "read SQLite attach command");
    }

    /**
     * Eval this directly on your sqlite connection to detach the backbeat database.
     *
     * Not sure why you'd need this, but for completeness' sake.
     */
    public String sqliteDetachCommand() {
        return ownedString(value -> NativeBindings.INSTANCE.bkb_store_sqlite_detach_command(requireOpen(), value),
                "read SQLite detach command");
    }

    /**
     * Get some basic statistics about this user's backbeat store.
     */
    public StoreStats stats() {
        NativeBindings.BkbStats value = new NativeBindings.BkbStats();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_stats(requireOpen(), value), "read store statistics");
        value.read();
        return new StoreStats(value.charts, value.tables, value.courses, value.packs,
                value.assetCount, value.assetBytes, value.dbBytes);
    }

    /**
     * Given a bundle ID, get the `BackbeatFile` that the user has installed for it.
     * Returns None if not installed.
     */
    public Optional<BackbeatFile> getBundle(String bundleId) {
        return getBackbeatFile(result -> NativeBindings.INSTANCE.bkb_store_get_bundle(requireOpen(), bundleId, result),
                "open bundle " + bundleId);
    }

    /**
     * Given a chart ID, get _a_ `BackbeatFile` that the user has installed for it.
     *
     * In the cases of multiple matches (i.e. chart exists twice but with different assets),
     * this function will pick a matching chart indeterminately.
     */
    public Optional<BackbeatFile> getChart(String chartId) {
        return getBackbeatFile(result -> NativeBindings.INSTANCE.bkb_store_get_chart(requireOpen(), chartId, result),
                "open chart " + chartId);
    }

    private Optional<BackbeatFile> getBackbeatFile(NativeBackbeatFileCall call, String operation) {
        PointerByReference result = new PointerByReference();
        int code = call.invoke(result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, operation);
        return Optional.of(new BackbeatFile(this, result.getValue()));
    }

    /**
     * Given a chart ID, retrieve just the chart data, if installed.
     */
    public Optional<byte[]> getChartData(String chartId) {
        NativeBindings.BkbBytes result = new NativeBindings.BkbBytes();
        int code = NativeBindings.INSTANCE.bkb_store_get_chart_data(requireOpen(), chartId, result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, "read chart data " + chartId);
        result.read();
        try {
            return Optional.of(NativeSupport.bytes(result));
        } finally {
            NativeBindings.INSTANCE.bkb_bytes_free(result.byValue());
        }
    }

    /**
     * Given an asset ID, get the contents for that asset, if installed.
     */
    public Optional<AssetData> getAsset(String assetId) {
        return assetData(data -> NativeBindings.INSTANCE.bkb_store_get_asset(requireOpen(), assetId, data),
                "read asset " + assetId);
    }

    /**
     * Use `BackbeatFile.resolveAsset` instead, it's easier.
     */
    public Optional<AssetData> resolveAsset(String bundleId, String relativePath) {
        return assetData(data -> NativeBindings.INSTANCE.bkb_store_resolve_path(
                requireOpen(), bundleId, relativePath, data),
                "resolve asset " + relativePath + " in bundle " + bundleId);
    }

    /**
     * Search installed bundles, returning one page of results.
     */
    public BundleSearchResult searchBundles(String query, long offset, int limit, String... extensions) {
        if (offset < 0) throw new IllegalArgumentException("offset must not be negative");
        if (limit < 1) throw new IllegalArgumentException("limit must be positive");

        String[] filters = filters(extensions);
        PointerByReference result = new PointerByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_search_bundles(requireOpen(), query,
                offset, limit, filters,
                new NativeBindings.SizeT(filters == null ? 0 : filters.length), result), "search bundles");
        Pointer pointer = result.getValue();
        try {
            NativeBindings.BkbBundleSearchResult page = new NativeBindings.BkbBundleSearchResult(pointer);
            int count = NativeSupport.count(page.chartsLen);
            List<BundleSearchChart> charts = new ArrayList<>(count);
            if (count != 0) {
                NativeBindings.BkbBundleSearchChart[] values =
                        (NativeBindings.BkbBundleSearchChart[]) new NativeBindings.BkbBundleSearchChart(page.charts).toArray(count);
                for (NativeBindings.BkbBundleSearchChart value : values) {
                    charts.add(new BundleSearchChart(
                            NativeSupport.borrowedString(value.bundleId),
                            NativeSupport.borrowedString(value.description),
                            NativeSupport.optionalBorrowedString(value.extension)));
                }
            }
            return new BundleSearchResult(charts, page.hasMore != 0, page.total);
        } finally {
            NativeBindings.INSTANCE.bkb_bundle_search_result_free(pointer);
        }
    }

    private Optional<AssetData> assetData(NativeAssetCall call, String operation) {
        NativeBindings.BkbAssetData data = new NativeBindings.BkbAssetData();
        int code = call.invoke(data);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, operation);
        data.read();
        try {
            return switch (data.kind) {
                case NativeBindings.ASSET_DATA_BYTES -> Optional.of(new AssetData.Bytes(NativeSupport.bytes(data.value.bytes)));
                case NativeBindings.ASSET_DATA_FILE -> Optional.of(new AssetData.File(
                        Path.of(NativeSupport.ownedStringValue(data.value.file))));
                default -> throw new BackbeatException("Unknown Backbeat asset representation: " + data.kind, null);
            };
        } finally {
            NativeBindings.INSTANCE.bkb_asset_data_free(data.byValue());
        }
    }

    /**
     * Get a _locally installed_ course. Note that `url` is the identifier for installed collections,
     * and that this function does not do any web access.
     */
    public Optional<CourseContents> getCourse(String url) {
        PointerByReference result = new PointerByReference();
        int code = NativeBindings.INSTANCE.bkb_store_get_course(requireOpen(), url, result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, "read course " + url);
        Pointer pointer = result.getValue();
        try {
            NativeBindings.BkbCourse value = new NativeBindings.BkbCourse(pointer);
            return Optional.of(new CourseContents(NativeSupport.borrowedString(value.name),
                    NativeSupport.instant(value.updated), NativeSupport.borrowedString(value.gamemode),
                    NativeSupport.tags(value.tags, value.tagsLen), NativeSupport.assets(value.assets, value.assetsLen),
                    NativeSupport.courseCharts(value.charts, value.chartsLen)));
        } finally {
            NativeBindings.INSTANCE.bkb_course_free(pointer);
        }
    }

    /**
     * Get a _locally installed_ pack. Note that `url` is the identifier for installed collections,
     * and that this function does not do any web access.
     */
    public Optional<PackContents> getPack(String url) {
        PointerByReference result = new PointerByReference();
        int code = NativeBindings.INSTANCE.bkb_store_get_pack(requireOpen(), url, result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, "read pack " + url);
        Pointer pointer = result.getValue();
        try {
            NativeBindings.BkbPack value = new NativeBindings.BkbPack(pointer);
            return Optional.of(new PackContents(NativeSupport.borrowedString(value.name),
                    NativeSupport.borrowedString(value.gamemode), NativeSupport.instant(value.updated),
                    NativeSupport.tags(value.tags, value.tagsLen), NativeSupport.assets(value.assets, value.assetsLen),
                    NativeSupport.packBundles(value.bundles, value.bundlesLen)));
        } finally {
            NativeBindings.INSTANCE.bkb_pack_free(pointer);
        }
    }

    /**
     * Get a _locally installed_ table. Note that `url` is the identifier for installed collections,
     * and that this function does not do any web access.
     */
    public Optional<TableContents> getTable(String url) {
        PointerByReference result = new PointerByReference();
        int code = NativeBindings.INSTANCE.bkb_store_get_table(requireOpen(), url, result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, "read table " + url);
        Pointer pointer = result.getValue();
        try {
            NativeBindings.BkbTable value = new NativeBindings.BkbTable(pointer);
            return Optional.of(new TableContents(NativeSupport.borrowedString(value.name),
                    NativeSupport.borrowedString(value.symbol), NativeSupport.borrowedString(value.gamemode),
                    NativeSupport.instant(value.updated), NativeSupport.tags(value.tags, value.tagsLen),
                    NativeSupport.assets(value.assets, value.assetsLen), NativeSupport.tableLevels(value.levels, value.levelsLen),
                    NativeSupport.tableFolders(value.folders, value.foldersLen)));
        } finally {
            NativeBindings.INSTANCE.bkb_table_free(pointer);
        }
    }

    /**
     * Get every bundle this user has installed, limited to the set of file extensions
     * you pass in (e.g. `listBundles("bms", "bme", "bml", "bmson")).
     */
    public List<String> listBundles(String... extensions) {
        String[] filters = filters(extensions);
        List<String> values = new ArrayList<>();
        long offset = 0;
        boolean hasMore;
        do {
            BundleSearchResult page = searchBundles(null, offset, 100, filters);
            for (BundleSearchChart chart : page.charts()) values.add(chart.bundleId());
            offset += page.charts().size();
            hasMore = page.hasMore();
        }
        while (hasMore);
        return List.copyOf(values);
    }

    /**
     * List all the tables this user has installed, limited to only the gamemodes you specify,
     * e.g. `listTables("bms-7k", "bms-14k", "pms-9k")`.
     */
    public List<CollectionMetadata> listTables(String... gamemodes) {
        return listCollections(gamemodes, (filters, count, result) -> NativeBindings.INSTANCE.bkb_store_list_tables(
                requireOpen(), filters, count, result), "list tables");
    }

    /**
     * List all the courses this user has installed, limited to only the gamemodes you specify,
     * e.g. `listCourses("bms-7k", "bms-14k", "pms-9k")`.
     */
    public List<CollectionMetadata> listCourses(String... gamemodes) {
        return listCollections(gamemodes, (filters, count, result) -> NativeBindings.INSTANCE.bkb_store_list_courses(
                requireOpen(), filters, count, result), "list courses");
    }

    /**
      * List all the tables this user has installed, limited to only the gamemodes you specify,
      * e.g. `listTables("bms-7k", "bms-14k", "pms-9k")`.
      */
    public List<CollectionMetadata> listPacks(String... gamemodes) {
        return listCollections(gamemodes, (filters, count, result) -> NativeBindings.INSTANCE.bkb_store_list_packs(
                requireOpen(), filters, count, result), "list packs");
    }

    private List<CollectionMetadata> listCollections(String[] values, NativeCollectionListCall call,
                                                               String operation) {
        String[] filters = filters(values);
        PointerByReference result = new PointerByReference();
        NativeSupport.check(call.invoke(filters, new NativeBindings.SizeT(filters == null ? 0 : filters.length), result),
                operation);
        Pointer pointer = result.getValue();
        try {
            NativeBindings.BkbCollectionMetadataList list = new NativeBindings.BkbCollectionMetadataList(pointer);
            int count = NativeSupport.count(list.itemsLen);
            if (count == 0) return List.of();
            NativeBindings.BkbCollectionMetadata[] items = (NativeBindings.BkbCollectionMetadata[])
                    new NativeBindings.BkbCollectionMetadata(list.items).toArray(count);
            List<CollectionMetadata> metadata = new ArrayList<>(count);
            for (NativeBindings.BkbCollectionMetadata item : items) {
                metadata.add(new CollectionMetadata(NativeSupport.borrowedString(item.url),
                        NativeSupport.borrowedString(item.name), NativeSupport.borrowedString(item.gamemode),
                        NativeSupport.instant(item.updated), item.installed, item.outOf));
            }
            return List.copyOf(metadata);
        } finally {
            NativeBindings.INSTANCE.bkb_collection_metadata_list_free(pointer);
        }
    }

    /**
     * Is this asset installed?
     */
    public boolean hasAsset(String assetId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_asset(requireOpen(), assetId, out), "check asset " + assetId); }
    /**
     * Is this bundle installed?
     */
    public boolean hasBundle(String bundleId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_bundle(requireOpen(), bundleId, out), "check bundle " + bundleId); }
    /**
     * Is there a bundle installed, that has this chart ID?
     */
    public boolean hasChart(String chartId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_chart(requireOpen(), chartId, out), "check chart " + chartId); }
    /**
     * Is any collection (pack, table, course) installed with this URL?
     */
    public boolean hasCollection(String url) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_collection(requireOpen(), url, out), "check collection " + url); }
    /**
     * Is any a course installed with this URL?
     */
    public boolean hasCourse(String url) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_course(requireOpen(), url, out), "check course " + url); }
    /**
     * Is any a pack installed with this URL?
     */
    public boolean hasPack(String url) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_pack(requireOpen(), url, out), "check pack " + url); }
    /**
     * Is any a table installed with this URL?
     */
    public boolean hasTable(String url) { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_table(requireOpen(), url, out), "check table " + url); }

    /**
     * Does this user have absolutely zero data servers installed?
     *
     * This is worth checking, as the user should probably be warned about this - all in game
     * downloads will fail.
     */
    public boolean hasZeroDataServers() { return bool(out -> NativeBindings.INSTANCE.bkb_store_has_zero_data_servers(requireOpen(), out), "check data servers"); }

    /**
     * A weird manual method you don't need. Manually import a file from the filesystem into this
     * users store as an asset.
     */
    public void importAsset(Path path) { check(NativeBindings.INSTANCE.bkb_store_import_asset(requireOpen(), absolute(path)), "import asset " + path); }
    /**
     * A weird manual method you don't need. Manually install a `.bbzip` file from the filesystem
     * into this users store.
     */
    public void importBbzip(Path path) { check(NativeBindings.INSTANCE.bkb_store_import_bbzip(requireOpen(), absolute(path)), "import bbzip " + path); }
    /**
     * A weird manual method you don't need. Manually install a bundle into the store.
     */
    public String importBundle(BackbeatFile bundle) {
        NativeBindings.BkbString result = new NativeBindings.BkbString();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_import_bundle(requireOpen(), bundle.requireOpen(), result),
                "import bundle " + bundle.filename());
        result.read();
        try { return NativeSupport.ownedStringValue(result); }
        finally { NativeBindings.INSTANCE.bkb_string_free(result.byValue()); }
    }

    /**
     * Add a new server to this user's set of servers. This is idempotent, so feel free to
     * call this at startup if you want to register new servers for a user automagically.
     */
    public void serverAdd(String url) { check(NativeBindings.INSTANCE.bkb_store_server_add(requireOpen(), new NativeBindings.BkbServerConfig(url)), "add server " + url); }
    /**
     * Remove a server from this user's set of servers. Don't be an asshole with this power.
     */
    public void serverRm(String url) { check(NativeBindings.INSTANCE.bkb_store_server_rm(requireOpen(), new NativeBindings.BkbServerConfig(url)), "remove server " + url); }
    /**
     * Trigger a download of an asset with this ID.
     */
    public void serverDownloadAsset(String assetId) { check(NativeBindings.INSTANCE.bkb_store_server_download_asset(requireOpen(), assetId), "download asset " + assetId); }
    /**
     * Trigger a download of a bundle with this ID.
     */
    public void serverDownloadBundle(String bundleId) { check(NativeBindings.INSTANCE.bkb_store_server_download_bundle(requireOpen(), bundleId), "download bundle " + bundleId); }
    /**
     * Trigger a download of a chart with this ID.
     */
    public void serverDownloadChart(String chartId) { check(NativeBindings.INSTANCE.bkb_store_server_download_chart(requireOpen(), chartId), "download chart " + chartId); }
    /**
     * Download all the assets that this bundle references.
     */
    public void bundleDownloadAssets(String bundleId) { check(NativeBindings.INSTANCE.bkb_store_bundle_download_assets(requireOpen(), bundleId), "download bundle assets " + bundleId); }
    /**
     * Remove a bundle from this user's store.
     */
    public void bundleRm(String bundleId) { check(NativeBindings.INSTANCE.bkb_store_bundle_rm(requireOpen(), bundleId), "remove bundle " + bundleId); }

    /**
     * If you're downloading this thing, what's the progress of it?
     */
    public Optional<DownloadProgress> downloadProgress(DataId dataId) {
        NativeBindings.BkbOptionalDownloadProgress value = new NativeBindings.BkbOptionalDownloadProgress();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_download_progress(requireOpen(), nativeDataId(dataId), value),
                "read download progress " + dataId.value());
        value.read();
        if (value.isSome == 0) return Optional.empty();
        return Optional.of(NativeSupport.downloadProgress(value.value));
    }

    /**
     * Snapshot every download currently retained by the download manager.
     */
    public List<DownloadSnapshot> downloadAllProgress() {
        NativeBindings.BkbDownloadSnapshotList value = new NativeBindings.BkbDownloadSnapshotList();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_download_all_progress(requireOpen(), value),
                "read all download progress");
        value.read();
        try {
            return NativeSupport.downloadSnapshots(value.items, value.itemsLen);
        } finally {
            NativeBindings.INSTANCE.bkb_download_snapshot_list_free(value.byValue());
        }
    }

    /**
     * Get aggregate counts for all downloads retained by the download manager.
     */
    public DownloadOverview downloadOverview() {
        NativeBindings.BkbDownloadOverview value = new NativeBindings.BkbDownloadOverview();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_download_overview(requireOpen(), value),
                "read download overview");
        value.read();
        try {
            return new DownloadOverview(value.total, value.queued, value.running, value.failed, value.done,
                    value.cancelled, NativeSupport.optionalOwnedString(value.firstError));
        } finally {
            NativeBindings.INSTANCE.bkb_download_overview_free(value.byValue());
        }
    }

    /**
     * Get one page of all downloads, including child asset transfers.
     */
    public DownloadListResult downloadList(long offset, int limit) {
        return downloadList(offset, limit, false);
    }

    /**
     * Get one page of chart and bundle downloads, excluding child asset transfers.
     */
    public DownloadListResult collectionDownloadList(long offset, int limit) {
        return downloadList(offset, limit, true);
    }

    private DownloadListResult downloadList(long offset, int limit, boolean collectionsOnly) {
        if (offset < 0) throw new IllegalArgumentException("offset must not be negative");
        if (limit < 0) throw new IllegalArgumentException("limit must not be negative");
        NativeBindings.BkbDownloadListResult value = new NativeBindings.BkbDownloadListResult();
        int code = collectionsOnly
                ? NativeBindings.INSTANCE.bkb_store_collection_download_list(requireOpen(), offset, limit, value)
                : NativeBindings.INSTANCE.bkb_store_download_list(requireOpen(), offset, limit, value);
        NativeSupport.check(code, collectionsOnly ? "list collection downloads" : "list downloads");
        value.read();
        try {
            return new DownloadListResult(value.total, value.queued, value.running, value.failed, value.done,
                    value.cancelled, NativeSupport.optionalOwnedString(value.firstError),
                    NativeSupport.downloadSnapshots(value.downloads, value.downloadsLen), value.hasMore != 0);
        } finally {
            NativeBindings.INSTANCE.bkb_download_list_result_free(value.byValue());
        }
    }

    /**
     * Cancel this download.
     */
    public boolean downloadCancel(DataId dataId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_download_cancel(requireOpen(), nativeDataId(dataId), out), "cancel download " + dataId.value()); }
    /**
     * Cancel this asset download.
     */
    public boolean downloadAssetCancel(String assetId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_download_asset_cancel(requireOpen(), assetId, out), "cancel asset download " + assetId); }
    /**
     * Cancel this bundle download.
     */
    public boolean downloadBundleCancel(String bundleId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_download_bundle_cancel(requireOpen(), bundleId, out), "cancel bundle download " + bundleId); }
    /**
     * Cancel this chart download.
     */
    public boolean downloadChartCancel(String chartId) { return bool(out -> NativeBindings.INSTANCE.bkb_store_download_chart_cancel(requireOpen(), chartId, out), "cancel chart download " + chartId); }

    /**
     * Prune all finished downloads from the download manager's reports.
     */
    public long downloadClearFinished() {
        NativeBindings.SizeTByReference count = new NativeBindings.SizeTByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_download_clear_finished(requireOpen(), count),
                "clear finished downloads");
        return count.getValue().longValue();
    }

    /**
     * Download all the content referenced by this collection. This is the "download the table"
     * function call.
     */
    public CollectionDownloadDataReport collectionFetchDownloadData(String url) {
        NativeBindings.BkbCollectionDownloadDataReport value = new NativeBindings.BkbCollectionDownloadDataReport();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_collection_fetch_download_data(requireOpen(), url, value),
                "download collection data " + url);
        value.read();
        try {
            int count = NativeSupport.count(value.errorsLen);
            List<CollectionDownloadDataFailure> errors = new ArrayList<>(count);
            if (count > 0) {
                NativeBindings.BkbCollectionDownloadDataFailure[] failures =
                        (NativeBindings.BkbCollectionDownloadDataFailure[]) new NativeBindings.BkbCollectionDownloadDataFailure(value.errors).toArray(count);
                for (NativeBindings.BkbCollectionDownloadDataFailure failure : failures) {
                    errors.add(new CollectionDownloadDataFailure(
                            new DataId(DataId.Kind.fromNative(failure.item.kind),
                                    NativeSupport.ownedStringValue(failure.item.value)),
                            NativeSupport.ownedStringValue(failure.message)));
                }
            }
            return new CollectionDownloadDataReport(value.chartsDownloaded, value.chartsSkipped, value.chartsFailed,
                    value.bundlesDownloaded, value.bundlesSkipped, value.bundlesFailed,
                    value.assetsDownloaded, value.assetsSkipped, value.assetsFailed, errors);
        } finally {
            NativeBindings.INSTANCE.bkb_collection_download_data_report_free(value.byValue());
        }
    }

    /**
     * Remove a collection that this user has installed.
     */
    public CollectionKind collectionRm(String url, boolean removeChartsToo) {
        IntByReference kind = new IntByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_collection_rm(requireOpen(), url,
                        (byte) (removeChartsToo ? 1 : 0), kind),
                "remove collection " + url);
        return CollectionKind.fromNative(kind.getValue());
    }

    /** Return whether the store changed since the supplied revision. */
    public RefreshStatus shouldRefresh(long lastRevision) {
        ByteByReference shouldRefresh = new ByteByReference();
        LongByReference revision = new LongByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_store_should_refresh(
                requireOpen(), lastRevision, shouldRefresh, revision), "read store revision");
        return new RefreshStatus(shouldRefresh.getValue() != 0, revision.getValue());
    }

    Pointer requireOpen() {
        if (handle == null) throw new IllegalStateException("Backbeat store is closed");
        return handle;
    }

    private String ownedString(NativeStringCall call, String operation) {
        NativeBindings.BkbString value = new NativeBindings.BkbString();
        NativeSupport.check(call.invoke(value), operation);
        value.read();
        try { return NativeSupport.ownedStringValue(value); }
        finally { NativeBindings.INSTANCE.bkb_string_free(value.byValue()); }
    }

    private boolean bool(NativeBoolCall call, String operation) {
        ByteByReference result = new ByteByReference();
        NativeSupport.check(call.invoke(result), operation);
        return result.getValue() != 0;
    }

    private static NativeBindings.BkbDataId nativeDataId(DataId dataId) {
        return new NativeBindings.BkbDataId(dataId.kind().nativeValue(), dataId.value());
    }

    private static String[] filters(String[] values) {
        return values == null || values.length == 0 ? null : values.clone();
    }

    private static String absolute(Path path) { return path.toAbsolutePath().normalize().toString(); }
    private static void check(int code, String operation) { NativeSupport.check(code, operation); }

    @Override
    public void close() {
        if (handle != null) {
            NativeBindings.INSTANCE.bkb_store_free(handle);
            handle = null;
        }
    }

    @FunctionalInterface private interface NativeStringCall { int invoke(NativeBindings.BkbString value); }
    @FunctionalInterface private interface NativeAssetCall { int invoke(NativeBindings.BkbAssetData value); }
    @FunctionalInterface private interface NativeBackbeatFileCall { int invoke(PointerByReference value); }
    @FunctionalInterface private interface NativeBoolCall { int invoke(ByteByReference value); }
    @FunctionalInterface private interface NativeCollectionListCall {
        int invoke(String[] filters, NativeBindings.SizeT count, PointerByReference result);
    }
}
