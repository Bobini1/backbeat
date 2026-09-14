import { A, useLocation } from "@solidjs/router";
import { createMemo, Index, Show } from "solid-js";

import {
	type CollectionJob,
	dismissSettledCollectionJobs,
	downloadSummary,
	type DownloadSummary,
	refreshDownloadActivity,
} from "../lib/downloadActivity";
import { clearFinishedDownloads } from "../lib/tauri";
import "./GlobalDownloadStatus.css";

const MAX_VISIBLE_COLLECTION_JOBS = 3;

function jobLabel(job: CollectionJob): string {
	if (job.name.trim()) {
		return job.name;
	}
	switch (job.kind) {
		case "course":
			return "Course";
		case "pack":
			return "Pack";
		case "table":
			return "Table";
	}
}

function primaryLine(summary: DownloadSummary): string {
	const job = summary.activeJobs[0];
	if (job && job.total > 0) {
		return `${jobLabel(job)} ${job.current}/${job.total}`;
	}
	if (job) {
		return `${jobLabel(job)} download`;
	}

	if (summary.running > 0 && summary.queued > 0) {
		return `${summary.running} active · ${summary.queued} queued`;
	}
	if (summary.running > 0) {
		return summary.running === 1 ? "1 download active" : `${summary.running} downloads active`;
	}
	if (summary.queued > 0) {
		return summary.queued === 1 ? "1 queued" : `${summary.queued} queued`;
	}
	return "Downloading…";
}

function secondaryLine(summary: DownloadSummary): null | string {
	const parts: string[] = [];
	const job = summary.activeJobs[0];
	if (job?.item) {
		parts.push(job.item);
	}
	if (summary.activeJobs.length > 1) {
		parts.push(`+${summary.activeJobs.length - 1} more`);
	}
	if (summary.failed > 0) {
		parts.push(summary.failed === 1 ? "1 failed" : `${summary.failed} failed`);
	}
	return parts.length > 0 ? parts.join(" · ") : null;
}

function progressRatio(summary: DownloadSummary): null | number {
	const job = summary.activeJobs[0];
	if (job && job.total > 0) {
		return Math.min(1, job.current / job.total);
	}
	return null;
}

function collectionJobDetail(
	job: CollectionJob,
	additional: number,
	failed: number,
): null | string {
	const parts: string[] = [];
	if (job.item) {
		parts.push(job.item);
	}
	if (additional > 0) {
		parts.push(`+${additional} more`);
	}
	if (failed > 0) {
		parts.push(failed === 1 ? "1 failed" : `${failed} failed`);
	}
	return parts.length > 0 ? parts.join(" · ") : null;
}

function CollectionJobStatus(props: {
	additional: number;
	failed: number;
	job: CollectionJob;
	onDownloadsPage: boolean;
}) {
	const ratio = () =>
		props.job.total > 0 ? Math.min(1, props.job.current / props.job.total) : null;
	const detail = () => collectionJobDetail(props.job, props.additional, props.failed);

	return (
		<div class="dl-activity dl-activity-active">
			<div class="dl-activity-main">
				<span aria-hidden="true" class="dl-activity-spinner" />
				<div class="dl-activity-copy">
					<span class="dl-activity-title">
						{props.job.total > 0
							? `${jobLabel(props.job)} ${props.job.current}/${props.job.total}`
							: `${jobLabel(props.job)} download`}
					</span>
					<Show when={detail()}>
						{(line) => (
							<span
								class={`dl-activity-detail ${props.failed > 0 ? "dl-activity-detail-warn" : ""}`}
							>
								{line()}
							</span>
						)}
					</Show>
				</div>
				<Show when={!props.onDownloadsPage}>
					<div class="dl-activity-actions">
						<A class="dl-activity-action dl-activity-link" href="/downloads">
							Open
						</A>
					</div>
				</Show>
			</div>
			<Show when={ratio() !== null}>
				<div
					aria-label={`${jobLabel(props.job)} download progress`}
					aria-valuemax={100}
					aria-valuemin={0}
					aria-valuenow={Math.round((ratio() ?? 0) * 100)}
					class="dl-activity-track"
					role="progressbar"
				>
					<div class="dl-activity-fill" style={{ width: `${(ratio() ?? 0) * 100}%` }} />
				</div>
			</Show>
		</div>
	);
}

