---
title: Backbeat Collection Endpoint Spec
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Overview

Collections are arrangements of the data you can get off of a data server.

A collection endpoint is significantly easier to host than a data server, and the two are separate concepts. Someone may want to host `My Awesome Pack`, but not serve any chart or image files, and splitting the concept of data and its arrangements helps with this.

This server is intended to be implemented by a public S3 bucket or file server with no bespoke binary needed. However, implementations may want to write their own backend if hosting a file server isn't for them.

## Example Usage

A user wants to download a table for the game they're playing. They want to have some level data associated with charts in the game they're playing and be able to browse them.

They want to install `https://col.example.com/tables/foo`. The URL is arbitrary, and servers may call them whatever they want, served where-ever they want.

If they open this endpoint in their browser: `https://col.example.com/tables/foo`, implementors **SHOULD** serve a user-readable, pretty html page allowing users to see what's in the table and what the deal is.

The actual data is at `https://col.example.com/tables/foo/body.json`, and a header is at `https://col.example.com/tables/foo/header.json`

The user then selects to install `https://col.example.com/tables/foo`.

The backbeat app goes to `https://col.example.com/tables/foo/data.(bbpack, or bbcourse, or bbtable)`, and downloads the table definition, installing it.

## Specification

Collections **MUST** be accessible over either HTTP OR HTTPS.

HTTPS is **RECOMMENDED**, but not required.

Every field documented for a collection header or data object is mandatory.
Empty `tags` and `assets` maps **MUST** be written as `{}`, and empty entry or
folder lists **MUST** be written as `[]`; fields **MUST NOT** be omitted.

## ENDPOINT: `GET {path-to-collection}`

**INTENT**: Display a friendly HTML page that users can read, representing this collection.

Either this is a table, course or pack.

**THIS IS THE URL THAT USERS ADD TO THEIR CLIENTS**.

## ENDPOINT: `GET {path-to-collection}/header.json`

**INTENT**: Retrieve the last timestamp this collection was modified, so clients can know when to download updates.

Also tells you whether this is a `table`, a `course` or a `pack`.

### Example Output:

```json
{
	"timestamp": "2026-07-11T14:27:38+01:00",
	"kind": "course"
}
```

If `course`, fetch `./data.bbcourse`.

If `table`, fetch `./data.bbtable`.

If `pack`, fetch `./data.bbpack`.

## ENDPOINT `GET {path-to-collection}/data.bbtable`

A backbeat table is a mapping of charts to difficulties. See the standalone
[Backbeat table specification](./backbeat-table.md) for field definitions and
authoring guidance.

### Example Output:

Comments are documentational commentary, and no comments are to be returned by the endpoint.

```json
{
	// The name of this table.
	"name": "発狂BMS難易度表",
	// The symbol to prefix levels with.
	"symbol": "★",
	// A valid-gamemode string indicating what gamemode this table is for
	// Gamemode strings must be a-z0-9 lowercase, with hyphens.
	//
	// There is no central authority on gamemode names, nor is it baked into
	// the spec, however, general convention helps clients filter out tables
	// that aren't relevant to them.
	"gamemode": "bms-7k",
	// Additional arbitrary key -> value pairs. Clients may read this and
	// do things based on them.
	"tags": {
		"foo": "bar",
		"banner-image": "banner.png"
	},
	// when this table was last updated.
	"updated": "2017-02-05T00:00:00.000Z",
	// Assets this table references. Use an empty object when there are none.
	"assets": {
		"banner.png": "a00a9e7f8bc3d6d6c061926f2fc9fd80d818935bcda54a8f8c5f686b9d02b7b0"
	},
	// Levels are the core bit of a table, this is where charts are assigned to things.
	"levels": [
		{
			// Levels are strings. You can put whatever you want in here.
			"level": "1",
			// again, arbitrary key value pairs that a client may do whatever it wants
			// with.
			"tags": {
				"color": "red"
			},
			"charts": [
				{
					"desc": "Grand Thaw - ERIS MX",
					"id": "md5/0108ff92119d48e0ca0e2eae1f3dffbf",
					"tags": {
						"scratch": "yes"
					}
				},
				{
					"desc": "ねこみみ魔法使い - 恋わずらい -shiny party floor- maniac",
					"id": "md5/084a1c763265e85e66a9b782cc6464e5",
					"tags": {}
				}
			]
		}
	],
	// Tables may define "extra" folders using the tinyfilter filtering language.
	//
	// This is most useful for defining skillset subfolders, or marathon charts only
	// et cetera.
	"folders": [
		{
			"name": "Level 1 scratch",
			"query": "level == \"1\" and tags.scratch == \"yes\"",
			"tags": {}
		}
	]
}
```

