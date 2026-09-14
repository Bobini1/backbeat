---
title: Backbeat Zip Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/info/rfc2119/).

## Overview

A Backbeat zip (`.bbzip`) is a ZIP archive used to carry one or more
[Backbeat files](./backbeat-file.md) and their asset bytes together. It is for
portable, last-mile import and export. It is not a collection format and does
not describe table levels, course order, or pack membership.

`.bbzip` is only a distinct filename extension. The bytes use the ordinary ZIP
archive format; producers **SHOULD** use the `.bbzip` extension rather than
`.zip`.

## Example layout

```text
example.bbzip
├── first-chart.bb
├── second-chart.bb
├── 6098ac7e3b056e5a85c14f8914dd6a8fd25a661b715904c9b57409c8ab829585
└── 63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c
```

The names shown for asset entries are conventional, not meaningful. For
example, the last entry could instead be named `assets/banner` without
changing what gets installed.

## Entry classification

Consumers inspect each file entry's final, case-sensitive extension:

| Entry                   | Meaning                                                                   |
| ----------------------- | ------------------------------------------------------------------------- |
| Name ending in `.bb`    | A Backbeat manifest, parsed according to the Backbeat file specification. |
| Name ending in `.bbzip` | A forbidden nested Backbeat zip. The archive **MUST** be rejected.        |
| Any other file name     | Asset bytes.                                                              |
| Directory entry         | Ignored.                                                                  |

Every `.bb` entry **MUST** be valid. A self-contained archive **MUST** include
asset bytes whose SHA-256 hashes satisfy every asset reference in its `.bb`
entries. Assets shared by several manifests only need to occur once.

An archive **MAY** contain more than one `.bb` entry. It **MAY** also contain
unreferenced asset entries, although producers **SHOULD** omit them. Producers
**SHOULD** deduplicate identical asset contents.

## Asset identity

Archive paths do not establish asset identity. Consumers **MUST** compute
`sha256(entry bytes)` for every asset entry and install the bytes under that
computed Asset ID. They **MUST NOT** trust an entry name that looks like a
checksum.

This means that directory structure and asset filenames inside the archive do
not need to match the paths in a `.bb` manifest. The manifest supplies the
chart-relative path-to-hash mapping; the archive only supplies bytes with those
hashes.

## Safety and compatibility

Consumers **MUST NOT** extract entries to archive-provided paths before
validation. Reading `.bb` manifests and hashing asset entries directly from the
archive avoids path traversal, filename collisions, and platform-specific path
problems.

Nested `.bbzip` file entries are forbidden. Producers **SHOULD** write
unencrypted archives using ZIP features supported by common readers. Entry
order has no meaning and consumers **MUST NOT** depend on it.

Importing a `.bbzip` installs its `.bb` manifests and makes its asset contents
available in the Backbeat store.