export default function GlobalDownloadStatus() {
	const location = useLocation();
	const summary = createMemo(() => downloadSummary());
	const onDownloadsPage = () => location.pathname === "/downloads";
	const visible = () => summary().phase !== "hidden";
	const ratio = createMemo(() => progressRatio(summary()));
	const visibleJobs = createMemo(() =>
		summary().activeJobs.slice(0, MAX_VISIBLE_COLLECTION_JOBS),
	);
	const hiddenJobs = () => summary().activeJobs.length - visibleJobs().length;
	const showingCollectionJobs = () => summary().phase === "active" && visibleJobs().length > 0;

	async function handleClearFailed(event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		await clearFinishedDownloads();
		dismissSettledCollectionJobs();
		await refreshDownloadActivity();
	}

	return (
		<>
			<Show when={showingCollectionJobs()}>
				<div aria-live="polite" class="dl-activity-stack" role="status">
					<Index each={visibleJobs()}>
						{(job, index) => {
							const last = () => index === visibleJobs().length - 1;
							return (
								<CollectionJobStatus
									additional={last() ? hiddenJobs() : 0}
									failed={last() ? summary().failed : 0}
									job={job()}
									onDownloadsPage={onDownloadsPage()}
								/>
							);
						}}
					</Index>
				</div>
			</Show>
			<Show when={visible() && !showingCollectionJobs()}>
				<div
					aria-live="polite"
					class={`dl-activity dl-activity-${summary().phase}`}
					role="status"
				>
					<div class="dl-activity-main">
						<Show
							fallback={
								<span
									aria-hidden="true"
									class={`dl-activity-dot dl-activity-dot-${summary().phase}`}
								/>
							}
							when={summary().phase === "active"}
						>
							<span aria-hidden="true" class="dl-activity-spinner" />
						</Show>

						<div class="dl-activity-copy">
							<Show when={summary().phase === "active"}>
								<span class="dl-activity-title">{primaryLine(summary())}</span>
								<Show when={secondaryLine(summary())}>
									{(line) => (
										<span
											class={`dl-activity-detail ${summary().failed > 0 ? "dl-activity-detail-warn" : ""}`}
										>
											{line()}
										</span>
									)}
								</Show>
							</Show>
							<Show when={summary().phase === "failed"}>
								<span class="dl-activity-title">
									{summary().failed === 1
										? "1 download failed"
										: `${summary().failed} downloads failed`}
								</span>
								<span class="dl-activity-detail dl-activity-detail-warn">
									{summary().failureMessage ?? "Review and clear when ready"}
								</span>
							</Show>
							<Show when={summary().phase === "caught_up"}>
								<span class="dl-activity-title">Finished!</span>
								<span class="dl-activity-detail">All downloads finished</span>
							</Show>
						</div>

						<div class="dl-activity-actions">
							<Show when={summary().phase === "failed"}>
								<button
									class="dl-activity-action"
									onClick={handleClearFailed}
									type="button"
								>
									Clear
								</button>
							</Show>
							<Show when={!onDownloadsPage() && summary().phase !== "caught_up"}>
								<A class="dl-activity-action dl-activity-link" href="/downloads">
									Open
								</A>
							</Show>
						</div>
					</div>

					<Show when={summary().phase === "active" && ratio() !== null}>
						<div
							aria-label="Download progress"
							aria-valuemax={100}
							aria-valuemin={0}
							aria-valuenow={Math.round((ratio() ?? 0) * 100)}
							class="dl-activity-track"
							role="progressbar"
						>
							<div
								class="dl-activity-fill"
								style={{ width: `${(ratio() ?? 0) * 100}%` }}
							/>
						</div>
					</Show>
				</div>
			</Show>
		</>
	);
}
