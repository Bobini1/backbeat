import { createSignal, For, onCleanup, onMount, Show } from "solid-js";

import {
	downloadBundle,
	downloadChart,
	type DownloadEvent,
	onDownloadEvent,
	takePendingDownloadEvents,
} from "../lib/tauri";
import "./DownloadToast.css";

type Entry = { addedAt: number; id: string; retrying?: boolean } & DownloadEvent;

const STATUS_LABEL: Record<DownloadEvent["status"], string> = {
	started: "Downloading…",
	ok: "Installed",
	failed: "Failed",
};

const KIND_LABEL: Record<string, string> = {
	chart: "Chart",
	bundle: "Bundle",
	collection: "Collection",
	unknown: "Link",
};

function entryId(event: DownloadEvent): string {
	return `${event.kind}:${event.target}`;
}

/** Re-trigger a deep-link install by kind. `unknown` links are malformed and cannot be retried. */
function retryInstall(entry: Entry): null | Promise<void> {
	if (entry.kind === "chart") {
		return downloadChart(entry.target);
	}
	if (entry.kind === "bundle") {
		return downloadBundle(entry.target);
	}
	return null;
}

/** A small toast stack in the bottom-right that reflects deep-link installs. */
export default function DownloadToast() {
	const [entries, setEntries] = createSignal<Entry[]>([]);
	let unlisten: (() => void) | undefined;

	function addEvent(event: DownloadEvent) {
		const id = entryId(event);
		setEntries((prev) => {
			const next = prev.filter((e) => e.id !== id);
			next.push({ ...event, id, addedAt: Date.now() });
			return next;
		});

		if (event.status === "ok") {
			setTimeout(() => {
				setEntries((prev) => prev.filter((e) => e.id !== id));
			}, 3500);
		}
	}

	onMount(() => {
		onDownloadEvent(addEvent).then(async (un) => {
			unlisten = un;
			for (const event of await takePendingDownloadEvents()) {
				addEvent(event);
			}
		});
	});

	onCleanup(() => {
		unlisten?.();
	});

	function dismiss(id: string) {
		setEntries((prev) => prev.filter((e) => e.id !== id));
	}

	function handleRetry(entry: Entry) {
		const run = retryInstall(entry);
		if (!run) {
			return;
		}
		setEntries((prev) => prev.map((e) => (e.id === entry.id ? { ...e, retrying: true } : e)));
		run.catch(() => {
			// The backend re-emits a `failed` event on retry; nothing to do here.
		}).finally(() => {
			setEntries((prev) =>
				prev.map((e) => (e.id === entry.id ? { ...e, retrying: false } : e)),
			);
		});
	}

	return (
		<div aria-live="polite" class="download-toast-stack">
			<For each={entries()}>
				{(entry) => (
					<div class={`download-toast download-toast-${entry.status}`} role="status">
						<div class="download-toast-head">
							<Show
								fallback={
									<span
										aria-hidden="true"
										class={`download-toast-dot download-toast-dot-${entry.status}`}
									/>
								}
								when={entry.status === "started"}
							>
								<span aria-hidden="true" class="download-toast-spinner" />
							</Show>
							<span class="download-toast-title">
								{KIND_LABEL[entry.kind] ?? entry.kind}
							</span>
							<Show when={entry.status === "failed"}>
								<button
									aria-label="Dismiss"
									class="download-toast-dismiss"
									onClick={() => dismiss(entry.id)}
									title="Dismiss"
								>
									×
								</button>
							</Show>
						</div>
						<div class="download-toast-target">{entry.label ?? entry.target}</div>
						<div class="download-toast-status">{STATUS_LABEL[entry.status]}</div>
						<Show when={entry.error}>
							<div class="download-toast-error">{entry.error}</div>
						</Show>
						<Show
							when={
								entry.status === "failed" &&
								(entry.kind === "chart" || entry.kind === "bundle")
							}
						>
							<button
								class="download-toast-retry"
								disabled={entry.retrying}
								onClick={() => handleRetry(entry)}
							>
								{entry.retrying ? "Retrying…" : "Retry"}
							</button>
						</Show>
					</div>
				)}
			</For>
		</div>
	);
}
