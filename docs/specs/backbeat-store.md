---
title: Backbeat Store Specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Overview

A backbeat store is how charts and assets are stored on a user's computer. It combines a SQLite database with a content
addressable store to create a space-efficient and performant storage solution for rhythm game content.

A backbeat store looks like this:

```
.downloading/
	01258ca0-1af6-4e8f-8fa6-d0188cdcd61e
	483ae1e2-689b-494c-8157-9e4776976543
.assets/
	60/98/ac7e3b056e5a85c14f8914dd6a8fd25a661b715904c9b57409c8ab829585
	63/07/3fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c
	db/5a/1a5b0cdabec3e45d4c3613276f28c141d612cd7008560efc741c04cfb99f
backbeat.db
.metadata_never_index
```

## Definitions

### Fanned-sha256

The concept of `fanned-sha256` is used in backbeat. This is a lowercase sha256 checksum. Take the first two hex characters as the first directory, the second two as the second directory, and the remaining 60 characters as the filename.

Given the checksum `63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c`, the `fanned-sha256` will look like this:

`63/07/3fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c`

This is intended for filesystem usage; the "fanning out" of paths ensures that files get distributed into smaller subfolders. This prevents issues on some filesystems when folders have too many files in them.

Unfanning sha256 is the process of taking a fanned path: `63/07/3fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c` and returning to `63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c`.

## `.metadata_never_index`

Implementations **MUST** provide an empty `.metadata_never_index` file at the root of the store.

This prevents macOS Finder from trying to index the `.assets/` folder.

## `.downloading/` folder

The `.downloading/` folder is a folder for temporary downloads while they are being streamed to disk. This allows for large file downloads without holding them in memory.

A folder called `.downloading/` **MUST** exist inside a backbeat store.

Implementations **SHOULD** stream downloads larger than the backbeat config's `downloads.stream` threshold to this folder, and **SHOULD** move the asset into its correct location under `assets/`. It is **RECOMMENDED** to use an atomic rename primitive to do this. (TODO, fact check this)

Implementations **SHOULD** choose globally unique names for downloads, and **SHOULD NOT** use the sha256 key, as multiple downloads for the same file across two clients will conflict.

It is **RECOMMENDED** that implementations use a V4 UUID for temporary downloads.

## `.assets/` folder

The `.assets/` folder is for storing larger asset data. This is a content addressable store using sha256.

Shorter asset data **MAY** be stored in `backbeat.db` instead, as a small-file optimisation.

- Files **MUST** be stored under `.assets/ + fanned-sha256(sha256(file))`. That means that a file with a sha256 of `63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c` **MUST** be stored under `.assets/63/07/3fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c`.
- Files **MUST** have filepaths that agree with their hash. The file stored at `63/07/3fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c` **MUST** have a `sha256` of `63073fba1bfe7a041bde5e851ba930fd95367136cd9137b32a6e761fc6e2992c`.
- Files **MUST NOT** have any file extension.
- Implementations **MUST** ignore files with a file name that is not a 60-character lowercase hex string. This is because some operating systems like to put `.DS_Store`, `Thumbs.db` next to files.
- Implementations **MAY** choose to delete non-sha256 files (e.g. `.assets/63/07/Thumbs.db`).

## `backbeat.db` database

`backbeat.db` is a SQLite database file.

Perhaps surprisingly, `backbeat.db` is part of the stable, public API for Backbeat.
Games that integrate with backbeat are expected to be able to do **read-only** queries against the backbeat tables.

Directly writing to the `backbeat.db` is explicitly forbidden, and clients **MUST NOT** obtain a writable connection to `backbeat.db`.

You are guaranteed the exact tables:

### `chart_data`

This is a table that stores chart bytes, **gzipped**, keyed by their sha256s.

```sql
CREATE TABLE "chart_data" (
	sha256 TEXT PRIMARY KEY NOT NULL,
	gzip_data BLOB NOT NULL,
	uncompressed_size INTEGER NOT NULL
) STRICT;
```

