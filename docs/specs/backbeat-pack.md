---
title: Backbeat Pack Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/info/rfc2119/).

## Overview

A Backbeat pack is an ordered set of bundles. Unlike tables and courses, which
refer to chart IDs, a pack uses [Bundle IDs](./bundle-id.md) and therefore
selects the exact chart filename and asset mapping to install.

A pack is a JSON file and **SHOULD** use the `.bbpack` extension. When it is
published as a collection, its filename is `data.bbpack`; see the
[collection endpoint specification](./backbeat-collection-endpoint.md).

## Example

```json
{
	"name": "Example Starter Pack",
	"gamemode": "sm-4k",
	"updated": "2026-09-14T12:00:00Z",
	"tags": {
		"author": "Example Author"
	},
	"assets": {
		"banner.png": "63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c"
	},
	"bundles": [
		{
			"id": "b-0ce1f2c355f96b2a5cf126484a744b76a37bb0a918bc6526d7c054dc405a8cff",
			"desc": "Example Artist - Example Song (Challenge 17)",
			"tags": {
				"sm/song-folder": "Example Song"
			}
		}
	]
}
```

## Fields

All fields are required. Objects **MUST NOT** contain fields not listed below.
An empty pack uses `[]` for `bundles` and `{}` for `tags` or `assets`.

| Field      | Type   | Meaning                                                                        |
| ---------- | ------ | ------------------------------------------------------------------------------ |
| `name`     | string | Human-readable pack name.                                                      |
| `gamemode` | string | Non-empty identifier containing only lowercase ASCII letters, digits, and `-`. |
| `updated`  | string | RFC 3339 timestamp for this revision.                                          |
| `tags`     | object | Arbitrary string-to-string metadata for clients.                               |
| `assets`   | object | Collection asset paths mapped to lowercase SHA-256 hashes.                     |
| `bundles`  | array  | Bundle objects in display order.                                               |

Every bundle object has:

| Field  | Type   | Meaning                                                       |
| ------ | ------ | ------------------------------------------------------------- |
| `id`   | string | `b-` followed by exactly 64 lowercase hexadecimal characters. |
| `desc` | string | Human-readable fallback description.                          |
| `tags` | object | Arbitrary string-to-string metadata for this pack entry.      |

The `id` **MUST** obey the [Bundle ID specification](./bundle-id.md). Asset
paths and hashes follow the `assets` rules in the
[Backbeat file specification](./backbeat-file.md#assets). Pack assets describe
the pack itself, such as its banner; they are separate from each bundle's
dependencies.

## Ordering and identity

Bundle array order is significant and **MUST** be preserved. The same Bundle
ID **MAY** appear more than once because each occurrence is a distinct pack
entry and may carry different tags.

## Publishing and downloading

`updated` is the collection revision. When the pack is served remotely, it
**MUST** represent the same instant as `timestamp` in `header.json`.

Downloading a pack's data asks a data server for each distinct Bundle ID and
collection asset. The downloaded `.bb` files then declare their own asset
dependencies as described by the [Backbeat file specification](./backbeat-file.md).
