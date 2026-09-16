import { useNavigate } from "@solidjs/router";
import { createResource, createSignal, type JSX, onCleanup, onMount, Show } from "solid-js";

import {
	type CollectionKind,
	collectionKindLabel,
	collectionSettingsPath,
	decodeCollectionUrl,
	describeCollectionError,
	pathForCollectionKind,
} from "../../lib/collectionUi";
import { timeAgo } from "../../lib/format";
import {
	checkInstalledCollectionUpdates,
	type CollectionDocumentStatus,
	installCollectionDocument,
	onCollectionDownloadEvent,
} from "../../lib/tauri";
import { BackLink } from "../BackLink";
import { PageHeader } from "../PageHeader";
import Banner from "../primitives/Banner";
import Button from "../primitives/Button";
import ExternalLink from "../primitives/ExternalLink";
import styles from "./CollectionCoverageDetail.module.css";
import CoverageCount from "./CoverageCount";
import StatusBadge from "./StatusBadge";

const DOWNLOAD_REFETCH_DEBOUNCE_MS = 500;

export type CollectionDownloadProgress = {
	current: number;
	error: null | string;
	item: null | string;
	state: "done" | "downloading" | "failed";
	total: number;
	url: string;
};

export type CollectionContentsSummary = {
	installed: number;
	total: number;
};

export function CollectionCoverageDetail<T extends { name: string }>(props: {
	children: (data: T) => JSX.Element;
	kind: Extract<CollectionKind, "course" | "pack" | "table">;
	loadContents: (url: string) => Promise<T>;
	startDownload: (url: string, name: string) => Promise<void>;
	subtitle: string;
	summarize: (data: T) => CollectionContentsSummary;
	unit: string;
	urlParam: string;
}) {
	const navigate = useNavigate();
	const url = () => decodeCollectionUrl(props.urlParam);
	const kindLower = () => collectionKindLabel(props.kind).toLowerCase();

	const [contents, { refetch }] = createResource(url, async (collectionUrl) => {
		try {
			return await props.loadContents(collectionUrl);
		} catch (error) {
			if (/not found:/i.test(String(error))) {
				navigate(pathForCollectionKind(props.kind) ?? "/", { replace: true });
				return;
			}
			throw error;
		}
	});
	const [status, { refetch: refetchStatus }] = createResource(url, async (collectionUrl) => {
		const statuses = await checkInstalledCollectionUpdates(props.kind);
		return statuses.find((entry) => entry.url === collectionUrl) as
			| CollectionDocumentStatus
			| undefined;
	});
	const [download, setDownload] = createSignal<CollectionDownloadProgress | null>(null);
	const [updating, setUpdating] = createSignal(false);
	const [updateError, setUpdateError] = createSignal<null | string>(null);
	let unlisten: (() => void) | undefined;
	let refetchTimer: ReturnType<typeof setTimeout> | undefined;
	let disposed = false;

	function refetchContentsAfterDownloadEvent(state: CollectionDownloadProgress["state"]) {
		if (refetchTimer) {
			clearTimeout(refetchTimer);
			refetchTimer = undefined;
		}

		if (state === "downloading") {
			refetchTimer = setTimeout(() => {
				refetchTimer = undefined;
				refetch();
			}, DOWNLOAD_REFETCH_DEBOUNCE_MS);
			return;
		}

		refetch();
	}

	onMount(() => {
		void onCollectionDownloadEvent((event) => {
			if (event.url !== url()) {
				return;
			}
			setDownload({ ...event, item: event.description ?? event.item?.value ?? null });
			refetchContentsAfterDownloadEvent(event.state);
		}).then((listener) => {
			if (disposed) {
				listener();
			} else {
				unlisten = listener;
			}
		});
	});

	onCleanup(() => {
		disposed = true;
		if (refetchTimer) {
			clearTimeout(refetchTimer);
		}
		unlisten?.();
	});

	function refreshAll() {
		refetch();
		refetchStatus();
	}

	async function handleDownload() {
		if (downloading()) {
			return;
		}
		const current = contents.latest;
		if (!current) {
			return;
		}
		const summary = props.summarize(current);
		setDownload({
			url: url(),
			state: "downloading",
			current: 0,
			total: summary.total,
			item: "Preparing download…",
			error: null,
		});
		try {
			await props.startDownload(url(), current.name);
		} catch (error) {
			setDownload({
				url: url(),
				state: "failed",
				current: 0,
				total: 0,
				item: null,
				error: String(error),
			});
		}
	}

	async function handleUpdate() {
		setUpdating(true);
		setUpdateError(null);
		try {
			await installCollectionDocument(url());
			refreshAll();
		} catch (error) {
			setUpdateError(describeCollectionError(error, props.kind));
		} finally {
			setUpdating(false);
		}
	}

	const missing = (data: T) => {
		const summary = props.summarize(data);
		return summary.total - summary.installed;
	};
	const currentContents = () => contents.latest;
	const downloading = () => download()?.state === "downloading";

	return (
		<>
			<p class="breadcrumb">
				<BackLink />
			</p>

			<PageHeader
				actions={
					<Show when={currentContents()}>
						<div class="installed-collection-actions">
							<Show when={status()?.needs_update}>
								<Button
									disabled={updating()}
									onClick={handleUpdate}
									variant="accent"
								>
									{updating() ? "Updating…" : "Update"}
								</Button>
							</Show>
							<Button
								as="a"
								href={collectionSettingsPath(props.kind, url())}
								variant="danger"
							>
								Delete
							</Button>
						</div>
					</Show>
				}
				subtitle={props.subtitle}
				title={`${collectionKindLabel(props.kind)}: ${currentContents()?.name ?? collectionKindLabel(props.kind)}`}
			/>

			<Show when={updateError()}>
				<Banner variant="danger">{updateError()}</Banner>
			</Show>

			<Show when={contents.loading && currentContents() === undefined}>
				<div>Loading {kindLower()}…</div>
			</Show>

			<Show when={contents.error}>
				<Banner variant="danger">
					Failed to load {kindLower()}: {String(contents.error)}
				</Banner>
			</Show>

			<Show when={currentContents()}>
				{(data) => (
					<>
						<Show when={status()?.error}>
							<Banner variant="danger">{status()!.error}</Banner>
						</Show>

						<ExternalLink href={url()}>
							View {collectionKindLabel(props.kind).toLowerCase()} in web browser
						</ExternalLink>

						<Show when={download()?.error || missing(data()) > 0}>
							<Banner
								button={{
									disabled: downloading(),
									label: downloading()
										? "Downloading…"
										: "Download missing content",
									onClick: handleDownload,
								}}
								progress={
									downloading() && {
										current: download()?.current ?? 0,
										total: download()?.total ?? 0,
									}
								}
								variant={downloading() ? "base" : "danger"}
							>
								{download()?.error ? (
									download()?.error
								) : downloading() ? (
									<div class={styles.downloadStatus}>
										<div class={styles.downloadCount}>
											{download()?.current} / {download()?.total}
										</div>
										<div>{download()?.item}</div>
									</div>
								) : (
									<CoverageCount
										installed={props.summarize(data()).installed}
										total={props.summarize(data()).total}
										unit={props.unit}
									/>
								)}
							</Banner>
						</Show>

						<Show when={props.kind === "table"}>
							<Banner variant="base">
								<StatusBadge status={status()} />
								<Show when={status()?.updated}>
									{(updated) => <span>Updated {timeAgo(updated())}</span>}
								</Show>
							</Banner>
						</Show>

						{props.children(data())}
					</>
				)}
			</Show>
		</>
	);
}
