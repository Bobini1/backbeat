import { createSignal } from "solid-js";

import {
	type CollectionDownloadEvent,
	downloadOverview,
	type DownloadOverview,
	onCollectionDownloadEvent,
} from "./tauri";

const ACTIVE_POLL_MS = 800;
const IDLE_POLL_MS = 4000;
const CAUGHT_UP_MS = 2400;

export type CollectionJobKind = CollectionDownloadEvent["kind"];

export type CollectionJob = {
	current: number;
	error: null | string;
	item: null | string;
	kind: CollectionJobKind;
	name: string;
	state: "done" | "downloading" | "failed";
	total: number;
	url: string;
};

export type DownloadPhase = "active" | "caught_up" | "failed" | "hidden";

export type DownloadSummary = {
	activeJobs: CollectionJob[];
	badgeCount: null | number;
	cancelled: number;
	done: number;
	failed: number;
	failureMessage: null | string;
	hasFailureAttention: boolean;
	inFlight: number;
	jobs: CollectionJob[];
	phase: DownloadPhase;
	queued: number;
	running: number;
	total: number;
};

const [overview, setOverview] = createSignal<DownloadOverview>({
	total: 0,
	queued: 0,
	running: 0,
	failed: 0,
	done: 0,
	cancelled: 0,
	first_error: null,
});
const [jobs, setJobs] = createSignal<CollectionJob[]>([]);
const [caughtUp, setCaughtUp] = createSignal(false);
const [ready, setReady] = createSignal(false);

let started = false;
let pollTimer: ReturnType<typeof setTimeout> | undefined;
let caughtUpTimer: ReturnType<typeof setTimeout> | undefined;
let wasBusy = false;
const unlistens: Array<() => void> = [];

function jobKey(kind: CollectionJobKind, url: string): string {
	return `${kind}:${url}`;
}

function upsertJob(event: CollectionDownloadEvent) {
	const next: CollectionJob = {
		kind: event.kind,
		name: event.name,
		url: event.url,
		state: event.state,
		current: event.current,
		total: event.total,
		item: event.description ?? event.item?.value ?? null,
		error: event.error,
	};

	setJobs((prev) => {
		const key = jobKey(event.kind, event.url);
		const index = prev.findIndex((job) => jobKey(job.kind, job.url) === key);
		if (event.state === "done") {
			return index === -1 ? prev : prev.filter((_, jobIndex) => jobIndex !== index);
		}
		if (index === -1) {
			return [...prev, next];
		}
		const updated = prev.slice();
		updated[index] = next;
		return updated;
	});

	if (event.state === "done") {
		considerCaughtUp();
	}
}

async function refreshOverview() {
	try {
		setOverview(await downloadOverview());
		setReady(true);
		considerCaughtUp();
	} catch {
		// Keep last known overview; next poll retries.
	}
}

function busyNow(inFlight: number, activeJobs: CollectionJob[]): boolean {
	return inFlight > 0 || activeJobs.length > 0;
}

function considerCaughtUp() {
	const o = overview();
	const activeJobs = jobs().filter((job) => job.state === "downloading");
	const failedJobs = jobs().filter((job) => job.state === "failed").length;
	const inFlight = o.queued + o.running;
	const failed = o.failed + failedJobs;
	const busy = busyNow(inFlight, activeJobs);

	if (busy) {
		wasBusy = true;
		if (caughtUpTimer) {
			clearTimeout(caughtUpTimer);
			caughtUpTimer = undefined;
		}
		setCaughtUp(false);
		return;
	}

	if (wasBusy && failed === 0) {
		wasBusy = false;
		setCaughtUp(true);
		if (caughtUpTimer) {
			clearTimeout(caughtUpTimer);
		}
		caughtUpTimer = setTimeout(() => setCaughtUp(false), CAUGHT_UP_MS);
	} else if (!busy) {
		wasBusy = false;
	}
}

function schedulePoll(delay: number) {
	if (pollTimer) {
		clearTimeout(pollTimer);
	}
	pollTimer = setTimeout(async () => {
		await refreshOverview();
		const summary = downloadSummary();
		const nextDelay =
			summary.phase === "active" || summary.phase === "failed"
				? ACTIVE_POLL_MS
				: IDLE_POLL_MS;
		schedulePoll(nextDelay);
	}, delay);
}

/** Start global download activity tracking once for the app shell. */
export function startDownloadActivity() {
	if (started) {
		return;
	}
	started = true;

	void refreshOverview().then(() => {
		schedulePoll(ACTIVE_POLL_MS);
	});

	void onCollectionDownloadEvent(upsertJob).then((unlisten) => {
		unlistens.push(unlisten);
	});
}

/** Drop finished collection jobs from ambient attention (after Clear). */
export function dismissSettledCollectionJobs() {
	setJobs((prev) => prev.filter((job) => job.state === "downloading"));
	considerCaughtUp();
}

/** Force an overview refresh (e.g. after clear/cancel on the Downloads page). */
export async function refreshDownloadActivity() {
	await refreshOverview();
}

export function downloadActivityReady() {
	return ready();
}

/** Live aggregate for ambient UI. Call from a reactive context. */
export function downloadSummary(): DownloadSummary {
	const o = overview();
	const jobList = jobs().slice();
	const activeJobs = jobList.filter((job) => job.state === "downloading");
	const failedJobs = jobList.filter((job) => job.state === "failed").length;
	const failed = o.failed + failedJobs;
	const inFlight = o.queued + o.running;
	const busy = busyNow(inFlight, activeJobs);

	let phase: DownloadPhase = "hidden";
	if (busy) {
		phase = "active";
	} else if (failed > 0) {
		phase = "failed";
	} else if (caughtUp()) {
		phase = "caught_up";
	}

	const failureFromJob =
		jobList.find((job) => job.state === "failed" && job.error)?.error ?? null;

	return {
		queued: o.queued,
		running: o.running,
		inFlight,
		failed,
		done: o.done,
		cancelled: o.cancelled,
		total: o.total,
		jobs: jobList,
		activeJobs,
		failureMessage: o.first_error ?? failureFromJob,
		phase,
		badgeCount: inFlight > 0 ? inFlight : null,
		hasFailureAttention: failed > 0,
	};
}