### `chart_id`

Additional chart IDs for charts. This stores strings like `md5/2b00042f7481c7b056c4b410d28f33cf`. Note that all charts _must_ have `sha256` calculated for them, so queries get a little hairy here.

These are more aptly called "additional chart ids".

```sql
CREATE TABLE "chart_id" (
	chart_sha256 TEXT NOT NULL REFERENCES chart_data(sha256) ON DELETE CASCADE,
	-- A string like "md5/2b00042f7481c7b056c4b410d28f33cf".
	id TEXT NOT NULL,

	PRIMARY KEY (chart_sha256, id)
) STRICT;
```

### `asset_map`

Every set of assets that are installed end up here, keyed by combined_assets_id. See the [combined-assets-id](./combined-assets-id.md) spec for more information on how that is calculated.

This is done this way, instead of just an `asset` table, because charts (sabuns in bms, for example) frequently share large sets of assets. Without this design, every new BMS chart for the same song, would add 1000 new rows to the asset table. This results in colossal databases!

```sql
CREATE TABLE "asset_map" (
	combined_assets_id TEXT NOT NULL,
	path TEXT NOT NULL,
	sha256 TEXT NOT NULL,
	PRIMARY KEY (combined_assets_id, path)
) STRICT;
```

### `bundle`

These are all the bundles this user has installed.

```sql
CREATE TABLE "bundle" (
	-- the bundle id
	id TEXT PRIMARY KEY NOT NULL,
	-- the sha256 of the chart bytes - after base64 decoding and gunzipping.
	chart_sha256 TEXT NOT NULL REFERENCES chart_data(sha256),
	-- the filename from the bundle
	filename TEXT NOT NULL,
	-- This is everything after the last "." in the filename field above.
	-- This is pre-stored for you so that you get a fast indexed lookup
	-- on this extremely common operation. If you need finer filename
	-- lookups, use the `filename` field.
	extension TEXT COLLATE NOCASE,
	combined_assets_id TEXT NOT NULL,
	-- "desc" in a bbfile. "desc" is a reserved word in sql, so
	-- description here.
	description TEXT NOT NULL
) STRICT;
```

### `downloaded_asset`

Assets are stored in `asset_map` regardless of whether they're installed or not. This table keeps track of the ones you've actually downloaded.

```sql
CREATE TABLE "downloaded_asset" (
	sha256 TEXT PRIMARY KEY NOT NULL,
	size INTEGER NOT NULL,
	-- small assets are inlined into the sql database.
	-- large assets are stored in the asset store, see above.
	inline_data BLOB
) STRICT;
```

### `pack`

Packs are groups of bundles.

```sql
CREATE TABLE "pack" (
	url TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	gamemode TEXT NOT NULL,
	updated TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}'
) STRICT;
```

### `pack_asset`

Packs may, optionally, contain filename->asset maps. These may be used for things
like banners, wallpapers, etc.

```sql
CREATE TABLE "pack_asset" (
	url TEXT NOT NULL REFERENCES pack(url) ON DELETE CASCADE,
	path TEXT NOT NULL,
	sha256 TEXT NOT NULL,

	PRIMARY KEY (url, path)
) STRICT;
```

### `pack_entry`

```sql
CREATE TABLE "pack_entry" (
	url TEXT NOT NULL REFERENCES pack(url) ON DELETE CASCADE,
	entry INTEGER NOT NULL CHECK (entry > 0),
	bundle_id TEXT NOT NULL,
	desc TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}',

	PRIMARY KEY (url, entry),
	UNIQUE (url, entry)
) STRICT;
```

### `course`

Courses are an ordered list of charts. They are most well known for their usage
in dan courses, but are also sometimes used more generally.

```sql
CREATE TABLE "course" (
	url TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	gamemode TEXT NOT NULL,
	updated TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}'
) STRICT;
```

