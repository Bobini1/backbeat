//! Parsing of Clone Hero `.chart` files.
//!
//! `.chart` is a text-based format with INI-like `[Section] { ... }` blocks.
//! The `[Song]` section holds song metadata, `[SyncTrack]` holds tempo and
//! time-signature events, `[Events]` holds global text events (section markers,
//! lyrics), and every remaining section describes a single instrument+difficulty
//! chart (e.g. `[ExpertSingle]`, `[HardDrums]`).
//!
//! ## Encoding
//!
//! `.chart` files are UTF-8 text. [`from_bytes`] returns an error if the input
//! cannot be decoded as UTF-8.
//!
//! ## Section format
//!
//! ```text
//! [SectionName]
//! {
//!   key = value
//!   tick = TYPE arg1 [arg2]
//! }
//! ```
//!
//! ## Timeline event types
//!
//! | Code | Sections          | Meaning                                          |
//! |------|-------------------|--------------------------------------------------|
//! | `B`  | `SyncTrack`       | BPM in milli-BPM (e.g. `120000` → 120.000 BPM)  |
//! | `TS` | `SyncTrack`       | Time signature: `numerator [denominator_exp]`     |
//! | `A`  | `SyncTrack`       | Tempo anchor (μs). Informational only.            |
//! | `N`  | track sections    | Note: `fret sustain_ticks`                       |
//! | `S`  | track sections    | Special phrase: `kind length_ticks`              |
//! | `E`  | any section       | Quoted text event                                 |
//!
//! ## Track naming
//!
//! Track section names combine difficulty and instrument in one word,
//! e.g. `ExpertSingle` (Expert Guitar), `HardDrums`, `MediumGHLBass`.

use std::io;
use std::path::Path;

/// Error returned by [`from_bytes`] when the input is not valid UTF-8.
#[derive(Debug, thiserror::Error)]
#[error("invalid UTF-8 in .chart file: {0}")]
pub struct ParseError(#[from] std::str::Utf8Error);

/// Metadata extracted from the `[Song]` section.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
	/// Song title (`Name` key).
	pub name: Option<String>,
	/// Song artist.
	pub artist: Option<String>,
	/// Album name.
	pub album: Option<String>,
	/// Year string as stored in the file. Often prefixed with `", "` (e.g. `", 2021"`).
	pub year: Option<String>,
	/// Charter / mapper name.
	pub charter: Option<String>,
	/// Ticks per quarter note. Defaults to 192 when absent.
	pub resolution: u32,
	/// Global audio offset in seconds.
	pub offset: f64,
	/// Explicit background image filename from `background =` key.
	pub background: Option<String>,
	/// Explicit background video filename from `video =` key.
	pub video: Option<String>,
	/// Explicit cover/album-art filename from `cover =` key.
	pub cover: Option<String>,
}

/// A single timing event from the `[SyncTrack]` section.
#[derive(Debug, Clone)]
pub struct SyncEvent {
	/// Position in ticks from the start of the chart.
	pub tick: u32,
	/// The kind of sync event.
	pub kind: SyncEventKind,
}

/// The variant of a [`SyncEvent`].
#[derive(Debug, Clone)]
pub enum SyncEventKind {
	/// BPM change. Value is in milli-BPM; divide by 1000 for actual BPM.
	Bpm(u64),
	/// Time-signature change.
	TimeSignature {
		/// Top number of the time signature.
		numerator: u32,
		/// Denominator as a power of 2. Absent in the file means 2 (→ `4` bottom number).
		denominator_exp: u32,
	},
	/// Tempo anchor in microseconds. Not used for playback; preserved verbatim.
	Anchor(u64),
}

/// A text event from the `[Events]` section.
#[derive(Debug, Clone)]
pub struct TextEvent {
	/// Position in ticks.
	pub tick: u32,
	/// Event text content (without surrounding quotes).
	pub text: String,
}

/// A single event inside an instrument track section.
#[derive(Debug, Clone)]
pub struct TrackEvent {
	/// Position in ticks.
	pub tick: u32,
	/// The kind of track event.
	pub kind: TrackEventKind,
}

/// The variant of a [`TrackEvent`].
#[derive(Debug, Clone)]
pub enum TrackEventKind {
	/// A note. `fret` encodes the lane:
	/// 0–4 = green/red/yellow/blue/orange, 5 = forced HOPO, 6 = tap, 7 = open note.
	Note {
		/// Lane index.
		fret: u8,
		/// Sustain length in ticks (0 for tap notes).
		sustain: u32,
	},
	/// A special phrase. Common kinds: 2 = Star Power, 8 = Solo.
	Special {
		/// Phrase type identifier.
		kind: u8,
		/// Phrase length in ticks.
		length: u32,
	},
	/// A text event embedded within a track section.
	Event(String),
}

