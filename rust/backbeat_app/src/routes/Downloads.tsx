import { createSignal, For, onCleanup, onMount, Show } from "solid-js";

import { PageHeader } from "../components/PageHeader";
import Button from "../components/primitives/Button";
import {
	dismissSettledCollectionJobs,
	refreshDownloadActivity,
	startDownloadActivity,
} from "../lib/downloadActivity";
import "./Downloads.css";
import {
	cancelDownload,
	clearFinishedDownloads,
	type DownloadKey,
	type DownloadListResult,
	type DownloadSnapshot,
	type DownloadState,
	listDownloads,
} from "../lib/tauri";

const PAGE_SIZE = 50;
/** Hard cap on rows kept in the DOM, even if the user keeps loading more. */
const MAX_RENDERED = 500;
const REFRESH_MS = 1000;

const STATE_LABEL: Record<DownloadState, string> = {
	queued: "Queued",
	downloading: "Downloading",
	verifying: "Verifying",
	committing: "Committing",
	done: "Done",
	failed: "Failed",
	cancelled: "Cancelled",
};

const TERMINAL_STATES: DownloadState[] = ["done", "failed", "cancelled"];

function isTerminal(state: DownloadState): boolean {
	return TERMINAL_STATES.includes(state);
}

function formatBytes(bytes: number): string {
	if (bytes === 0) {
		return "0 B";
	}
	const units = ["B", "KB", "MB", "GB"];
	const i = Math.min(units.length - 1, Math.floor(Math.log10(bytes) / 3));
	const value = bytes / Math.pow(1000, i);
	return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function downloadKind(snapshot: DownloadSnapshot): string {
	return snapshot.key.kind;
}

function keyId(key: DownloadKey): string {
	return JSON.stringify(key);
}

function statusClass(state: DownloadState): string {
	if (state === "done") {
		return "download-status-done";
	}
	if (state === "failed" || state === "cancelled") {
		return "download-status-bad";
	}
	return "download-status-active";
}

function progressPercent(progress: DownloadSnapshot["progress"]): number {
	if (progress.items_total !== null && progress.items_total > 0) {
		return Math.min(100, (progress.items_done / progress.items_total) * 100);
	}
	if (progress.total && progress.total > 0) {
		return Math.min(100, (progress.bytes / progress.total) * 100);
	}
	return 0;
}

function progressLabel(progress: DownloadSnapshot["progress"]): null | string {
	if (progress.items_total !== null) {
		const total = progress.items_total;
		const unit = total === 1 ? "asset" : "assets";
		return `${progress.items_done} / ${total} ${unit}`;
	}
	if (progress.bytes > 0 || progress.total !== null) {
		return `${formatBytes(progress.bytes)}${progress.total ? ` / ${formatBytes(progress.total)}` : ""}`;
	}
	return null;
}

export default function Downloads() {
	const [stats, setStats] = createSignal<null | Omit<
		DownloadListResult,
		"descriptions" | "downloads" | "has_more"
	>>(null);
	const [rows, setRows] = createSignal<DownloadSnapshot[]>([]);
	const [descriptions, setDescriptions] = createSignal<Record<string, string>>({});
	const [hasMore, setHasMore] = createSignal(false);
	const [loading, setLoading] = createSignal(true);
	const [loadingMore, setLoadingMore] = createSignal(false);
	const [clearing, setClearing] = createSignal(false);
	const [cancelling, setCancelling] = createSignal<Set<string>>(new Set());
	let refreshTimer: ReturnType<typeof setInterval> | undefined;

	function applyPage(result: DownloadListResult, append: boolean) {
		setStats({
			total: result.total,
			queued: result.queued,
			running: result.running,
			failed: result.failed,
			done: result.done,
			cancelled: result.cancelled,
			first_error: result.first_error,
		});
		setDescriptions(result.descriptions);
		setRows((prev) => {
			const merged = append ? [...prev, ...result.downloads] : result.downloads;
			const capped = merged.slice(0, MAX_RENDERED);
			setHasMore(result.has_more && capped.length < MAX_RENDERED);
			return capped;
		});
	}

	async function refreshFirstPage() {
		try {
			const want = Math.min(MAX_RENDERED, Math.max(PAGE_SIZE, rows().length || PAGE_SIZE));
			const result = await listDownloads(0, want);
			applyPage(result, false);
		} finally {
			setLoading(false);
		}
	}

	async function loadMore() {
		setLoadingMore(true);
		try {
			const result = await listDownloads(rows().length, PAGE_SIZE);
			applyPage(result, true);
		} finally {
			setLoadingMore(false);
		}
	}

	onMount(() => {
		startDownloadActivity();
		void refreshDownloadActivity();
		void refreshFirstPage();
		refreshTimer = setInterval(() => {
			void refreshFirstPage();
		}, REFRESH_MS);
	});

	onCleanup(() => {
		if (refreshTimer) {
			clearInterval(refreshTimer);
		}
	});

	async function handleClear() {
		setClearing(true);
		try {
			await clearFinishedDownloads();
			dismissSettledCollectionJobs();
			await refreshDownloadActivity();
			await refreshFirstPage();
		} finally {
			setClearing(false);
		}
	}

	async function handleCancel(snapshot: DownloadSnapshot) {
		const id = keyId(snapshot.key);
		setCancelling((prev) => {
			const next = new Set(prev);
			next.add(id);
			return next;
		});
		try {
			await cancelDownload(snapshot.key);
		} finally {
			setCancelling((prev) => {
				const next = new Set(prev);
				next.delete(id);
				return next;
			});
			await refreshDownloadActivity();
			await refreshFirstPage();
		}
	}

	const finished = () =>
		(stats()?.done ?? 0) + (stats()?.cancelled ?? 0) + (stats()?.failed ?? 0);

	return (
		<>
			<PageHeader
				actions={
					<Button
						disabled={clearing() || finished() === 0}
						onClick={handleClear}
						variant="base"
					>
						{clearing() ? "Clearing…" : "Clear completed"}
					</Button>
				}
				subtitle="What you're downloading at the moment."
				title="Downloads"
			/>

			<Show fallback={<div class="download-loading">Loading…</div>} when={!loading()}>
				<div class="download-summary">
					<div class="download-stat">
						<span class="download-stat-value">{stats()?.total ?? 0}</span>
						<span class="download-stat-label">Total</span>
					</div>
					<div class="download-stat">
						<span class="download-stat-value">{stats()?.queued ?? 0}</span>
						<span class="download-stat-label">Queued</span>
					</div>
					<div class="download-stat">
						<span class="download-stat-value">{stats()?.running ?? 0}</span>
						<span class="download-stat-label">Active</span>
					</div>
					<div
						class={`download-stat ${(stats()?.failed ?? 0) > 0 ? "download-stat-failed" : ""}`}
					>
						<span class="download-stat-value">{stats()?.failed ?? 0}</span>
						<span class="download-stat-label">Failed</span>
					</div>
					<div class="download-stat">
						<span class="download-stat-value">{finished()}</span>
						<span class="download-stat-label">Finished</span>
					</div>
				</div>

				<Show
					fallback={<div class="empty-state">No downloads in the queue.</div>}
					when={(stats()?.total ?? 0) > 0}
				>
					<div class="download-list">
						<For each={rows()}>
							{(snapshot) => {
								const kind = downloadKind(snapshot);
								const target = () => descriptions()[snapshot.key.value] ?? snapshot.key.value;
								const progress = snapshot.progress;
								const percent = progressPercent(progress);
								const label = progressLabel(progress);
								const id = keyId(snapshot.key);
								const isCancelling = () => cancelling().has(id);

								return (
									<div
										class={`download-row ${progress.state === "failed" ? "download-row-failed" : ""}`}
									>
										<div class="download-row-main">
											<div class="download-row-info">
												<span
													class={`download-kind download-kind-${kind.toLowerCase()}`}
												>
													{kind}
												</span>
												<span
													class={`download-target ${descriptions()[snapshot.key.value] === undefined ? "" : "download-target-description"}`}
													title={target()}
												>
													{target()}
												</span>
												<span
													class={`download-status ${statusClass(progress.state)}`}
												>
													{STATE_LABEL[progress.state]}
												</span>
											</div>
											<Show when={!isTerminal(progress.state)}>
												<Button
													disabled={isCancelling()}
													onClick={() => handleCancel(snapshot)}
													type="button"
													variant={isCancelling() ? "base" : "danger"}
												>
													{isCancelling() ? "Cancelling…" : "Cancel"}
												</Button>
											</Show>
										</div>
										<Show when={label !== null || !isTerminal(progress.state)}>
											<div class="download-progress-row">
												<div class="download-progress-track">
													<div
														class="download-progress-bar"
														style={{ width: `${percent}%` }}
													/>
												</div>
												<span class="download-bytes">
													{label ?? "Fetching manifest…"}
												</span>
											</div>
										</Show>
										<Show when={progress.state === "failed" ? snapshot.error : undefined}>
											{(message) => (
												<p class="download-row-error">{message()}</p>
											)}
										</Show>
									</div>
								);
							}}
						</For>
					</div>

					<Show when={hasMore()}>
						<div class="search-list-footer">
							<Button disabled={loadingMore()} onClick={loadMore} variant="base">
								{loadingMore() ? "Loading…" : "Load more"}
							</Button>
						</div>
					</Show>
				</Show>
			</Show>
		</>
	);
}
