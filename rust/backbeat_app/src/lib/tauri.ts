import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { CollectionKind } from "./collectionUi";

/** Build metadata embedded at compile time via vergen. */
export interface BuildInfo {
	git_sha_short: null | string;
	version: string;
}

export function buildInfo(): Promise<BuildInfo> {
	return invoke<BuildInfo>("build_info");
}

export interface StoreStats {
	asset_bytes: number;
	asset_count: number;
	charts: number;
	courses: number;
	db_bytes: number;
	packs: number;
	tables: number;
}

export function storeSummary(): Promise<StoreStats> {
	return invoke<StoreStats>("store_summary");
}

/** One chart in a search result page. */
export interface ChartEntry {
	bundle_id: string;
	description: string;
	extension: null | string;
}

export interface ChartSearchResponse {
	charts: ChartEntry[];
	has_more: boolean;
	total: number;
}

export function searchCharts(
	query: string | undefined,
	offset: number,
	limit?: number,
	extension?: null | string,
): Promise<ChartSearchResponse> {
	return invoke<ChartSearchResponse>("search_charts", {
		query: query ?? null,
		offset,
		limit: limit ?? null,
		extension: extension || null,
	});
}

export function listAvailableExtensions(): Promise<string[]> {
	return invoke<string[]>("list_available_extensions");
}

/** One asset a chart bundle depends on. */
export interface BundleAsset {
	asset_id: string;
	path: string;
	/** `undefined`/`null` if this asset was never actually downloaded. */
	size: null | number;
}

export type BundleCollectionKind = "course" | "pack" | "table";

/** An installed collection that references a chart bundle. */
export interface BundleCollectionAppearance {
	collection_kind: BundleCollectionKind;
	level: null | string;
	name: string;
	symbol: null | string;
	url: string;
}

export type ChartId = string;

/** Full detail for a single bundle, shown on the chart detail screen. */
export interface BundleDetailResponse {
	appearances: BundleCollectionAppearance[];
	assets: BundleAsset[];
	bundle_id: string;
	/** Alternate chart identifiers in canonical `algorithm/value` form. */
	chart_ids: ChartId[];
	chart_sha256: string;
	description: string;
	filename: string;
	uncompressed_size: number;
}

export function getBundleDetail(bundleId: string): Promise<BundleDetailResponse> {
	return invoke<BundleDetailResponse>("get_bundle_detail", { bundleId });
}

export function removeBundle(bundleId: string): Promise<void> {
	return invoke<void>("remove_bundle", { bundleId });
}

export function importFiles(paths: string[]): Promise<number> {
	return invoke<number>("import_files", { paths });
}

export function exportBundle(bundleId: string, bbzip: boolean): Promise<void> {
	return invoke<void>("export_bundle", { bundleId, bbzip });
}

/** A chart bundle that depends on an asset, from a reverse lookup. */
export interface AssetDependent {
	bundle_id: string;
	description: string;
	/** Path the asset is referenced at within this bundle. */
	path: string;
}

/** Full detail for a single asset, shown on the asset detail screen. */
export interface AssetDetailResponse {
	dependents: AssetDependent[];
	/** Whether this asset lives as a file on disk (vs. inline in SQLite, or not stored at all). */
	has_file: boolean;
	id: string;
	/** `null` if this asset was never actually downloaded. */
	size: null | number;
}

export function getAssetDetail(assetId: string): Promise<AssetDetailResponse> {
	return invoke<AssetDetailResponse>("get_asset_detail", { assetId });
}

/** Preview for image/audio/text assets, with MIME guessed from dependent filenames. */
export interface AssetPreviewResponse {
	/** Base64 bytes for inline media and plain-text previews. */
	data_base64: null | string;
	/** The supported preview type, or `null` when this asset cannot be displayed. */
	kind: "audio" | "image" | "text" | null;
	mime_type: string;
	/** Absolute path when the asset is on disk (use with `convertFileSrc`). */
	path: null | string;
}

export function getAssetPreview(assetId: string): Promise<AssetPreviewResponse | null> {
	return invoke<AssetPreviewResponse | null>("get_asset_preview", { assetId });
}

/** Reveal the asset's containing folder in the OS's default file manager. */
export function openAssetFolder(assetId: string): Promise<void> {
	return invoke<void>("open_asset_folder", { assetId });
}

/** A configured `[[server]]` entry (a remote Backbeat Data Server). */
export interface ServerConfig {
	url: string;
}

export function listRemotes(): Promise<ServerConfig[]> {
	return invoke<ServerConfig[]>("list_remotes");
}

export function addRemote(url: string): Promise<void> {
	return invoke<void>("add_remote", { url });
}

export function removeRemote(url: string): Promise<void> {
	return invoke<void>("remove_remote", { url });
}

/** Result of a liveness probe against a remote's `/backbeat/info` endpoint. */
export interface PingResult {
	contact: null | string;
	error: null | string;
	latency_ms: number;
	name: null | string;
	up: boolean;
}

