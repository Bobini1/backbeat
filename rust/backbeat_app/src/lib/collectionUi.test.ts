import { describe, expect, test } from "vitest";

import {
	collectionDocumentPath,
	collectionSettingsPath,
	decodeCollectionUrl,
	describeCollectionError,
	normalizeCollectionUrl,
} from "./collectionUi";

describe("collection paths", () => {
	test("encodes collection URLs in document and settings paths", () => {
		const url = "https://example.com/collections/a b";
		expect(collectionDocumentPath("pack", url)).toBe(
			"/packs/https%3A%2F%2Fexample.com%2Fcollections%2Fa%20b",
		);
		expect(collectionSettingsPath("pack", url)).toBe(
			"/packs/https%3A%2F%2Fexample.com%2Fcollections%2Fa%20b/settings",
		);
	});

	test("leaves malformed escaped URLs readable", () => {
		expect(decodeCollectionUrl("https%3A%2F%2Fexample.com%2Fa")).toBe("https://example.com/a");
		expect(decodeCollectionUrl("%not-escaped")).toBe("%not-escaped");
	});
});

describe("normalizeCollectionUrl", () => {
	test("adds a scheme and removes known collection documents", () => {
		expect(normalizeCollectionUrl(" example.com/packs/demo/header.json ")).toBe(
			"https://example.com/packs/demo",
		);
		expect(normalizeCollectionUrl("https://example.com/packs/demo/data.bbpack")).toBe(
			"https://example.com/packs/demo",
		);
	});

	test("preserves a collection root without trailing slashes", () => {
		expect(normalizeCollectionUrl("https://example.com/packs/demo///")).toBe(
			"https://example.com/packs/demo",
		);
	});

	test("removes URL fragments", () => {
		expect(normalizeCollectionUrl("https://example.com/packs/demo#charts")).toBe(
			"https://example.com/packs/demo",
		);
		expect(normalizeCollectionUrl("example.com/packs/demo/header.json#charts")).toBe(
			"https://example.com/packs/demo",
		);
	});
});

describe("describeCollectionError", () => {
	test("turns common transport and schema errors into actionable messages", () => {
		expect(describeCollectionError("failed to fetch")).toContain("Couldn't reach");
		expect(describeCollectionError("missing field `charts`", "pack")).toContain(
			"This pack was invalid",
		);
	});
});