/// One instrument+difficulty track section (e.g. `[ExpertSingle]`).
#[derive(Debug, Clone)]
pub struct Track {
	/// Raw section name as it appears in the file, e.g. `"ExpertSingle"`.
	pub name: String,
	/// Parsed events within this track.
	pub events: Vec<TrackEvent>,
	pub(crate) raw_body: String,
}

/// A fully parsed `.chart` file.
#[derive(Debug, Clone)]
pub struct Chart {
	/// Song-level metadata from the `[Song]` section.
	pub metadata: Metadata,
	/// Timing events from the `[SyncTrack]` section.
	pub sync_track: Vec<SyncEvent>,
	/// Global text events from the `[Events]` section.
	pub events: Vec<TextEvent>,
	/// Instrument+difficulty track sections.
	pub tracks: Vec<Track>,
	raw_song_body: String,
	raw_sync_body: String,
	raw_events_body: String,
}

impl Chart {
	/// Produce the bytes of a single-track fractured `.chart` file.
	///
	/// The result is a valid `.chart` containing the global `[Song]`,
	/// `[SyncTrack]`, and `[Events]` sections followed by exactly one
	/// instrument track section.
	pub fn fractured_bytes(&self, track: &Track) -> Vec<u8> {
		let cap = self.raw_song_body.len()
			+ self.raw_sync_body.len()
			+ self.raw_events_body.len()
			+ track.raw_body.len()
			+ 128;
		let mut out = String::with_capacity(cap);
		push_section(&mut out, "Song", &self.raw_song_body);
		push_section(&mut out, "SyncTrack", &self.raw_sync_body);
		push_section(&mut out, "Events", &self.raw_events_body);
		push_section(&mut out, &track.name, &track.raw_body);
		out.into_bytes()
	}
}

fn push_section(out: &mut String, name: &str, body: &str) {
	out.push('[');
	out.push_str(name);
	out.push_str("]\n{\n");
	out.push_str(body);
	out.push_str("}\n");
}

/// Parse a `.chart` file from its raw bytes.
///
/// Returns [`ParseError`] if the bytes are not valid UTF-8. Unknown sections
/// and unrecognised event codes within known sections are silently skipped.
///
/// A leading UTF-8 BOM (`EF BB BF`) is stripped automatically.
pub fn from_bytes(bytes: &[u8]) -> Result<Chart, ParseError> {
	// Some tools (e.g. YARG.Core's test suite) emit a UTF-8 BOM. Strip it so
	// that the first section header is not prefixed with U+FEFF.
	let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
	let text = std::str::from_utf8(bytes)?;
	Ok(parse_text(text))
}

/// Read and parse a `.chart` file from disk.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Result<Chart, ParseError>> {
	let bytes = std::fs::read(path)?;
	Ok(from_bytes(&bytes))
}

fn parse_text(text: &str) -> Chart {
	let mut metadata = Metadata {
		resolution: 192,
		..Default::default()
	};
	let mut sync_track = Vec::new();
	let mut events = Vec::new();
	let mut tracks = Vec::new();
	let mut raw_song_body = String::new();
	let mut raw_sync_body = String::new();
	let mut raw_events_body = String::new();

	let mut lines = text.lines().peekable();
	while let Some(line) = lines.next() {
		let trimmed = line.trim();

		// Section headers look like [Name]
		if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
			continue;
		}
		let name = &trimmed[1..trimmed.len() - 1];

		// Skip blank lines before the opening brace.
		while lines.peek().map(|l| l.trim().is_empty()).unwrap_or(false) {
			lines.next();
		}
		// Expect `{` on the next non-blank line.
		if lines.peek().map(|l| l.trim()) != Some("{") {
			continue;
		}
		lines.next(); // consume `{`

		// Collect body lines until `}`.
		let mut body_lines: Vec<&str> = Vec::new();
		for body_line in lines.by_ref() {
			if body_line.trim() == "}" {
				break;
			}
			body_lines.push(body_line);
		}
		// Preserve trailing newline so re-serialized bodies are formatted
		// identically to the original.
		let body = if body_lines.is_empty() {
			String::new()
		} else {
			body_lines.join("\n") + "\n"
		};

		match name {
			"Song" => {
				parse_song_section(&body, &mut metadata);
				raw_song_body = body;
			}
			"SyncTrack" => {
				sync_track = parse_sync_section(&body);
				raw_sync_body = body;
			}
			"Events" => {
				events = parse_events_section(&body);
				raw_events_body = body;
			}
			_ => {
				// Everything else is an instrument track.
				let track_events = parse_track_section(&body);
				tracks.push(Track {
					name: name.to_owned(),
					events: track_events,
					raw_body: body,
				});
			}
		}
	}

	Chart {
		metadata,
		sync_track,
		events,
		tracks,
		raw_song_body,
		raw_sync_body,
		raw_events_body,
	}
}

