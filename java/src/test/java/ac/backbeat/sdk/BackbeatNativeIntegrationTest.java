package ac.backbeat.sdk;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.io.TempDir;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.sql.DriverManager;
import java.util.Base64;
import java.util.HexFormat;
import java.util.List;
import java.util.zip.GZIPOutputStream;

import static org.junit.jupiter.api.Assertions.*;

@EnabledIfSystemProperty(named = "backbeat.integration", matches = "true")
class BackbeatNativeIntegrationTest {
    @TempDir Path temporaryDirectory;

    @Test
    void standaloneBundlesWork() throws Exception {
        assertEquals(Backbeat.LIB_VERSION, Backbeat.libVersion());
        assertFalse(Backbeat.libCommitHash().isBlank());
        assertTrue(Backbeat.sqliteVersionNumber() >= Backbeat.SQLITE_MIN_VERSION_NUMBER);
        assertFalse(Backbeat.errorString(ErrorCode.NOT_FOUND).isBlank());

        byte[] chart = "#TITLE Java SDK\n".getBytes(StandardCharsets.UTF_8);
        byte[] asset = "asset bytes".getBytes(StandardCharsets.UTF_8);
        String assetId = sha256(asset);
        byte[] json = bundleJson(chart, assetId);

        try (BackbeatFile standalone = BackbeatFile.fromJson(json)) {
            assertEquals("chart.bms", standalone.filename());
            assertEquals("Java SDK test bundle", standalone.desc());
            assertArrayEquals(chart, standalone.chartData());
            assertFalse(standalone.bundleId().isBlank());
            assertFalse(standalone.chartSha256().isBlank());
            assertFalse(standalone.combinedAssetsId().isBlank());
            assertTrue(standalone.toJson().length > 0);

            Path bundleFile = temporaryDirectory.resolve("chart.bb");
            Files.write(bundleFile, standalone.toJson());
            try (BackbeatFile fromFile = BackbeatFile.fromFile(bundleFile)) {
                assertEquals(standalone.bundleId(), fromFile.bundleId());
            }
        }
    }

    @Test
    void zeroByteChartWorksThroughStandaloneAndStoreApis() throws Exception {
        Path fixture = Path.of("..", "fixtures", "bb", "zero-byte-chart.bb")
                .toAbsolutePath().normalize();
        byte[] json = Files.readAllBytes(fixture);

        try (BackbeatFile standalone = BackbeatFile.fromFile(fixture);
             BackbeatFile fromJson = BackbeatFile.fromJson(json)) {
            assertArrayEquals(new byte[0], standalone.chartData());
            assertArrayEquals(new byte[0], fromJson.chartData());
            try (BackbeatFile roundtrip = BackbeatFile.fromJson(standalone.toJson())) {
                assertArrayEquals(new byte[0], roundtrip.chartData());
            }

            String chartId = "sha256/" + standalone.chartSha256();
            try (Store store = Store.open()) {
                String bundleId = store.importBundle(standalone);
                assertArrayEquals(new byte[0], store.getChartData(chartId).orElseThrow());
                try (BackbeatFile bundle = store.getBundle(bundleId).orElseThrow();
                     BackbeatFile chart = store.getChart(chartId).orElseThrow()) {
                    assertArrayEquals(new byte[0], bundle.chartData());
                    assertArrayEquals(new byte[0], chart.chartData());
                }
            }
        }
    }

    @Test
    void synchronousStoreMethodsWork() throws Exception {
        byte[] chart = ("#TITLE Java SDK " + temporaryDirectory + "\n").getBytes(StandardCharsets.UTF_8);
        byte[] asset = ("asset bytes " + temporaryDirectory).getBytes(StandardCharsets.UTF_8);
        String assetId = sha256(asset);
        try (BackbeatFile bundle = BackbeatFile.fromJson(bundleJson(chart, assetId))) {
            Path assetFile = temporaryDirectory.resolve("test.wav");
            Files.write(assetFile, asset);
            try (Store store = Store.open()) {
                assertTrue(Files.isDirectory(store.configDir()));
                assertTrue(Files.isDirectory(store.storeDir()));
                assertNotNull(store.logsDir());
                assertFalse(store.sqliteConnectionUrl().isBlank());
                assertFalse(store.sqliteAttachCommand().isBlank());
                assertFalse(store.sqliteDetachCommand().isBlank());
                RefreshStatus baseline = store.shouldRefresh(-1);
                assertTrue(baseline.shouldRefresh());

                store.importAsset(assetFile);
                RefreshStatus afterAsset = store.shouldRefresh(baseline.revision());
                assertTrue(afterAsset.shouldRefresh());
                String importedId = store.importBundle(bundle);
                assertTrue(store.shouldRefresh(afterAsset.revision()).shouldRefresh());
                assertEquals(bundle.bundleId(), importedId);
                assertTrue(store.hasAsset(assetId));
                assertTrue(store.hasBundle(importedId));
                assertTrue(store.hasChart("sha256/" + bundle.chartSha256()));
                assertTrue(store.listBundles("bms").contains(importedId));
                try (BackbeatFile opened = store.getBundle(importedId).orElseThrow()) {
                    assertEquals(importedId, opened.bundleId());
                    assertArrayEquals(asset, assetBytes(opened.resolveAsset("AUDIO/TEST.WAV").orElseThrow()));
                }
                try (BackbeatFile opened = store.getChart("sha256/" + bundle.chartSha256()).orElseThrow()) {
                    assertEquals(importedId, opened.bundleId());
                }
                assertArrayEquals(chart, store.getChartData("sha256/" + bundle.chartSha256()).orElseThrow());
                assertArrayEquals(asset, assetBytes(store.getAsset(assetId).orElseThrow()));
                assertArrayEquals(asset, assetBytes(store.resolveAsset(importedId, "audio/test.wav").orElseThrow()));
                store.bundleDownloadAssets(importedId);
                assertTrue(store.stats().charts() >= 1);
                store.bundleRm(importedId);
                assertFalse(store.hasBundle(importedId));
                assertFalse(store.hasChart("sha256/" + bundle.chartSha256()));
                assertTrue(store.hasAsset(assetId));
            }
        }
    }

