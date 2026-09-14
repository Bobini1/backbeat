---
title: Backbeat File Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Overview

A backbeat (`.bb`) file is a "package" format for rhythm game charts. It takes a primary
"chart" file and bundles together references to the assets it needs.

A `.bb` file is a JSON file, and all of RFC8259 applies here.

Backbeat clients can freely consume these files in order to install charts.

These files are central to backbeat. They are the "unit" of shipping charts; a `.bb` file should tell you
everything you need to know in order to load the chart into
your game correctly. This is distinct from just having the
chart file, as the chart file will not include or ensure assets.

There's not one answer to the question "what is the file `song.mp3` needed for a given chart",
and so packages may disagree; one packager may use a compressed version of the
song, or the chart could appear in multiple places with different backgrounds, but
the same background filename.

## Full Example

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

## File Extension

Backbeat files **SHOULD** have the extension `.bb`. Other extensions **SHOULD NOT**
be recognised, this includes `.json`.

## Data

Inside the backbeat file there are three fields.

Backbeat files **MUST NOT** contain extra top-level fields. Parsers **MUST** reject backbeat
files with extra top-level fields.

### Filename

This is packager-set metadata about the chart. Most importantly, this field
contains the file extension for this chart, which is an important piece in
knowing how to parse the chart data you've been given. Backbeat provides the
whole filename instead of just the file extension, because some games don't use file extensions to identify charts.

Please note - this field is packager set, and not verifiable.

- The `filename` field **MUST** exist.
- Filenames **MUST** be strings.
- They **MUST NOT** be the empty string.
- They **MUST NOT** contain the `/` character (ascii 0x2F).
- They **MUST NOT** contain the null byte character (ascii 0x00).
- They **MUST NOT** be the empty string.
- They **MUST** be valid UTF-8.
- They **MUST NOT** start or end with whitespace. ‘Whitespace’ is defined according to the terms of the Unicode Derived Core Property White_Space, which includes newlines.
- They **MUST NOT** have the file extension `.bb`, which is to say that they end with `.bb` after lowercasing.
- They **MUST NOT** be an [illegal filename](./backbeat-file.md#illegal-filenames).

### Assets

Assets are the pointers to the other files that this chart needs in order to
start up. If you are looking to load a backbeat file, the `assets` tell you what
files need to be accessed when the chart references "file.ogg", etc.

- The `assets` field **MUST** exist.
- It **MUST** be an object that maps strings to strings.
- The object **MUST NOT** contain duplicate keys. Implementations **SHOULD** reject duplicate keys
- The object **SHOULD NOT** contain overlapping paths likely to cause issues. For example, having `foo.wav` and `FOO.wav`.
- An empty object is OK.
- Packagers **SHOULD** avoid having multiple filenames that differ only by casing.
- File paths **MUST NOT** be absolute paths, i.e. they must not start with a forward slash (`/`).
- File paths **MUST NOT** contain nul bytes (`\0`).
- File paths **MUST NOT** end in a forward slash (`/`).
- File paths **MUST NOT** be an empty string.
- File paths **MUST NOT** contain backslash (`\`) characters. Paths must be normalised to use forward slashes (`/`).
- File paths **MUST** be valid UTF-8.
- File paths **MUST NOT** start or end with whitespace. ‘Whitespace’ is defined according to the terms of the Unicode Derived Core Property White_Space, which includes newlines.
- File paths **MUST NOT** be an [illegal filename](./backbeat-file.md#illegal-filenames).

### Desc

The desc field is a human readable description of what the chart is. Generally,
people put things like `Artist - Song Title (Difficulty)` here.

- The `desc` field **MUST** exist.
- It **MUST** be a string.
- It **MUST NOT** exceed 10,000 bytes. Implementations **MUST** reject `desc` fields longer than 10,000 bytes.
- Implementations **MAY** use this to display what a chart is, instead of having to implement game-specific file format parsing.

### Chart

The chart field is the actual chart data. It is gzip compressed
and then base64 encoded.

The gzip compression helps massively with most games, as most bespoke chart formats are
very compressible.

The base64 encoding is necessary _anyway_ as some games might have non-utf8 formats, but after gzipping it is definitely necessary.

**Chart files are not necessarily UTF-8 strings**. They may be in Shift-JIS, and they may be binary files.

The contents of charts **MUST NOT** be multiple "playable charts". That is to say, if the chart data contains two playable levels (like `.sm` files), the packager **MUST** break them up into multiple packages. One bundle **MUST** refer to one playable entity.

Implementations **SHOULD** expose the uncompressed chart data as a byte array (e.g. `Vec<u8>`), and **SHOULD NOT** convert it or interpret it as a string.

- The `chart` field **MUST** be a string.
- It **MUST** be valid base64, using the RFC4648 alphabet: `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/`, with = for padding.
- It **MUST** be valid GZIP content, once base64 decoded.
- Implementations should be aware that, once decoded, parsing of the underlying chart data is outside the scope of this specification.
- Implementations **MUST** refuse chart fields that unzip to more than (`>`) one gigabyte (`1024 * 1024 * 1024`) of data.
- Implementations **MAY** mandate a smaller decompression limit.

#### Illegal Filenames

The following filenames are illegal, for compatibility reasons:

- `.`
- `..`
- Anything that ends with `/.`
- Anything that ends with `/..`
- Anything that contains a unicode character less than `\u{20}`
- Anything that contains `<`, `>`, `:`, `"`, `|`, `?`, `*` or `\`.

And containing any segment - that is, after splitting on `/`, trimming off the extension, that is one of the following (case insensitive):

- `CON`
- `PRN`
- `AUX`
- `NUL`
- `CLOCK$`
- `COM1`, `COM2`, `COM3`, `COM4`, `COM5`, `COM6`, `COM7`, `COM8`, `COM9`
- `LPT1`, `LPT2`, `LPT3`, `LPT4`, `LPT5`, `LPT6`, `LP7`, `LPT8`, `LPT9`
- `COM¹`, `COM²`, `COM³`
- `LPT¹`, `LPT²`, `LPT³`

e.g. `CON.txt`, `con.txt` and `foo/CON/bar.txt` are all illegal.

This is because these filenames cannot be constructed reasonably on windows.