// ── Section parsers ──────────────────────────────────────────────────────────

fn parse_song_section(body: &str, meta: &mut Metadata) {
	for line in body.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		let Some((key, val)) = line.split_once('=') else {
			continue;
		};
		let key = key.trim();
		let val = val.trim().trim_matches('"');
		match key {
			"Name" => meta.name = Some(val.to_owned()),
			"Artist" => meta.artist = Some(val.to_owned()),
			"Album" => meta.album = Some(val.to_owned()),
			"Year" => {
				// Years are often stored as `", 2021"` — strip the leading `, `.
				meta.year = Some(val.trim_start_matches(", ").to_owned());
			}
			"Charter" => meta.charter = Some(val.to_owned()),
			"Resolution" => {
				if let Ok(r) = val.parse::<u32>() {
					meta.resolution = r;
				}
			}
			"Offset" => {
				if let Ok(o) = val.parse::<f64>() {
					meta.offset = o;
				}
			}
			// Explicit asset overrides (usually from song.ini but may appear here).
			"background" => meta.background = Some(val.to_owned()),
			"video" => meta.video = Some(val.to_owned()),
			"cover" => meta.cover = Some(val.to_owned()),
			_ => {}
		}
	}
}

fn parse_sync_section(body: &str) -> Vec<SyncEvent> {
	let mut out = Vec::new();
	for line in body.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		let Some((tick_str, rest)) = line.split_once('=') else {
			continue;
		};
		let Ok(tick) = tick_str.trim().parse::<u32>() else {
			continue;
		};
		let rest = rest.trim();
		let mut parts = rest.split_ascii_whitespace();
		let Some(code) = parts.next() else { continue };
		let kind = match code {
			"B" => {
				let Ok(val) = parts.next().unwrap_or("").parse::<u64>() else {
					continue;
				};
				SyncEventKind::Bpm(val)
			}
			"TS" => {
				let Ok(num) = parts.next().unwrap_or("").parse::<u32>() else {
					continue;
				};
				let denom_exp = parts
					.next()
					.and_then(|s| s.parse::<u32>().ok())
					.unwrap_or(2);
				SyncEventKind::TimeSignature {
					numerator: num,
					denominator_exp: denom_exp,
				}
			}
			"A" => {
				let Ok(val) = parts.next().unwrap_or("").parse::<u64>() else {
					continue;
				};
				SyncEventKind::Anchor(val)
			}
			_ => continue,
		};
		out.push(SyncEvent { tick, kind });
	}
	out
}

fn parse_events_section(body: &str) -> Vec<TextEvent> {
	let mut out = Vec::new();
	for line in body.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		let Some((tick_str, rest)) = line.split_once('=') else {
			continue;
		};
		let Ok(tick) = tick_str.trim().parse::<u32>() else {
			continue;
		};
		let rest = rest.trim();
		// Event lines start with `E `
		let text_part = match rest.strip_prefix("E ") {
			Some(t) => t.trim().trim_matches('"'),
			None if rest == "E" => "",
			None => continue,
		};
		out.push(TextEvent {
			tick,
			text: text_part.to_owned(),
		});
	}
	out
}

fn parse_track_section(body: &str) -> Vec<TrackEvent> {
	let mut out = Vec::new();
	for line in body.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		let Some((tick_str, rest)) = line.split_once('=') else {
			continue;
		};
		let Ok(tick) = tick_str.trim().parse::<u32>() else {
			continue;
		};
		let rest = rest.trim();
		let mut parts = rest.split_ascii_whitespace();
		let Some(code) = parts.next() else { continue };
		let kind = match code {
			"N" => {
				let Ok(fret) = parts.next().unwrap_or("").parse::<u8>() else {
					continue;
				};
				let sustain = parts
					.next()
					.and_then(|s| s.parse::<u32>().ok())
					.unwrap_or(0);
				TrackEventKind::Note { fret, sustain }
			}
			"S" => {
				let Ok(kind) = parts.next().unwrap_or("").parse::<u8>() else {
					continue;
				};
				let length = parts
					.next()
					.and_then(|s| s.parse::<u32>().ok())
					.unwrap_or(0);
				TrackEventKind::Special { kind, length }
			}
			"E" => {
				// `E "text"` or `E text` — the rest of the line after `E `.
				let text_part = rest[1..].trim().trim_matches('"');
				TrackEventKind::Event(text_part.to_owned())
			}
			_ => continue,
		};
		out.push(TrackEvent { tick, kind });
	}
	out
}
