package ac.backbeat.sdk;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;

import java.nio.file.Path;
import java.util.List;
import java.util.Optional;

/**
 * A BackbeatFile is chart data and "file->assetID" mappings.
 */
public final class BackbeatFile implements AutoCloseable {
	/**
	 * We hold on to a reference to the store so that `assetResolve` is ergonomic
	 * and not shit.
	 */
    private final Store store;
    private Pointer handle;
    private final String filename;
    private final List<Asset> assets;
    private final String desc;
    private final byte[] chartData;

    BackbeatFile(Store store, Pointer handle) {
        this.store = store;
        this.handle = handle;
        try {
            NativeBindings.BkbBb view = new NativeBindings.BkbBb(handle);
            filename = NativeSupport.borrowedString(view.filename);
            assets = NativeSupport.assets(view.assets, view.assetsLen);
            desc = NativeSupport.borrowedString(view.desc);
            chartData = NativeSupport.bytes(view.chart, view.chartLen);
        } catch (Throwable error) {
            NativeBindings.INSTANCE.bkb_bb_free(handle);
            this.handle = null;
            throw error;
        }
    }

    public static BackbeatFile fromFile(Path path) {
        PointerByReference result = new PointerByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_bb_from_file(
                path.toAbsolutePath().normalize().toString(), result), "read bundle file " + path);
        return new BackbeatFile(null, result.getValue());
    }

    public static BackbeatFile fromJson(byte[] json) {
        PointerByReference result = new PointerByReference();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_bb_from_json(
                json, new NativeBindings.SizeT(json.length), result), "parse bundle JSON");
        return new BackbeatFile(null, result.getValue());
    }

    public String filename() { return filename; }
    public List<Asset> assets() { return assets; }
    public String desc() { return desc; }
    public String bundleId() {
        Pointer bundle = requireOpen();
        return ownedString(value -> NativeBindings.INSTANCE.bkb_bb_bundle_id(bundle, value), "read bundle ID");
    }
    public String chartSha256() {
        Pointer bundle = requireOpen();
        return ownedString(value -> NativeBindings.INSTANCE.bkb_bb_chart_sha256(bundle, value), "read chart SHA-256");
    }
    public String combinedAssetsId() {
        Pointer bundle = requireOpen();
        return ownedString(value -> NativeBindings.INSTANCE.bkb_bb_combined_assets_id(bundle, value),
                "read combined assets ID");
    }
    public byte[] chartData() { return chartData.clone(); }

    public byte[] toJson() {
        NativeBindings.BkbBytes result = new NativeBindings.BkbBytes();
        NativeSupport.check(NativeBindings.INSTANCE.bkb_bb_to_json(requireOpen(), result), "serialize bundle JSON");
        result.read();
        try {
            return NativeSupport.bytes(result);
        } finally {
            NativeBindings.INSTANCE.bkb_bytes_free(result.byValue());
        }
    }

    private Optional<String> resolveAssetId(String relativePath) {
        NativeBindings.BkbString result = new NativeBindings.BkbString();
        int code = NativeBindings.INSTANCE.bkb_bb_resolve_path(requireOpen(), relativePath, result);
        if (code == NativeBindings.NOT_FOUND) return Optional.empty();
        NativeSupport.check(code, "resolve bundle path " + relativePath);
        result.read();
        try {
            return Optional.of(NativeSupport.ownedStringValue(result));
        } finally {
            NativeBindings.INSTANCE.bkb_string_free(result.byValue());
        }
    }

    /**
     * When this chart makes reference to a file, i.e. `foo.wav`, put it into this function
     * to get the actual underlying asset data. It is not an understatement to say this is the
     * most important function in all of backbeat.
     */
    public Optional<AssetData> resolveAsset(String relativePath) {
        Store associatedStore;
        Optional<String> assetId;

        // ensure the store hasn't been closed by some asshole
        requireOpen();
        associatedStore = store;
        // literally shouldn't happen
        if (associatedStore == null) {
            throw new IllegalStateException("Bundle is not associated with a Backbeat store");
        }
        assetId = resolveAssetId(relativePath);

        return assetId.flatMap(associatedStore::getAsset);
    }

    Pointer requireOpen() {
        if (handle == null) throw new IllegalStateException("Backbeat bundle is closed");
        return handle;
    }

    private String ownedString(NativeStringCall call, String operation) {
        NativeBindings.BkbString value = new NativeBindings.BkbString();
        NativeSupport.check(call.invoke(value), operation);
        value.read();
        try {
            return NativeSupport.ownedStringValue(value);
        } finally {
            NativeBindings.INSTANCE.bkb_string_free(value.byValue());
        }
    }

    @Override
    public void close() {
        if (handle != null) {
            NativeBindings.INSTANCE.bkb_bb_free(handle);
            handle = null;
        }
    }

    @FunctionalInterface
    private interface NativeStringCall { int invoke(NativeBindings.BkbString value); }
}
