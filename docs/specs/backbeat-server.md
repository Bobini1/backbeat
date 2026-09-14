---
title: Backbeat Server Spec
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Overview

The point of the Backbeat Data Server (henceforth BDS) is to serve asset and `.bb` files.
Backbeat is designed such that pieces of data are identified by checksums and can be verified
by clients. This allows anyone to spin up a data server -- clients are able to verify that
they are being served the correct data.

The BDS specification describes a series of HTTP endpoints, intended to be implementable in any language with minimal code.

An implementation for hosting a Backbeat Store is shipped with the cli, under `bkb start-data-server`.

The data server is the complement to the [Backbeat Collection Endpoint](./backbeat-collection-endpoint.md). This server serves charts and assets, while a collection endpoint serves ways of arranging that data meaningfully (i.e. packs, tables, courses).

## Example Usage

A user is linked to a chart and wishes to have it in their game.

This chart has the `sha256` of `4e955fea0268518cbaa500409dfbec88f0ecebad28d84ecbe250baed97dba889`.

They have one data server registered, at `https://data.example.com`

Their client makes a request to `https://data.example.com/charts/sha256/4e955fea0268518cbaa500409dfbec88f0ecebad28d84ecbe250baed97dba889`.
This request returns a status code of 200, and a response body containing a `.bb` file.

The `.bb` file they get will look like this:

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
	"desc": "nao.paradigm - 0x1311 (Hard)",
	"chart": "H4sIAHJMRGoCA8vI5AIAenpv7QMAAAA="
}
```

A client will go on to request:

- `https://content.example.com/assets/db5a1a5b0cdabec3e45d4c3613276f28c141d612cd7008560efc741c04cfb99f`
- `https://content.example.com/assets/6098ac7e3b056e5a85c14f8914dd6a8fd25a661b715904c9b57409c8ab829585`

... and so on.

Their client can then proceed to install these files into their game, resolving the missing content.

## Specification

A BDS is built ontop of HTTP.
It **MUST** be accessible over either HTTP OR HTTPS.

HTTPS is **RECOMMENDED**, but not required.

## CONCEPT: `{assetHash}`

Backbeat uses SHA256 for all of its asset checksums. That is to say, song and image files are not identified by their filenames, instead, they are referred to by the SHA256 checksum of their contents.

When present in a url, this is read as a parameter. Asset Hashes **MUST** be interpreted as follows:

- They **MUST** be 64 characters long.
- They **MUST** be comprised of the characters `0123456789abcdef`.
- Implementations **MUST NOT** respond to uppercase checksums, or checksums in any other format.
- These restrictions apply to asset hashes. They do **NOT** apply to chart ID algorithms, below.

## CONCEPT: `{bundleId}`

A `.bb` file is deterministically identified by its `bundleId`.
This is an algorithm that combines a chart, the dependencies specified, and the filename.

A `bundleId` looks like this: `b-d1bc8d3ba4afc7e109612cb73acbdddac052c93025aa1f82942edabb7deb82a1`

A `bundleId` **MUST** conform to the `Bundle ID` string form specified in the [bundle ID specification](./bundle-id.md):

- It **MUST** be the ASCII prefix `b-` followed by a 64-character lowercase hex `sha256` (66 characters total).
- The hex portion **MUST** be comprised of the characters `0123456789abcdef`.
- Implementations **MUST NOT** respond to uppercase, a missing `b-` prefix, or any other format.

## CONCEPT: `{combinedAssetsId}`

TODO

## CONCEPT: `{idAlgorithm}`

Charts are identified in many ways by many games, and backbeat generally tries to support them
so that clients can resolve charts by their preferred method.

Implementations **MUST** evaluate `sha256(chart_content)` and provide it as an ID algorithm for all charts it knows of.

Some ID algorithms are just hashes of the file, and some can be "canonicalisations" of the file,
where metadata that doesn't affect the game. It is perfectly reasonable for different charts to
share IDs. For example, changing the song title in a chart will not change its `etterna-chartkey`,
but it will change its `sha256`.

