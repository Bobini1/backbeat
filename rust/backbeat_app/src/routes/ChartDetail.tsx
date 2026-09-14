import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, on, onCleanup, onMount, Show } from "solid-js";

import { BackLink } from "../components/BackLink";
import ChartAppearance from "../components/ChartAppearance";
import { ConfirmActions } from "../components/ConfirmActions";
import { CopyableId } from "../components/CopyableId";
import DetailLister from "../components/DetailLister";
import "../components/search-list.css";
import { PageHeader } from "../components/PageHeader";
import Banner from "../components/primitives/Banner";
import Button from "../components/primitives/Button";
import Section from "../components/primitives/Section";
import { formatBytes } from "../lib/format";
import {
	type BundleDetailResponse,
	exportBundle,
	getBundleDetail,
	removeBundle,
} from "../lib/tauri";
import { useConfirmDestructive } from "../lib/useConfirmDestructive";

const PAGE_SIZE = 50;

function createHashList(chart: BundleDetailResponse) {
	return [
		{ alg: "bundle", id: chart.bundle_id },
		{ alg: "sha256", id: chart.chart_sha256 },
		...chart.chart_ids.map((chartId) => {
			const separator = chartId.indexOf("/");
			return { alg: chartId.slice(0, separator), id: chartId.slice(separator + 1) };
		}),
	];
}

export default function ChartDetail() {
	const params = useParams();
	const navigate = useNavigate();
	const [detail] = createResource(() => params.bundleId, getBundleDetail);
	const [exportError, setExportError] = createSignal<null | string>(null);
	const [exporting, setExporting] = createSignal(false);
	const [deleteError, setDeleteError] = createSignal<null | string>(null);
	const [deleting, setDeleting] = createSignal(false);
	const deleteConfirm = useConfirmDestructive();
	const [assetLimit, setAssetLimit] = createSignal(PAGE_SIZE);
	let exportMenu: HTMLDetailsElement | undefined;

	onMount(() => {
		const closeExportMenu = (event: PointerEvent) => {
			if (exportMenu?.open && !exportMenu.contains(event.target as Node)) {
				exportMenu.removeAttribute("open");
			}
		};
		document.addEventListener("pointerdown", closeExportMenu);
		onCleanup(() => document.removeEventListener("pointerdown", closeExportMenu));
	});

	createEffect(
		on(
			() => params.bundleId,
			() => setAssetLimit(PAGE_SIZE),
		),
	);

	async function exportChart(bundleId: string, bbzip: boolean) {
		exportMenu?.removeAttribute("open");
		setExportError(null);
		setExporting(true);
		try {
			await exportBundle(bundleId, bbzip);
		} catch (err) {
			setExportError(String(err));
		} finally {
			setExporting(false);
		}
	}

	function armDelete() {
		exportMenu?.removeAttribute("open");
		setDeleteError(null);
		deleteConfirm.arm();
	}

	async function deleteChart(bundleId: string) {
		setDeleteError(null);
		setDeleting(true);
		try {
			await removeBundle(bundleId);
			navigate("/bundles", { replace: true });
		} catch (err) {
			setDeleteError(String(err));
			setDeleting(false);
			deleteConfirm.cancel();
		}
	}

	return (
		<>
			<p class="breadcrumb">
				<BackLink />
			</p>

			<Show when={detail.loading}>
				<div class="empty-state">Loading chart…</div>
			</Show>

			<Show when={detail.error}>
				<p class="error">Failed to load chart: {String(detail.error)}</p>
			</Show>

			<Show when={detail()}>
				{(chart) => (
					<>
						<PageHeader
							actions={
								<>
									<details class="export-menu" ref={exportMenu}>
										<summary class="export-menu-trigger">
											Export <span aria-hidden="true">▾</span>
										</summary>
										<div class="export-menu-options">
											<button
												disabled={exporting() || deleting()}
												onClick={() =>
													void exportChart(chart().bundle_id, false)
												}
												type="button"
											>
												Export to folder
											</button>
											<button
												disabled={exporting() || deleting()}
												onClick={() =>
													void exportChart(chart().bundle_id, true)
												}
												type="button"
											>
												Export to .bbzip
											</button>
										</div>
									</details>
									<ConfirmActions
										armLabel="Delete"
										confirming={deleteConfirm.confirming()}
										confirmLabel="Confirm delete"
										disabled={exporting()}
										onArm={armDelete}
										onCancel={deleteConfirm.cancel}
										onConfirm={() => void deleteChart(chart().bundle_id)}
										working={deleting()}
										workingLabel="Deleting…"
									/>
								</>
							}
							subtitle={chart().filename}
							title={chart().description || "(untitled)"}
						/>

						<Show when={exportError()}>
							<p class="error">{exportError()}</p>
						</Show>
						<Show when={deleteError()}>
							{(error) => <Banner variant="danger">{error()}</Banner>}
						</Show>
						<Show when={deleteConfirm.confirming() && chart().appearances.length > 0}>
							<Banner variant="warning">
								<b>
									This chart appears in collections. Deleting it will leave those
									collections installed without this chart!
								</b>
							</Banner>
						</Show>

						<Section heading="Appears in">
							<ChartAppearance chart={chart()} />
						</Section>

						<Section heading="Details">
							<DetailLister
								content={(asset) => asset.alg}
								detail={(asset) => <CopyableId value={asset.id} />}
								items={createHashList(chart())}
							/>
						</Section>

						<Section heading={`assets (${chart().assets.length})`}>
							<Show
								fallback={
									<div class="empty-state empty-state-compact">
										This chart has no assets.
									</div>
								}
								when={chart().assets.length > 0}
							>
								<DetailLister
									content={(asset) => asset.path}
									detail={(asset) =>
										asset.size !== null ? formatBytes(asset.size!) : "missing"
									}
									href={(asset) => `/assets/${asset.asset_id}`}
									items={chart().assets.slice(0, assetLimit())}
								/>
								<Show when={chart().assets.length > assetLimit()}>
									<Button
										onClick={() => setAssetLimit((limit) => limit + PAGE_SIZE)}
										variant="base"
									>
										See more
									</Button>
								</Show>
							</Show>
						</Section>
					</>
				)}
			</Show>
		</>
	);
}
