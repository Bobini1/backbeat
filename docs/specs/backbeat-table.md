---
title: Backbeat Table Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/info/rfc2119/).

## Overview

A Backbeat table maps charts to named difficulty levels. Tables describe an
ordering and a rating system; they do not contain chart data or install one
particular packaging of a chart.

A table is a JSON file and **SHOULD** use the `.bbtable` extension. When it is
published as a collection, its filename is `data.bbtable`; see the
[collection endpoint specification](./backbeat-collection-endpoint.md).

## Example

```json
{
	"name": "Example Difficulty Table",
	"symbol": "★",
	"gamemode": "bms-7k",
	"updated": "2026-09-14T12:00:00Z",
	"tags": {
		"author": "Example Author"
	},
	"assets": {
		"banner.png": "63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c"
	},
	"levels": [
		{
			"level": "1",
			"tags": {
				"color": "#8fd14f"
			},
			"charts": [
				{
					"id": "md5/0108ff92119d48e0ca0e2eae1f3dffbf",
					"desc": "ERIS MX - Grand Thaw",
					"tags": {
						"scratch": "yes"
					}
				}
			]
		}
	],
	"folders": [
		{
			"name": "Level 1 scratch charts",
			"query": "level == \"1\" and tags.scratch == \"yes\"",
			"tags": {}
		}
	]
}
```

## Fields

All fields below are required. Objects **MUST NOT** contain fields not listed
for that object. A table with no levels, folders, charts, tags, or assets uses
an empty array or object for the corresponding field.

| Field      | Type   | Meaning                                                                        |
| ---------- | ------ | ------------------------------------------------------------------------------ |
| `name`     | string | Human-readable table name.                                                     |
| `symbol`   | string | Prefix displayed with a level, such as `★` in `★12`.                           |
| `gamemode` | string | Non-empty identifier containing only lowercase ASCII letters, digits, and `-`. |
| `updated`  | string | RFC 3339 timestamp for this revision.                                          |
| `tags`     | object | Arbitrary string-to-string metadata for clients.                               |
| `assets`   | object | Collection asset paths mapped to lowercase SHA-256 hashes.                     |
| `levels`   | array  | Level objects in display order.                                                |
| `folders`  | array  | Additional folder objects in display order.                                    |

Every level object has:

| Field    | Type   | Meaning                                                                                       |
| -------- | ------ | --------------------------------------------------------------------------------------------- |
| `level`  | string | The level label. Levels are strings, not numbers. Labels **MUST** be unique within the table. |
| `tags`   | object | Arbitrary string-to-string level metadata.                                                    |
| `charts` | array  | Chart objects in display order.                                                               |

Every chart object has:

| Field  | Type   | Meaning                                                                                                |
| ------ | ------ | ------------------------------------------------------------------------------------------------------ |
| `id`   | string | A chart ID in `algorithm/value` form. This deliberately identifies chart content rather than a bundle. |
| `desc` | string | Human-readable fallback description.                                                                   |
| `tags` | object | Arbitrary string-to-string chart metadata. Use `{}` when there are no tags.                            |

A chart ID algorithm is `sha256` or a custom name of at most 128 characters.
Custom names use lowercase ASCII letters and digits separated by single
hyphens. The value **MUST NOT** contain `/`, `?`, or `#`.

Asset paths and hashes follow the `assets` rules in the
[Backbeat file specification](./backbeat-file.md#assets). Collection assets
are presentation resources such as banners; they are not dependencies of the
charts listed in the table.

## Folders

Every folder object has a string `name`, a string `query`, and a string-to-string
`tags` object. `query` uses [tinyfilter](https://github.com/zkldi/tinyfilter)
syntax and is evaluated once for every chart with this context:

| Name         | Value                                                  |
| ------------ | ------------------------------------------------------ |
| `level`      | The chart's level label as a string.                   |
| `level_tags` | The containing level's tags as a string-to-string map. |
| `tags`       | The chart's tags as a string-to-string map.            |

A chart appears in the folder when the expression evaluates to `true`.
Clients **MUST** treat invalid expressions, missing values, and non-boolean
results as no match.

## Publishing and downloading

The order of levels and folders is significant and **MUST** be
preserved. `updated` is the collection revision: when served remotely it
**MUST** represent the same instant as `timestamp` in `header.json`.

Downloading a table's data asks a data server for each distinct chart ID and
collection asset. Since a chart ID can resolve to more than one bundle, a table
does not guarantee a specific set of packaged assets. Use a
[pack](./backbeat-pack.md) when exact bundles are required.
