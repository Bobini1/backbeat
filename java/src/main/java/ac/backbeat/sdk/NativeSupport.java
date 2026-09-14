package ac.backbeat.sdk;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;

/**
 * Bunch of utility functions for dealing with FFI.
 */
final class NativeSupport {
    private NativeSupport() {}

    static void check(int code, String operation) {
        if (code == NativeBindings.OK) return;
        String error = NativeBindings.INSTANCE.bkb_error_string(code).getString(0, "UTF-8");
        throw new BackbeatException(code, operation + ": " + error);
    }

    static int count(NativeBindings.SizeT value) {
        return Math.toIntExact(value.longValue());
    }

    static String borrowedString(NativeBindings.BkbStr value) {
        if (value.ptr == null) return "";
        return new String(value.ptr.getByteArray(0, count(value.len)), StandardCharsets.UTF_8);
    }

    static Optional<String> optionalBorrowedString(NativeBindings.BkbStr value) {
        return value.ptr == null ? Optional.empty() : Optional.of(borrowedString(value));
    }

    static String ownedStringValue(NativeBindings.BkbString value) {
        if (value.ptr == null) return "";
        return new String(value.ptr.getByteArray(0, count(value.len)), StandardCharsets.UTF_8);
    }

    static byte[] bytes(NativeBindings.BkbBytes value) {
        return bytes(value.ptr, value.len);
    }

    static byte[] bytes(Pointer pointer, NativeBindings.SizeT length) {
        return pointer == null ? new byte[0] : pointer.getByteArray(0, count(length));
    }

    static Instant instant(NativeBindings.BkbTimestamp timestamp) {
        return Instant.ofEpochSecond(timestamp.seconds, Integer.toUnsignedLong(timestamp.nanoseconds));
    }

    static OptionalLong optionalLong(NativeBindings.BkbOptionalU64 value) {
        return value.isSome == 0 ? OptionalLong.empty() : OptionalLong.of(value.value);
    }

    static Optional<String> optionalOwnedString(NativeBindings.BkbString value) {
        return value.ptr == null ? Optional.empty() : Optional.of(ownedStringValue(value));
    }

    static DownloadProgress downloadProgress(NativeBindings.BkbDownloadProgress value) {
        return new DownloadProgress(value.bytes, optionalLong(value.total), value.itemsDone,
                optionalLong(value.itemsTotal), DownloadState.fromNative(value.state));
    }

    static List<DownloadSnapshot> downloadSnapshots(Pointer pointer, NativeBindings.SizeT length) {
        int count = count(length);
        if (count == 0) return List.of();
        NativeBindings.BkbDownloadSnapshot[] values = array(pointer, length, NativeBindings.BkbDownloadSnapshot::new);
        List<DownloadSnapshot> snapshots = new ArrayList<>(values.length);
        for (NativeBindings.BkbDownloadSnapshot value : values) {
            snapshots.add(new DownloadSnapshot(
                    new DataId(DataId.Kind.fromNative(value.key.kind), ownedStringValue(value.key.value)),
                    downloadProgress(value.progress), optionalOwnedString(value.error)));
        }
        return List.copyOf(snapshots);
    }

    static List<Tag> tags(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbTag[] values = array(pointer, length, NativeBindings.BkbTag::new);
        List<Tag> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbTag value : values) {
            result.add(new Tag(borrowedString(value.key), borrowedString(value.value)));
        }
        return List.copyOf(result);
    }

    static List<Asset> assets(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbAsset[] values = array(pointer, length, NativeBindings.BkbAsset::new);
        List<Asset> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbAsset value : values) {
            result.add(new Asset(borrowedString(value.path), borrowedString(value.id)));
        }
        return List.copyOf(result);
    }

    static List<CourseContentsChart> courseCharts(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbCourseChart[] values = array(pointer, length, NativeBindings.BkbCourseChart::new);
        List<CourseContentsChart> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbCourseChart value : values) {
            result.add(new CourseContentsChart(borrowedString(value.id), borrowedString(value.desc),
                    tags(value.tags, value.tagsLen), optionalBorrowedString(value.bundleId)));
        }
        return List.copyOf(result);
    }

    static List<PackContentsBundle> packBundles(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbPackBundle[] values = array(pointer, length, NativeBindings.BkbPackBundle::new);
        List<PackContentsBundle> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbPackBundle value : values) {
            result.add(new PackContentsBundle(borrowedString(value.id), borrowedString(value.desc),
                    tags(value.tags, value.tagsLen), value.installed != 0));
        }
        return List.copyOf(result);
    }

    static List<TableContentsChart> tableCharts(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbTableChart[] values = array(pointer, length, NativeBindings.BkbTableChart::new);
        List<TableContentsChart> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbTableChart value : values) {
            result.add(new TableContentsChart(borrowedString(value.id), borrowedString(value.desc),
                    tags(value.tags, value.tagsLen), optionalBorrowedString(value.bundleId)));
        }
        return List.copyOf(result);
    }

    static List<TableContentsLevel> tableLevels(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbTableLevel[] values = array(pointer, length, NativeBindings.BkbTableLevel::new);
        List<TableContentsLevel> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbTableLevel value : values) {
            result.add(new TableContentsLevel(borrowedString(value.level), tags(value.tags, value.tagsLen),
                    tableCharts(value.charts, value.chartsLen)));
        }
        return List.copyOf(result);
    }

    static List<TableContentsFolder> tableFolders(Pointer pointer, NativeBindings.SizeT length) {
        if (count(length) == 0) return List.of();
        NativeBindings.BkbTableFolder[] values = array(pointer, length, NativeBindings.BkbTableFolder::new);
        List<TableContentsFolder> result = new ArrayList<>(values.length);
        for (NativeBindings.BkbTableFolder value : values) {
            result.add(new TableContentsFolder(borrowedString(value.name), borrowedString(value.query),
                    tags(value.tags, value.tagsLen), tableCharts(value.charts, value.chartsLen)));
        }
        return List.copyOf(result);
    }

    @SuppressWarnings("unchecked")
    private static <T extends Structure> T[] array(Pointer pointer, NativeBindings.SizeT length,
                                                    StructureFactory<T> factory) {
        int count = count(length);
        T first = factory.create(pointer);
        return (T[]) first.toArray(count);
    }

    @FunctionalInterface
    private interface StructureFactory<T extends Structure> {
        T create(Pointer pointer);
    }
}
