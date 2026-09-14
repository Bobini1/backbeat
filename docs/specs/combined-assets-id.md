---
title: Combined Assets ID Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119.

## Overview

The combined assets ID is a way of deterministically identifying a list of asset references.

In a backbeat file, you have fields like this:

```json
{
	"filename": "0x1311.sm",
	"assets": {
		"0x1311.png": "63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c",
		"0x1311.mp3": "6098ac7e3b056e5a85c14f8914dd6a8fd25a661b715904c9b57409c8ab829585",
		"NVLM-cdtitle.png": "db5a1a5b0cdabec3e45d4c3613276f28c141d612cd7008560efc741c04cfb99f",
		"0x1311-bg.png": "c201579dd1b1489727fe97295a02fc06e3722d8fedf52ba6ceb8b4197b5cda54",
		"readme.txt": "cf3724dda7273ba380e1508461f6aa5aab8c311701207addfaaf8f4ef0850e6b"
	},
	"desc": "xi - FREEDOM DiVE (ANOTHER)",
	"chart": "H4sIAHJMRGoCA8vI5AIAenpv7QMAAAA="
}
```

If a bundle ID determistically identifies `(filename, assets, chart)`, then the combined assets ID determistically identifies `(assets)`.

## Combined Assets ID

A Combined Assets ID is the SHA-256 checksum of a canonical framing of the
complete asset set. Its string form is:

```text
a-<64 lowercase hexadecimal characters>
```

The framing is:

```text
u32 asset_count                         (little-endian)
repeat asset_count times, sorted by path in ascending byte-wise order:
    u32 path_length                     (little-endian)
    path bytes                          (raw UTF-8)
    32 bytes                            (raw asset SHA-256)
```

The path length and asset count are byte lengths/counts, not character counts.
Asset paths MUST be sorted lexicographically by their UTF-8 byte sequences
before framing. Hashes MUST be written as their 32 raw bytes, not as 64 hex
characters.

The Combined Assets ID is the SHA-256 of the resulting framing. The canonical
string form prefixes the lowercase hexadecimal digest with `a-`.

Implementations MUST produce the same ID regardless of the order of the entries
in an `assets` map (hence the sorting).