An ID algorithm name **MUST** be be comprised of all lowercase a-z characters, numbers and hyphens.

An ID value **MUST NOT** contain URL special characters, (`/`, `?`, `#`).

Other examples may include:

- `md5`, which is available on `.bms` and its ilk

## ENDPOINT: `GET /backbeat/up`

**INTENT:** This can be used by clients to check if a BDS is up and healthy. It can also be used as a healthcheck in other contexts.

If this BDS is available and accessible by the user, it **MUST** respond to `/backbeat/up` with a 200 OK.

A BDS that does not respond to `/backbeat/up` with a 200 will be deemed to be offline or unavailable.
If a non-200 status code is returned, clients **SHOULD** use the returned HTTP Status Code to
display information to the user.

For example, if the returned status code is `503` instead, a game client can display `Service Unavailable` to the user.

If the response is 200 OK, the body returned from this request is irrelevant and **MUST NOT**
be processed by clients. It is **RECOMMENDED** to return an empty body here for performance reasons.

## ENDPOINT: `GET /backbeat/info`

**INTENT:** A client may want to get some information about your server, like a display name.

A BDS **MUST** respond to `/backbeat/info` with a valid JSON document.

This document **MUST** be a JSON object. The keys in this object are as follows:

### INFO JSON FIELD: `"name"`

This is the name of the server, and is displayed to users.
This **MUST** be present and a string.
It **SHOULD** be a human readable piece of information.

### INFO JSON FIELD: `"contact"`

This is a human-readable piece of contact information for the host of this server.
If present, this **MUST** be a string.
This field is optional and **MAY** not be present.

### Example `/backbeat/info`

```json
{
	"name": "My Server",
	"contact": "I can be reached at zk <at> Backbeat <dot> ac, or on github at zkldi."
}
```

## ENDPOINT: `GET /assets/{assetHash}`

**INTENT:** This endpoint should return a file that checksums to this sha256 checksum. If the server doesn't have it, the server **MUST** return 404.

Clients **MUST** check that the information they've received aligns with what they've
requested. That is to say:

If you download `/assets/87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`, the downloaded image **SHOULD** have a sha256 of `87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`.
The client **SHOULD** verify this -- and **SHOULD NOT** blindly trust the server.

On success this endpoint:

- **MUST** return a status code of 200.
- **MUST** have a HTTP header of `Content-Length` with the correct content length.
- **MUST** return a file that sha256 hashes to the requested hash.
- **MAY** return a status code of 400 and no data if the sha256 was unparsable; that it did not match `^[a-f0-9]{64}$`.
- **MAY** have a HTTP header of `Content-Type` to indicate what kind of file it is.
- **MAY** add a `Content-Disposition` header to enable automatic downloading in browsers.

This endpoint **MUST** return 404 if the requested asset hash is not found on the server.

## ENDPOINT: `HEAD /assets/{assetHash}`

**INTENT:** A client should use this endpoint to ask a client if an asset is available without fetching it.

Servers **MUST** return a status code of `200` if this asset hash can be found on this server.

Servers **MUST** return a status code of `404` if this asset hash can NOT be found on this server.

Servers **MAY** return a status code of 400 and no data if the sha256 was unparsable; that it did not match `^[a-f0-9]{64}$`.

## ENDPOINT: `GET /bundles/{bundleId}`

**INTENT:** This endpoint should return the `.bb` file whose `bundleId` matches exactly. Unlike
`/charts/{idAlgorithm}/{idValue}`, this endpoint is never ambiguous: a `{bundleId}` fully determines
the chart bytes, path, and dependency set, so there is exactly one possible response.

Clients **MUST** check that the information they've received aligns with what they've requested.
That is to say: clients **SHOULD** recompute the `bundleId` of the returned `.bb` file and verify
it matches the requested `{bundleId}` -- and **SHOULD NOT** blindly trust the server.

On success this endpoint:

