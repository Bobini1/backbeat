export type CollectionKind = "course" | "pack" | "table";

export function describeCollectionError(err: unknown, kind?: CollectionKind): string {
	const raw = String(err);
	if (/failed to fetch|failed to connect|connection|timed out|timeout/i.test(raw)) {
		return "Couldn't reach this collection. Check its URL or try again.";
	}
	if (/server returned 404/i.test(raw)) {
		return "Could not load a collection from this URL! Is this URL definitely a valid backbeat collection?";
	}
	if (/server returned/i.test(raw)) {
		return "The collection endpoint rejected the request.";
	}
	if (
		/missing field|unknown field|unknown variant|invalid type|duplicate field|expected .* at line/i.test(
			raw,
		)
	) {
		const label = kind ? collectionKindLabel(kind).toLowerCase() : "collection";
		return `This ${label} was invalid, complain to the server owner!`;
	}
	if (/invalid|parse|json/i.test(raw)) {
		return "The collection file isn't valid Backbeat collection data.";
	}
	if (/not found/i.test(raw)) {
		return "That collection is not installed anymore.";
	}
	return raw;
}

export function pathForCollectionKind(kind: CollectionKind): null | string {
	switch (kind) {
		case "course":
			return "/courses";
		case "pack":
			return "/packs";
		case "table":
			return "/tables";
		default:
			return null;
	}
}

export function collectionDocumentPath(kind: CollectionKind, url: string): string {
	const base = pathForCollectionKind(kind);
	if (!base) {
		throw new Error(`Unknown collection kind: ${kind}`);
	}
	return `${base}/${encodeURIComponent(url)}`;
}

export function collectionSettingsPath(kind: CollectionKind, url: string): string {
	return `${collectionDocumentPath(kind, url)}/settings`;
}

export function decodeCollectionUrl(encoded: string): string {
	try {
		return decodeURIComponent(encoded);
	} catch {
		return encoded;
	}
}

/** Normalize a pasted collection URL to its collection root. */
export function normalizeCollectionUrl(url: string): string {
	let base = url.trim();
	if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(base)) {
		base = `https://${base}`;
	}
	base = base.split("#", 1)[0];
	base = base.replace(/\/+$/, "");
	if (base.endsWith("/header.json")) {
		return base.slice(0, -"/header.json".length).replace(/\/+$/, "");
	}
	const dataSuffix = base.match(/\/data\.(bbtable|bbpack|bbcourse|bbvf)$/);
	if (dataSuffix) {
		return base.slice(0, -dataSuffix[0].length).replace(/\/+$/, "");
	}
	return base;
}

export function collectionKindLabel(kind: CollectionKind): string {
	switch (kind) {
		case "course":
			return "Course";
		case "pack":
			return "Pack";
		case "table":
			return "Table";
	}
}
