---
title: Backbeat Course Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/info/rfc2119/).

## Overview

A Backbeat course is an ordered sequence of charts, such as a dan course or a
set intended to be played in one session. A course identifies chart content,
not a particular bundle containing that content.

A course is a JSON file and **SHOULD** use the `.bbcourse` extension. When it is
published as a collection, its filename is `data.bbcourse`; see the
[collection endpoint specification](./backbeat-collection-endpoint.md).

## Example

```json
{
	"name": "Example 7K Course",
	"gamemode": "bms-7k",
	"updated": "2026-09-14T12:00:00Z",
	"tags": {
		"bms/gauge": "lr2"
	},
	"assets": {
		"banner.png": "63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c"
	},
	"charts": [
		{
			"id": "md5/6d64d463466a08eab2fa0768a298bb58",
			"desc": "SHIKI - Air [Long]",
			"tags": {}
		},
		{
			"id": "md5/c46a81cb184f5a804c119930d6eba748",
			"desc": "xi - FREEDOM DiVE [FOUR DIMENSIONS]",
			"tags": {
				"stage": "final"
			}
		}
	]
}
```

## Fields

All fields are required. Objects **MUST NOT** contain fields not listed below.
An empty course uses `[]` for `charts` and `{}` for `tags` or `assets`.

| Field      | Type   | Meaning                                                                        |
| ---------- | ------ | ------------------------------------------------------------------------------ |
| `name`     | string | Human-readable course name.                                                    |
| `gamemode` | string | Non-empty identifier containing only lowercase ASCII letters, digits, and `-`. |
| `updated`  | string | RFC 3339 timestamp for this revision.                                          |
| `tags`     | object | Arbitrary string-to-string metadata for clients, such as gauge rules.          |
| `assets`   | object | Collection asset paths mapped to lowercase SHA-256 hashes.                     |
| `charts`   | array  | Chart objects in play order.                                                   |

Every chart object has:

| Field  | Type   | Meaning                                                    |
| ------ | ------ | ---------------------------------------------------------- |
| `id`   | string | A chart ID in `algorithm/value` form.                      |
| `desc` | string | Human-readable fallback description.                       |
| `tags` | object | Arbitrary string-to-string metadata for this course entry. |

A chart ID algorithm is `sha256` or a custom name of at most 128 characters.
Custom names use lowercase ASCII letters and digits separated by single
hyphens. The value **MUST NOT** contain `/`, `?`, or `#`.

Asset paths and hashes follow the `assets` rules in the
[Backbeat file specification](./backbeat-file.md#assets). Course assets are
presentation resources for the course itself; they are not chart dependencies.

## Ordering and identity

Array order is the course's play order and **MUST** be preserved. The same
chart ID **MAY** occur more than once, because each occurrence is a distinct
course entry and may have different tags.

Chart IDs deliberately allow a data server to choose any matching bundle. Use
a [pack](./backbeat-pack.md) instead if the collection must name exact bundles.

## Publishing and downloading

`updated` is the collection revision. When the course is served remotely, it
**MUST** represent the same instant as `timestamp` in `header.json`.

Downloading a course's data asks a data server for each distinct chart ID and
collection asset.