- **MUST** return a status code of 200.
- **MUST** have a HTTP header of `Content-Length` with the correct content length.
- **MUST** return a `.bb` file whose `bundleId` is the requested `{bundleId}`.
- **MAY** return a status code of `400` if this is an invalid `{bundleId}`, i.e. it is not in the form `b-[a-f0-9]{64}`.
- **MAY** have a HTTP header of `Content-Type` with value `application/json`.

This endpoint **MUST** return 404 if the requested `{bundleId}` is not found on the server.

## ENDPOINT: `HEAD /bundles/{bundleId}`

**INTENT:** A client should use this endpoint to ask a server if a bundle is available without fetching it.

Servers **MUST** return a status code of `200` if this `{bundleId}` can be found on this server.

Servers **MUST** return a status code of `404` if this `{bundleId}` can NOT be found on this server.

Servers **MAY** return a status code of `400` if this is an invalid `{bundleId}`, i.e. it is not in the form `b-[a-f0-9]{64}`.

## ENDPOINT: `GET /charts/{idAlgorithm}/{idValue}`

**INTENT:** A client should use this endpoint to fetch a `.bb` file.

Clients **MUST** check that the information they've received aligns with what they've
requested. That is to say:

If you download `/charts/sha256/87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`, the downloaded chart content **MUST** have a `sha256` of `87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`.
The client **SHOULD** verify this -- and can not blindly trust the server.

On success this endpoint:

- **MUST** return a status code of 200.
- **MUST** have a HTTP header of `Content-Length` with the correct content length.
- **MUST** return a chart that has an `idAlgorithm` of `idValue`.
- **SHOULD** have a HTTP header of `Content-Type` with value `application/json`.
- **MAY** add a `Content-Disposition` header to enable automatic downloading in browsers.

On failure this endpoint:

- **MUST** return a 404 if the requested `idAlgorithm` is not recognised by the server.
- **MUST** return 404 if the requested `idValue` is not found on the server and the
  `idAlgorithm` is recognised.

### Dealing with ambigious IDs

It is possible for your backbeat client to have multiple possible charts for a given algorithm+value,
as some algorithms undergo "canonicalisation", or just generally have collisions (e.g. md5), or
the chart appears in multiple places, and therefore has multiple packages.

You **MUST** choose one of the possible bundles to return; which one you pick is implementation
defined. No guarantees can be given - if a client wants a very specific packaging of the chart, they should download the bundle-id directly.

## ENDPOINT: `HEAD /charts/{idAlgorithm}/{idValue}`

**INTENT:** A client should use this endpoint to ask a client if a chart is available without fetching it.

Servers **MUST** return a status code of `200` if this `{idAlgorithm}/{idValue}` can be found on this server.

Servers **MUST** return a status code of `404` is the `idAlgorithm` requested is not recognised by the server.

Servers **MUST** return a status code of `404` if this `{idAlgorithm}/{idValue}` can NOT be found on this server AND the `idAlgorithm` is recognised by the server.

## ENDPOINT: `HEAD /precombined-assets/{combinedAssetsId}`

**INTENT:** A client should use this endpoint to ask if precombined assets are available without fetching it.

Servers **MUST** return a status code of `200` if this `{combinedAssetsId}` can be found on this server.

Servers **MUST** return a status code of `404` if this `{combinedAssetsId}` can NOT be found on this server.

## ENDPOINT: `GET /precombined-assets/{combinedAssetsId}`

**INTENT:** This is an optimisation. Servers may choose to serve combined assets, which prevents downloads of lots of small files. Clients may choose to fetch this is a bundle carries lots of assets.

Servers **MUST** return a status code of `404` if this `{combinedAssetsId}` can NOT be found on this server.

If these precombined assets can be found on this server, a server **MUST** return a status code of `200` and return a precombined-assets file.

The client **SHOULD** verify that the combined assets file returned has the `{combinedAssetsId}` they requested -- and **SHOULD NOT** blindly trust the server.

The returned precombined-assets file should be a valid `.tar.gz` file containing `asset filename -> asset file`. contents.
For more details, see the precombined-assets spec.