    @Test
    void emptyCollectionAndDownloadQueriesWork() throws Exception {
        String assetId = sha256("missing asset".getBytes(StandardCharsets.UTF_8));
        String bundleId = "b-" + "0".repeat(64);
        String chartId = "sha256/" + "0".repeat(64);
        String collectionUrl = "https://collections.example/java-sdk-missing";

        try (Store store = Store.open()) {
            assertFalse(store.hasCollection(collectionUrl));
            assertFalse(store.hasCourse(collectionUrl));
            assertFalse(store.hasPack(collectionUrl));
            assertFalse(store.hasTable(collectionUrl));
            assertTrue(store.getCourse(collectionUrl).isEmpty());
            assertTrue(store.getPack(collectionUrl).isEmpty());
            assertTrue(store.getTable(collectionUrl).isEmpty());
            assertTrue(store.listTables("missing-mode").isEmpty());
            assertTrue(store.listCourses("missing-mode").isEmpty());
            assertTrue(store.listPacks("missing-mode").isEmpty());

            DataId dataId = DataId.asset(assetId);
            assertTrue(store.downloadProgress(dataId).isEmpty());
            assertTrue(store.downloadAllProgress().isEmpty());
            assertEquals(0, store.downloadOverview().total());
            assertTrue(store.downloadOverview().firstError().isEmpty());
            assertTrue(store.downloadList(0, 20).downloads().isEmpty());
            assertTrue(store.collectionDownloadList(0, 20).downloads().isEmpty());
            assertThrows(IllegalArgumentException.class, () -> store.downloadList(-1, 20));
            assertThrows(IllegalArgumentException.class, () -> store.downloadList(0, -1));
            assertFalse(store.downloadCancel(dataId));
            assertFalse(store.downloadAssetCancel(assetId));
            assertFalse(store.downloadBundleCancel(bundleId));
            assertFalse(store.downloadChartCancel(chartId));

            store.serverDownloadAsset(assetId);
            long deadline = System.nanoTime() + 5_000_000_000L;
            while (store.downloadProgress(dataId)
                    .map(progress -> progress.state() != DownloadState.FAILED)
                    .orElse(true)) {
                assertTrue(System.nanoTime() < deadline, "download did not finish");
                Thread.sleep(10);
            }

            DownloadSnapshot snapshot = assertDoesNotThrow(() -> store.downloadAllProgress().get(0));
            assertEquals(dataId, snapshot.key());
            assertEquals(DownloadState.FAILED, snapshot.progress().state());
            assertTrue(snapshot.error().isPresent());

            DownloadOverview overview = store.downloadOverview();
            assertEquals(1, overview.total());
            assertEquals(1, overview.failed());
            assertTrue(overview.firstError().isPresent());

            DownloadListResult list = store.downloadList(0, 1);
            assertEquals(1, list.total());
            assertEquals(1, list.failed());
            assertEquals(List.of(snapshot), list.downloads());
            assertFalse(list.hasMore());
            assertEquals(0, store.collectionDownloadList(0, 20).total());
            assertTrue(store.downloadClearFinished() >= 1);
        }
    }

    @Test
    void storeCanBeAttachedThroughSqliteJdbc() throws Exception {
        try (Store store = Store.open();
             var connection = DriverManager.getConnection("jdbc:sqlite::memory:");
             var statement = connection.createStatement()) {
            statement.execute(store.sqliteAttachCommand());
            try (var result = statement.executeQuery("SELECT count(*) FROM backbeat.bundle")) {
                assertTrue(result.next());
                result.getLong(1);
            }
            statement.execute(store.sqliteDetachCommand());
        }
    }

    private static byte[] assetBytes(AssetData asset) throws Exception {
        if (asset instanceof AssetData.Bytes bytes) return bytes.data();
        return Files.readAllBytes(((AssetData.File) asset).path());
    }

    private static byte[] bundleJson(byte[] chart, String assetId) throws Exception {
        ByteArrayOutputStream compressed = new ByteArrayOutputStream();
        try (GZIPOutputStream gzip = new GZIPOutputStream(compressed)) { gzip.write(chart); }
        String json = "{\"filename\":\"chart.bms\",\"assets\":{\"audio/test.wav\":\""
                + assetId + "\"},\"desc\":\"Java SDK test bundle\",\"chart\":\""
                + Base64.getEncoder().encodeToString(compressed.toByteArray()) + "\"}";
        return json.getBytes(StandardCharsets.UTF_8);
    }

    private static String sha256(byte[] value) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(value));
    }
}