export function pingRemote(url: string): Promise<PingResult> {
	return invoke<PingResult>("ping_remote", { url });
}

export interface CollectionDocumentStatus {
	error: null | string;
	kind: CollectionKind;
	needs_update: boolean;
	updated: null | string;
	url: string;
}

export function checkInstalledCollectionUpdates(
	kind: CollectionKind,
): Promise<CollectionDocumentStatus[]> {
	return invoke<CollectionDocumentStatus[]>("check_installed_collection_updates", { kind });
}

export interface CollectionMetadata {
	gamemode: string;
	installed: number;
	name: string;
	total: number;
	updated: string;
	url: string;
}

export function listInstalledCollectionDocuments(
	kind: CollectionKind,
): Promise<CollectionMetadata[]> {
	return invoke<CollectionMetadata[]>("list_installed_collection_documents", { kind });
}

export interface TableContentsChart {
	bundle_id: null | string;
	desc: string;
	id: string;
	tags: Record<string, string>;
}

export interface TableContentsLevel {
	charts: TableContentsChart[];
	level: string;
	tags: Record<string, string>;
}

export interface TableContentsFolder {
	charts: TableContentsChart[];
	name: string;
	query: string;
	tags: Record<string, string>;
}

export interface TableContents {
	assets: Record<string, string>;
	folders: TableContentsFolder[];
	gamemode: string;
	levels: TableContentsLevel[];
	name: string;
	symbol: string;
	tags: Record<string, string>;
	updated: string;
}

export interface CollectionDownloadEvent {
	current: number;
	description: null | string;
	error: null | string;
	item: DownloadKey | null;
	kind: CollectionKind;
	name: string;
	state: "done" | "downloading" | "failed";
	total: number;
	url: string;
}

export function getTable(url: string): Promise<TableContents> {
	return invoke<TableContents>("get_table", { url });
}

export function downloadCollectionContent(
	kind: CollectionKind,
	url: string,
	name: string,
): Promise<void> {
	return invoke<void>("download_collection_content", { kind, url, name });
}

export function onCollectionDownloadEvent(
	handler: (event: CollectionDownloadEvent) => void,
): Promise<UnlistenFn> {
	return listen<CollectionDownloadEvent>("collection-download", (event) =>
		handler(event.payload),
	);
}

export interface PackContentsBundle {
	desc: string;
	id: string;
	installed: boolean;
	tags: Record<string, string>;
}

export interface PackContents {
	assets: Record<string, string>;
	bundles: PackContentsBundle[];
	gamemode: string;
	name: string;
	tags: Record<string, string>;
	updated: string;
}

export function getPack(url: string): Promise<PackContents> {
	return invoke<PackContents>("get_pack", { url });
}

export interface CourseContentsChart {
	bundle_id: null | string;
	desc: string;
	id: string;
	tags: Record<string, string>;
}

export interface CourseContents {
	assets: Record<string, string>;
	charts: CourseContentsChart[];
	gamemode: string;
	name: string;
	tags: Record<string, string>;
	updated: string;
}

export function getCourse(url: string): Promise<CourseContents> {
	return invoke<CourseContents>("get_course", { url });
}

export function installCollectionDocument(url: string): Promise<CollectionKind> {
	return invoke<CollectionKind>("install_collection_document", { url });
}

export function uninstallCollectionDocument(url: string, removeChartsToo = false): Promise<void> {
	return invoke<void>("remove_collection", {
		url,
		removeChartsToo,
	});
}

/** Current global settings from `backbeat.toml`. */
export interface SettingsSnapshot {
	config_file: string;
	download_concurrency: number;
	download_stream: string;
	log_dir: string;
	store_inline: string;
}

export function getSettings(): Promise<SettingsSnapshot> {
	return invoke<SettingsSnapshot>("get_settings");
}

export interface SaveSettingsInput {
	download_concurrency: number;
	download_stream: string;
	store_inline: string;
}

export function saveSettings(input: SaveSettingsInput): Promise<SettingsSnapshot> {
	return invoke<SettingsSnapshot>("save_settings", {
		storeInline: input.store_inline,
		downloadConcurrency: input.download_concurrency,
		downloadStream: input.download_stream,
	});
}

export function openConfigFolder(): Promise<void> {
	return invoke<void>("open_config_folder");
}

/** Open the rotating-log folder in the OS file manager. */
export function openLogsFolder(): Promise<void> {
	return invoke<void>("open_logs_folder");
}

export interface CorruptionCheckResponse {
	chart_count: number;
	chart_id_count: number;
	corrupt_assets: number;
	corrupt_charts: number;
	dangling_asset_refs: number;
	is_ok: boolean;
	issue_count: number;
	large_asset_count: number;
	missing_assets: number;
	uncomputable_chart_ids: number;
	wrong_chart_ids: number;
}

export function assetPrune(): Promise<number> {
	return invoke<number>("asset_prune");
}

export function diskPrune(): Promise<number> {
	return invoke<number>("disk_prune");
}

