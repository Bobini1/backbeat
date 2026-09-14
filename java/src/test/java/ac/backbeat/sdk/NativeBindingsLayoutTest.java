package ac.backbeat.sdk;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;

import java.lang.reflect.Method;
import java.util.Set;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;

@EnabledIfSystemProperty(named = "backbeat.integration", matches = "true")
class NativeBindingsLayoutTest {
    @Test
    void sixtyFourBitStructureLayoutsMatchTheCHeader() {
        assertEquals(16, new NativeBindings.BkbStr().size());
        assertEquals(64, new NativeBindings.BkbBb().size());
        assertEquals(48, new NativeBindings.BkbBundleSearchChart().size());
        assertEquals(32, new NativeBindings.BkbBundleSearchResult().size());
        assertEquals(24, new NativeBindings.BkbAssetData().size());
        assertEquals(16, new NativeBindings.BkbTimestamp().size());
        assertEquals(80, new NativeBindings.BkbCollectionMetadata().size());
        assertEquals(24, new NativeBindings.BkbCollectionHeader().size());
        assertEquals(8, new NativeBindings.BkbCollectionUpsertResult().size());
        assertEquals(64, new NativeBindings.BkbWrongChartId().size());
        assertEquals(48, new NativeBindings.BkbUncomputedChartId().size());
        assertEquals(48, new NativeBindings.BkbDanglingAssetRef().size());
        assertEquals(136, new NativeBindings.BkbCorruptionReport().size());
        assertEquals(32, new NativeBindings.BkbTag().size());
        assertEquals(32, new NativeBindings.BkbAsset().size());
        assertEquals(64, new NativeBindings.BkbCourseChart().size());
        assertEquals(96, new NativeBindings.BkbCourse().size());
        assertEquals(56, new NativeBindings.BkbPackBundle().size());
        assertEquals(96, new NativeBindings.BkbPack().size());
        assertEquals(64, new NativeBindings.BkbTableChart().size());
        assertEquals(48, new NativeBindings.BkbTableLevel().size());
        assertEquals(64, new NativeBindings.BkbTableFolder().size());
        assertEquals(128, new NativeBindings.BkbTable().size());
		assertEquals(56, new NativeBindings.BkbStats().size());
        assertEquals(16, new NativeBindings.BkbDataId(1, "id").size());
        assertEquals(16, new NativeBindings.BkbOptionalU64().size());
        assertEquals(56, new NativeBindings.BkbDownloadProgress().size());
        assertEquals(64, new NativeBindings.BkbOptionalDownloadProgress().size());
        assertEquals(24, new NativeBindings.BkbOwnedDataId().size());
        assertEquals(96, new NativeBindings.BkbDownloadSnapshot().size());
        assertEquals(16, new NativeBindings.BkbDownloadSnapshotList().size());
        assertEquals(64, new NativeBindings.BkbDownloadOverview().size());
        assertEquals(88, new NativeBindings.BkbDownloadListResult().size());
        assertEquals(40, new NativeBindings.BkbCollectionDownloadDataFailure().size());
        assertEquals(88, new NativeBindings.BkbCollectionDownloadDataReport().size());
    }

    @Test
    void everyExportedFunctionInTheHeaderHasAJnaBinding() {
        Set<String> actual = java.util.Arrays.stream(NativeBindings.class.getDeclaredMethods())
                .map(Method::getName).filter(name -> name.startsWith("bkb_")).collect(Collectors.toSet());
        Set<String> expected = Set.of(
                "bkb_asset_data_free", "bkb_bb_bundle_id", "bkb_bb_chart_sha256", "bkb_bb_combined_assets_id",
                "bkb_bb_free", "bkb_bb_from_file", "bkb_bb_from_json", "bkb_bb_resolve_path", "bkb_bb_to_json",
                "bkb_bundle_search_result_free", "bkb_bytes_free", "bkb_collection_download_data_report_free",
                "bkb_corruption_report_free",
                "bkb_collection_metadata_list_free", "bkb_course_free", "bkb_error_string", "bkb_libcommithash",
                "bkb_libversion", "bkb_sqlite_version_number", "bkb_pack_free", "bkb_download_snapshot_list_free",
                "bkb_download_overview_free", "bkb_download_list_result_free",
                "bkb_store_asset_prune", "bkb_store_bundle_download_assets", "bkb_store_bundle_rm",
                "bkb_store_collection_fetch_download_data", "bkb_store_collection_fetch_header",
                "bkb_store_collection_fetch_upsert", "bkb_store_collection_rm", "bkb_store_config_dir",
                "bkb_store_corruption_check", "bkb_store_corruption_repair",
                "bkb_store_dir", "bkb_store_download_asset_cancel", "bkb_store_download_bundle_cancel",
                "bkb_store_download_cancel", "bkb_store_download_chart_cancel", "bkb_store_download_clear_finished",
                "bkb_store_download_progress", "bkb_store_download_all_progress", "bkb_store_download_overview",
                "bkb_store_download_list", "bkb_store_collection_download_list", "bkb_store_free", "bkb_store_get_asset",
                "bkb_store_get_bundle", "bkb_store_get_chart", "bkb_store_get_chart_data", "bkb_store_get_course",
                "bkb_store_get_pack", "bkb_store_get_table", "bkb_store_has_asset", "bkb_store_has_bundle",
                "bkb_store_has_chart", "bkb_store_has_collection", "bkb_store_has_course", "bkb_store_has_pack",
                "bkb_store_has_table", "bkb_store_has_zero_data_servers", "bkb_store_import_asset",
                "bkb_store_import_bbzip", "bkb_store_import_bundle", "bkb_store_disk_prune",
                "bkb_store_export_bundle", "bkb_store_export_chart", "bkb_store_search_bundles",
                "bkb_store_list_courses", "bkb_store_list_packs", "bkb_store_list_tables", "bkb_store_logs_dir",
                "bkb_store_open", "bkb_store_server_add", "bkb_store_server_download_asset",
                "bkb_store_server_download_bundle", "bkb_store_server_download_chart", "bkb_store_server_rm",
                "bkb_store_resolve_path", "bkb_store_sqlite_attach_command", "bkb_store_sqlite_connection_url",
                "bkb_store_should_refresh", "bkb_store_sqlite_detach_command", "bkb_store_stats",
                "bkb_string_free",
                "bkb_table_chart_count", "bkb_table_free");
        assertEquals(expected, actual);
    }
}
