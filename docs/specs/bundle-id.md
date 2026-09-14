---
title: Bundle ID Algorithm
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Overview

A `Bundle ID` is a deterministic identifier for a `.bb` file.

Two `.bb` files with identical chart bytes but different assets, or different filenames, will have different `Bundle ID`s, even though their chart `sha256` is the same.

A `Bundle ID` is the `sha256` checksum of a canonical byte framing built from three inputs taken from a `.bb` file:

- `path` - the `filename` field.
- `chart_sha256` - the `sha256` of the gunzipped chart bytes.
- `assets` - the `assets` map (filename to asset `sha256`).

Please note that `desc` is deliberately ignored, and is not part of the bundle ID.

## Format

A `Bundle ID` is a 32-byte `sha256` checksum derived from the framing below. Its canonical string form is not a bare hex digest; see [String form](#string-form).

## String form

The canonical string form of a `Bundle ID` is the ASCII prefix `b-` followed by the 64-character lowercase hex `sha256`:

```text
b-9a911e91b5a5a1fef3189e417d5b929e55664e58551ad19875289ed92e321d49
```

- Parsers **MUST** require the `b-` prefix. Bare 64-character hex strings **MUST NOT** be accepted as `Bundle ID`s.
- Parsers **MUST** reject strings whose prefix is not exactly `b-`, or whose suffix is not a valid 64-character lowercase hex `sha256`.

## Inputs

- `path` **MUST** be the `filename` field of a `.bb` file.
- `chart_sha256` **MUST** be the 32-byte `sha256` of the gunzipped chart bytes — that is, `sha256(gunzip(base64decode(chart)))` where `chart` is the `.bb` file's `chart` field.
- `assets` **MUST** be the `assets` map from the `.bb` file. Each key is a filename, each value is a 32-byte asset `sha256`.

## Framing

The `Bundle ID` is `sha256` of the following byte framing. All integers are little-endian `u32`. Every variable-length field is prefixed by its byte length, so there is no delimiter ambiguity.

```text
b"b1"                                   (magic header)
u32 filename_len ++ filename bytes      (UTF-8)
32 bytes                                chart_sha256 raw bytes
u32 asset_count
  repeated, sorted by filename ascending (byte-wise, lex order):
    u32 filename_len ++ filename bytes   (UTF-8)
    32 bytes                             asset sha256 raw bytes
```

The framing begins with the 2-byte ASCII string `b1`.

Each variable-length field (`path`, and each `filename`) is written as a little-endian `u32` byte length followed by the raw bytes. The `asset_count` is a little-endian `u32` giving the number of entries that follow. Fields larger than `2^32 - 1` bytes cannot be encoded and **MUST NOT** appear in a bundle frame.

The asset entries **MUST** be sorted by filename in ascending byte-wise (lexicographic) order before encoding.

The `chart_sha256` and each asset `sha256` are written as their 32 raw bytes, **not** as their 64-character hex encoding. The `path` and `filename` fields are written as raw UTF-8 bytes.

## Computation

Given a `.bb` file, computing its `Bundle ID` **MUST** be equivalent to:

1. Decode the `chart` field's base64, then gunzip, then take `sha256` to get `chart_sha256`.
2. Build the byte framing above from `path` (the `filename` field), `chart_sha256`, and the `assets` map (sorted by filename).
3. Take `sha256` of the framed bytes.
4. Render the result as a `Bundle ID` string: `b-` followed by the 64-character lowercase hex `sha256` (see [String form](#string-form)).

Implementations **MUST** produce identical `Bundle ID`s for inputs that differ only in asset insertion order.