export function corruptionCheck(): Promise<CorruptionCheckResponse> {
	return invoke<CorruptionCheckResponse>("corruption_check");
}

export function corruptionRepair(): Promise<void> {
	return invoke<void>("corruption_repair");
}

/** Open an http(s) URL in the system default browser. */
export function openUrl(url: string): Promise<void> {
	return invoke<void>("open_url", { url });
}

/**
 * Forward a frontend log line into the Rust `tracing` log file. `level` is
 * `"error"`, `"warn"`, or `"info"`; `location` is an optional `file:line` hint.
 */
export function frontendLog(
	level: "error" | "info" | "warn",
	message: string,
	location?: string,
): Promise<void> {
	return invoke<void>("frontend_log", { level, message, location: location ?? null });
}

// ── Downloads / deep links ───────────────────────────────────────────────────

/**
 * Install a chart from the configured servers by its canonical
 * `algorithm/value` chart ID.
 */
export function downloadChart(chartId: ChartId): Promise<void> {
	return invoke<void>("download_chart", { chartId });
}

/**
 * Install a `.bb` bundle from the configured `[[server]]`s by bundle id
 * (`b-<sha256>`). Mirrors the `backbeat://bundle/<id>` deep link.
 */
export function downloadBundle(bundleId: string): Promise<void> {
	return invoke<void>("download_bundle", { bundleId });
}

// ── Download manager state ───────────────────────────────────────────────────

/** Coarse state of a single download, as reported by the manager. */
export type DownloadState =
	| "cancelled"
	| "committing"
	| "done"
	| "downloading"
	| "failed"
	| "queued"
	| "verifying";

/** Per-download progress: bytes for assets, item counts for chart/bundle jobs. */
export interface DownloadProgress {
	bytes: number;
	/** Child assets finished (chart/bundle jobs). Zero for assets. */
	items_done: number;
	/** Total child assets when this is a chart/bundle job; otherwise `null`. */
	items_total: null | number;
	state: DownloadState;
	total: null | number;
}

/** Manager key for a single download, mirrored from the Rust `DataId`. */
export type DownloadKey =
	| { kind: "Asset"; value: string }
	| { kind: "Bundle"; value: string }
	| { kind: "Chart"; value: ChartId };

/** One entry in the download manager snapshot. */
export interface DownloadSnapshot {
	/** Failure reason when `progress.state` is `"failed"`; otherwise `null`. */
	error: null | string;
	key: DownloadKey;
	progress: DownloadProgress;
}

/** Aggregate download counts for ambient UI (no row payload). */
export interface DownloadOverview {
	cancelled: number;
	done: number;
	failed: number;
	first_error: null | string;
	queued: number;
	running: number;
	total: number;
}

/** One page of download manager rows. */
export interface DownloadListResult {
	cancelled: number;
	/** Descriptions supplied by installed collections, keyed by chart or bundle ID. */
	descriptions: Record<string, string>;
	done: number;
	downloads: DownloadSnapshot[];
	failed: number;
	first_error: null | string;
	has_more: boolean;
	queued: number;
	running: number;
	total: number;
}

/** Fetch aggregate download counts without shipping every row. */
export function downloadOverview(): Promise<DownloadOverview> {
	return invoke<DownloadOverview>("download_overview");
}

/** Fetch one page of downloads. */
export function listDownloads(offset: number, limit?: number): Promise<DownloadListResult> {
	return invoke<DownloadListResult>("list_downloads", {
		offset,
		limit: limit ?? null,
	});
}

/** Cancel a single in-flight download. Returns `true` if it was cancelled. */
export function cancelDownload(key: DownloadKey): Promise<boolean> {
	return invoke<boolean>("cancel_download", { data: key });
}

/** Remove all finished downloads from the manager. Returns the count removed. */
export function clearFinishedDownloads(): Promise<number> {
	return invoke<number>("clear_finished_downloads");
}

/**
 * One update emitted on the `download` event by the deep-link handler.
 * `kind` is `"chart"`, `"bundle"`, `"collection"`, or `"unknown"` (for a malformed link);
 * `status` is `"started"`, `"ok"`, or `"failed"`. `label` is the collection or installed
 * bundle description when known; `target` remains the stable chart or bundle ID.
 */
export interface DownloadEvent {
	error?: string;
	kind: "bundle" | "chart" | "collection" | "unknown";
	label?: string;
	status: "failed" | "ok" | "started";
	target: string;
}

/**
 * Subscribe to `download` events (deep-link installs). Returns an unlisten
 * function. The handler is called for each `started` / `ok` / `failed`
 * transition, keyed by `kind` + `target` so the UI can match them up.
 */
export function onDownloadEvent(handler: (event: DownloadEvent) => void): Promise<UnlistenFn> {
	return listen<DownloadEvent>("download", (e) => handler(e.payload));
}

/** Download updates produced before the frontend registered its event listener. */
export function takePendingDownloadEvents(): Promise<DownloadEvent[]> {
	return invoke<DownloadEvent[]>("take_pending_events");
}
