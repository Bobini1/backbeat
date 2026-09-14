---
title: backbeat.toml specification
---

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119 (https://www.rfc-editor.org/info/rfc2119/).

## Outline

The `backbeat.toml` file is how users configure Backbeat. Users are expected to
have one user-wide backbeat "install".

On these operating systems, the backbeat config
**MUST** be at these positions.

| OS      | Default config path                                                                        |
| ------- | ------------------------------------------------------------------------------------------ |
| Linux   | `$XDG_CONFIG_HOME/backbeat/backbeat.toml` (default `$HOME/.config/backbeat/backbeat.toml`) |
| macOS   | `$HOME/.config/backbeat/backbeat.toml`                                                     |
| Windows | `%APPDATA%\backbeat\backbeat.toml`                                                         |

There is no way (by design) to configure the location of a `backbeat.toml` file. They are at a statically known place on the user's filesystem.

The actual contents of the store is configurable - by this very config - but the config itself is statically known.

The config file is **NOT** intended to be portable across operating systems,
users should not feel the need to copy this between multiple systems - however,
the backbeat store is perfectly OK to share
between systems.

It's a TOML v1.0 file, which means that all the specification of TOML 1.0 should
be applied here too.

## Full Example

```toml
[store]
# Where data is stored.
path = "/Users/zk/.local/share/backbeat"
# At what threshold downloads are inlined. The default is 128Ki and
# you likely don't need to override it!
inline = "128Ki"

[downloads]
# How many downloads you allow in parallel.
concurrency = 8
# At what threshold download are streamed to disk. The default is
# 8Mi and you likely don't need to override it!
stream = "8Mi"

[[server]]
url = "https://backbeat.example.com"

[[server]]
url = "https://fallback.example.com"

# This is an optional extra, used by the `backbeat_server` module.
# If you want to host your store as a backbeat server, you can put your information here.
[info]
name = "Example Backbeat"
contact = "ops@example.com"
```

## Config Options

The following parameters are available:

### `store.path`

Absolute path to where data should be stored:
where `backbeat.db`, `assets/` and `.downloading/`
live.

If unspecified, implementations **MUST** default to:

/// | Platform | Value | Example |
/// | -------- | ----------------------------------------- | ------------------------------------------------- |
/// | Linux | `$XDG_DATA_HOME` or `$HOME`/.local/share/backbeat | /home/alice/.local/share/backbeat |
/// | macOS | `$HOME`/.local/share/backbeat | /Users/Alice/.local/share/backbeat |
/// | Windows | `{FOLDERID_LocalAppData}\backbeat` | C:\Users\Alice\AppData\Local\backbeat |

### `store.inline`

Controls the maximum size of an asset that will be stored inline in SQLite
rather than as a separate file in `.assets/`. Assets strictly larger than this
threshold are always written to the filesystem.

This **MUST** be a byte quantity string (see [Byte quantities](#byte-quantities)).

The reference implementation defaults to `"128Ki"`.

This must be a positive value. Zero is allowed, and means that no inlining should be performed.

Implementations do not need to retroactively inline/uninline assets if this config value changes.

### `downloads.concurrency`

Maximum number of asset downloads the download manager runs in parallel across
all sources.

The reference implementation defaults to `8`.

### `downloads.stream`

Controls the maximum size of an asset that will be downloaded in-memory instead of being streamed to disk.

Assets strictly larger than this threshold are always streamed to disk.

This **MUST** be a byte quantity string (see [Byte quantities](#byte-quantities)).

The reference implementation defaults to `"32Mi"`.

### `[[server]]`

Here, users define what servers they want to fetch content from.

They are always fetched in order, so the first defined one in the config file
will be the first one that receives a request for content.

### `[[server]].url`

A url **MUST** be a string, and **MUST** be a valid URL.

Urls **MUST** have a scheme of either `http://` or `https://`.

## Byte quantities

Byte size options MUST be strings with
the following prefixes accepted.

| Suffix | Multiplier           |
| ------ | -------------------- |
| (none) | 1                    |
| `Ki`   | 1024                 |
| `Mi`   | 1024 \* 1024         |
| `Gi`   | 1024 \* 1024 \* 1024 |
| `Ti`   | 1024 \* 1024 \* 1024 |
| `k`    | 1000                 |
| `M`    | 1,000,000            |
| `G`    | 1,000,000,000        |
| `T`    | 1,000,000,000,000    |

Suffixes are NOT case-sensitive.

So the following values should parse like:

- `"123k"` = 123,000
- `"100Mi"` = 100 \* 1024 \* 1024
- `"100"` = 100

Values must not be negative. A value of `"0"` is usually allowed; consult the specific guidance for each field.

Raw integer TOML values (e.g. `inline = 16384`) **MUST NOT** be accepted.

## Additional Fields

Parsers **MUST** refuse a `backbeat.toml` file that contains unrecognised keys.
