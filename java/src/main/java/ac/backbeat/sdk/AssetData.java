package ac.backbeat.sdk;

import java.nio.file.Path;
import java.util.Arrays;
import java.util.Objects;

/**
 * AssetData is either a path to somewhere on your filesystem, or an in memory array of bytes.
 *
 * Backbeat stores small files in SQLite for a massive, massive performance uplift, so you
 * need to handle both cases.
 */
public sealed interface AssetData permits AssetData.Bytes, AssetData.File {
    final class Bytes implements AssetData {
        private final byte[] data;

        public Bytes(byte[] data) {
            this.data = data.clone();
        }

        public byte[] data() {
            return data.clone();
        }

        @Override public boolean equals(Object other) {
            return other instanceof Bytes bytes && Arrays.equals(data, bytes.data);
        }
        @Override public int hashCode() { return Arrays.hashCode(data); }
        @Override public String toString() { return "Bytes[data=" + Arrays.toString(data) + "]"; }
    }

    final class File implements AssetData {
        private final Path path;

        public File(Path path) {
            this.path = path.toAbsolutePath().normalize();
        }

        public Path path() { return path; }

        @Override public boolean equals(Object other) {
            return other instanceof File file && Objects.equals(path, file.path);
        }
        @Override public int hashCode() { return Objects.hashCode(path); }
        @Override public String toString() { return "File[path=" + path + "]"; }
    }
}