## ENDPOINT `GET {path-to-collection}/data.bbcourse`

A backbeat course is a set of charts played in order. See the standalone
[Backbeat course specification](./backbeat-course.md) for field definitions
and authoring guidance.

### Example Output:

Comments are documentational commentary, and no comments are to be returned by the endpoint.

```json
{
	"name": "zzzzzzzzzz",
	"gamemode": "bms-7k",
	// Arbitrary key -> value pairs to be interpreted by your game client.
	"tags": {
		"bms/gauge": "lr2"
	},
	"updated": "2026-09-05T15:04:31.809Z",
	"assets": {
		"banner.png": "a00a9e7f8bc3d6d6c061926f2fc9fd80d818935bcda54a8f8c5f686b9d02b7b0"
	},
	"charts": [
		{
			// Purely user-friendly name for this chart in case they don't have it installed.
			"desc": "SHIKI / black train / DDX - Air -BLAST- [Long]",
			// The ChartID for this chart. Note that courses are comprised of chart IDs, and not bundle IDs.
			"id": "md5/6d64d463466a08eab2fa0768a298bb58",
			"tags": {}
		},
		{
			"desc": "zest:rave - H.S : Hadron Strike [7Keys - EX]",
			"id": "md5/940d69a59ed86f27a9848c0d8c29fec7",
			"tags": {}
		},
		{
			"desc": "姉ヶ崎 寧々 BGA : photonskyto/obj:mfmf - ZENITHALIZE [SP Love+]",
			"id": "md5/99ec056cff4c2ba72664e78c87cb0354",
			"tags": {}
		},
		{
			"desc": "哀愁マニア - the lost dedicated [life]",
			"id": "md5/8f2a82e4f1f3e299c5d761a4c673b9ae",
			"tags": {}
		},
		{
			"desc": "xi / air - FREEDOM DiVE [FOUR DIMENSIONS]",
			"id": "md5/c46a81cb184f5a804c119930d6eba748",
			"tags": {}
		}
	]
}
```

## ENDPOINT `GET {path-to-collection}/data.bbpack`

A backbeat pack is an ordered set of exact bundles. See the standalone
[Backbeat pack specification](./backbeat-pack.md) for field definitions and
authoring guidance.

### Example Output

A pack is a set of bundles.

```json
{
	"name": "Scintill Tinypack",
	"gamemode": "sm-4k",
	"tags": {
		"banner": "2b62595b31d45b22a8ed1f87ec29ce29879bdd1e2430b795b728f6cd333c70aa"
	},
	"updated": "2026-08-12T08:21:44.682Z",
	"assets": {
		"UC-banner.png": "2b62595b31d45b22a8ed1f87ec29ce29879bdd1e2430b795b728f6cd333c70aa"
	},
	"bundles": [
		{
			"id": "b-0ce1f2c355f96b2a5cf126484a744b76a37bb0a918bc6526d7c054dc405a8cff",
			"desc": "Undead Corporation - Put Curse of You (Challenge 17)",
			"tags": {
				"sm/song-folder": "Put Curse on You"
			}
		},
		{
			"id": "b-6d41af1417de9ae1a227fc14c2218c2d48f1ad07bf153b84f7c20116ce602de2",
			"desc": "Undead Corporation - Chain Heart Girl (Challenge 12)",
			"tags": {
				"sm/song-folder": "Chain Heart Girl"
			}
		},
		{
			"id": "b-71f509b3a6e4bf156ac3bd79175a7dffcf3157034c9fcddaea53bcd713c29401",
			"desc": "Undead Corporation - Magus Night Fever (Challenge 15)",
			"tags": {
				"sm/song-folder": "Magus Night Fever"
			}
		},
		{
			"id": "b-fd510480bf5ebb272998c8541636e2743b18505d122a8f431f3cd7630c2235de",
			"desc": "Undead Corporation - Heian no Burasuto (Challenge 16)",
			"tags": {
				"sm/song-folder": "Heian no Burasuto"
			}
		}
	]
}
```
