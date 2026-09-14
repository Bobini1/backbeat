import { invoke } from "@tauri-apps/api/core";

export function markStartupAt(label: string, elapsedMs: number) {
	void invoke("startup_mark", {
		label,
		elapsedMs,
	}).catch(() => {
		// Startup instrumentation must never affect app boot.
	});
}

export function markStartup(label: string) {
	markStartupAt(label, performance.now());
}

function resourceLabel(entry: PerformanceResourceTiming) {
	let name = entry.name;
	try {
		const url = new URL(entry.name);
		name = `${url.pathname}${url.search}`;
	} catch {
		// Keep the original name if it is not a URL.
	}
	if (name.length > 120) {
		name = `${name.slice(0, 117)}...`;
	}
	return name;
}

export function reportStartupResources() {
	const resources = performance
		.getEntriesByType("resource")
		.filter((entry): entry is PerformanceResourceTiming => entry.entryType === "resource")
		.filter(
			(entry) =>
				entry.initiatorType === "script" ||
				entry.initiatorType === "link" ||
				entry.initiatorType === "css",
		);

	for (const entry of [...resources].sort((a, b) => b.duration - a.duration).slice(0, 8)) {
		markStartupAt(
			`slow resource ${Math.round(entry.duration)}ms ${resourceLabel(entry)}`,
			entry.responseEnd,
		);
	}

	for (const entry of [...resources].sort((a, b) => b.responseEnd - a.responseEnd).slice(0, 8)) {
		markStartupAt(
			`late resource end=${Math.round(entry.responseEnd)}ms ${resourceLabel(entry)}`,
			entry.responseEnd,
		);
	}
}
