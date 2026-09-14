---
title: Precombined Assets Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119.

## Overview

Precombined assets are an optional transport optimisation for Backbeat data
servers. A server can return one compressed archive containing all the assets
referenced by a bundle, allowing a client to replace many small asset requests
with one request.

This is especially valuable for BMS files, which can have thousands of assets.

Precombined assets are interchangeable with downloading the same assets one at a
time.

The archive is identified by a `Combined Assets ID`, which identifies the
asset path-to-content mapping.

## Asset Set

An asset set is a map from an [`AssetPath`](./backbeat-file.md#assets) to an
`AssetId`:

```text
asset path -> sha256(asset bytes)
```

Every path MUST be a valid Backbeat asset path. Every value MUST be a lowercase
SHA-256 checksum represented by 32 raw bytes when used in the ID algorithm.

Different paths MAY refer to the same `AssetId`. For example, `music.ogg` and
`preview.ogg` may contain identical bytes. Both paths still belong in the
archive because the paths are part of the bundle's asset map.

An empty asset set is valid, although clients normally do not request a
precombined archive for it.

## Archive Format

A precombined-assets response MUST be a gzip-compressed tar stream (`tar.gz`)
containing the assets in the requested asset set.

Each archive entry MUST:

- be a regular file;
- have a UTF-8 pathname;
- have a valid Backbeat asset path as its pathname;
- correspond exactly to one path in the expected asset set; and
- contain bytes whose SHA-256 equals the `AssetId` expected for that path.

The archive MUST contain every expected path exactly once. It MUST NOT contain
extraneous paths, duplicate paths, symlinks, or hard links.

The archive MAY contain entries in any order.