### `course_asset`

Courses may, optionally, contain filename->asset maps. These may be used for things
like banners, wallpapers, etc.

```sql
CREATE TABLE "course_asset" (
	url TEXT NOT NULL REFERENCES course(url) ON DELETE CASCADE,
	path TEXT NOT NULL,
	sha256 TEXT NOT NULL,

	PRIMARY KEY (url, path)
) STRICT;
```

### `course_chart`

A chart entry (and its index) in a course.

```sql
CREATE TABLE "course_chart" (
	url TEXT NOT NULL REFERENCES course(url) ON DELETE CASCADE,
	entry INTEGER NOT NULL CHECK (entry > 0),
	id TEXT NOT NULL,
	desc TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}',

	PRIMARY KEY (url, entry)
) STRICT;
```

### `difftable`

A difficulty table is a mapping of chart ids to a difficulty value. These are intended to let users
define rating systems, and so on.

`table` is a reserved word in SQL, hence the name.

```sql
CREATE TABLE "difftable" (
	url TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	symbol TEXT NOT NULL,
	updated TEXT NOT NULL,
	gamemode TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}'
) STRICT;
```

### `difftable_asset`

Tables may, optionally, contain filename->asset maps, for things like banners.

```sql
CREATE TABLE "difftable_asset" (
	url TEXT NOT NULL REFERENCES difftable(url) ON DELETE CASCADE,
	path TEXT NOT NULL,
	sha256 TEXT NOT NULL,

	PRIMARY KEY (url, path)
) STRICT;
```

### `difftable_level`

A level in a table is a grouping of charts. Levels are not numbers - they are strings,
and the order of levels is kept in sync here with `level_order`.

```sql
CREATE TABLE "difftable_level" (
	url TEXT NOT NULL REFERENCES difftable(url) ON DELETE CASCADE,
	level TEXT NOT NULL,
	level_order INTEGER NOT NULL CHECK (level_order > 0),
	tags TEXT NOT NULL DEFAULT '{}',

	PRIMARY KEY (url, level),
	UNIQUE (url, level_order)
) STRICT;
```

### `difftable_chart`

An entry in a table. This maps a chart to a difficulty level in the table.

```sql
CREATE TABLE "difftable_chart" (
	url TEXT NOT NULL,
	level TEXT NOT NULL,
	chart_order INTEGER NOT NULL CHECK (chart_order > 0),
	id TEXT NOT NULL,
	desc TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}',

	PRIMARY KEY (url, level, chart_order),
	FOREIGN KEY (url, level) REFERENCES difftable_level(url, level) ON DELETE CASCADE
) STRICT;
```

### `difftable_folder`

Tables are allowed to define their own "folders". Queries are
[tinyfilter](https://github.com/zkldi/tinyfilter) filters. For more details,
see the backbeat docs.

```sql
CREATE TABLE "difftable_folder" (
	url TEXT NOT NULL REFERENCES difftable(url) ON DELETE CASCADE,
	folder_order INTEGER NOT NULL CHECK (folder_order > 0),
	name TEXT NOT NULL,
	query TEXT NOT NULL,
	tags TEXT NOT NULL DEFAULT '{}',

	PRIMARY KEY (url, folder_order)
) STRICT;
```

### `bundle_fts`

```sql
CREATE VIRTUAL TABLE bundle_fts USING fts5(
	bundle_id UNINDEXED,
	description,
	tokenize = 'unicode61 remove_diacritics 2'
);
```

### `refresh`

This is a table that you can poll, to know when changes have occured in the database. After installing a chart, an asset, or some other mutation, the SDK will increment the `revision` number in this table.

The SDK provides a simple API to interface with this table, and you shouldn't need to care about it.

```sql
CREATE TABLE "refresh" (
	id INTEGER PRIMARY KEY CHECK (id = 1),
	revision INTEGER NOT NULL CHECK (revision >= 0)
) STRICT;
```
