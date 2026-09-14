const BYTE_UNITS = ["B", "KiB", "MiB", "GiB", "TiB"] as const;

/** Format a byte count in the most appropriate unit: `"123 B"`, `"1.5 GiB"`, etc. */
export function formatBytes(n: number): string {
	let value = n;
	let unit: (typeof BYTE_UNITS)[number] = BYTE_UNITS[0];

	for (const candidate of BYTE_UNITS.slice(1)) {
		if (value < 1024) {
			break;
		}
		value /= 1024;
		unit = candidate;
	}

	return unit === "B" ? `${n} B` : `${value.toFixed(1)} ${unit}`;
}

const HASH_PREFIX_LEN = 8;

/** Show the first N characters of a hex hash for compact display. */
export function truncateHash(hash: string, prefixLen = HASH_PREFIX_LEN): string {
	if (hash.length <= prefixLen) {
		return hash;
	}
	return `${hash.slice(0, prefixLen)}...`;
}

/** Final path segment of a bundle-relative asset path. */
export function basename(path: string): string {
	const segment = path.split(/[/\\]/).pop();
	return segment ?? path;
}

const RELATIVE_THRESHOLDS: [number, Intl.RelativeTimeFormatUnit][] = [
	[60, "second"],
	[3600, "minute"],
	[86400, "hour"],
	[604800, "day"],
	[2592000, "week"],
	[31536000, "month"],
	[Infinity, "year"],
];

const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

/**
 * Returns a string like `"2 hours ago (Jul 7, 2026)"`.
 * Falls back to the raw ISO string if the date is unparseable.
 */
export function timeAgo(isoString: string): string {
	const date = new Date(isoString);
	if (Number.isNaN(date.getTime())) {
		return isoString;
	}

	const diffSecs = (date.getTime() - Date.now()) / 1000;
	const absDiff = Math.abs(diffSecs);

	let unit: Intl.RelativeTimeFormatUnit = "second";
	let value = diffSecs;

	for (let i = 0; i < RELATIVE_THRESHOLDS.length - 1; i++) {
		const [threshold, u] = RELATIVE_THRESHOLDS[i];
		if (absDiff < threshold) {
			unit = u;
			const divisors: Record<string, number> = {
				second: 1,
				minute: 60,
				hour: 3600,
				day: 86400,
				week: 604800,
				month: 2592000,
				year: 31536000,
			};
			value = diffSecs / (divisors[u] ?? 1);
			break;
		}
		if (i === RELATIVE_THRESHOLDS.length - 2) {
			unit = "year";
			value = diffSecs / 31536000;
		}
	}

	const relative = rtf.format(Math.round(value), unit);
	const absolute = date.toLocaleString("en", {
		month: "short",
		day: "numeric",
		year: "numeric",
		hour: "numeric",
		minute: "2-digit",
	});
	return `${relative} (${absolute})`;
}

/** Pretty-print an optional ISO timestamp, or `"Never"` when absent. */
export function formatTimestamp(isoString: null | string | undefined): string {
	if (!isoString) {
		return "Never";
	}
	return timeAgo(isoString);
}
