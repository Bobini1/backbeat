import { describe, expect, test } from "vitest";

import { basename, formatBytes, formatTimestamp, truncateHash } from "./format";

describe("formatBytes", () => {
	test("uses bytes below one kibibyte", () => {
		expect(formatBytes(0)).toBe("0 B");
		expect(formatBytes(1023)).toBe("1023 B");
	});

	test("scales and rounds larger values", () => {
		expect(formatBytes(1024)).toBe("1.0 KiB");
		expect(formatBytes(1536)).toBe("1.5 KiB");
		expect(formatBytes(1024 ** 3)).toBe("1.0 GiB");
	});
});

describe("path and hash display", () => {
	test("keeps short hashes and truncates longer ones", () => {
		expect(truncateHash("deadbeef")).toBe("deadbeef");
		expect(truncateHash("deadbeefcafebabe")).toBe("deadbeef...");
		expect(truncateHash("abcdef", 4)).toBe("abcd...");
	});

	test("finds a basename on either path separator", () => {
		expect(basename("songs/Artist/track.ogg")).toBe("track.ogg");
		expect(basename("songs\\Artist\\track.ogg")).toBe("track.ogg");
	});
});

test("formats absent timestamps as Never and preserves invalid timestamps", () => {
	expect(formatTimestamp(null)).toBe("Never");
	expect(formatTimestamp(undefined)).toBe("Never");
	expect(formatTimestamp("not-a-date")).toBe("not-a-date");
});
