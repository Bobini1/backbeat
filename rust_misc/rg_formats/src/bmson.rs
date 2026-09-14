//! Parser for the `.bmson` file format.
//!
//! BMSON is a JSON-based rhythm game chart format (version 1.0.0). Unlike BMS,
//! one `.bmson` file represents exactly one chart (no fracturing needed).
//! Sound assets are referenced via `sound_channels[*].name` with slicing
//! semantics; BGA images/videos live in `bga.bga_header[*].name`.
//!
//! # Usage
//!
//! ```rust,ignore
//! let chart = rg_formats::bmson::from_file("song.bmson")
//!     .expect("failed to read file")
//!     .expect("invalid bmson");
//! ```
//!
//! # References
//!
//! - [BMSON 1.0.0 spec](https://bmson-spec.readthedocs.io/en/master/doc/index.html)
//! - [jbms-parser](https://github.com/exch-bms2/jbms-parser) — beatoraja's BMSON decoder

use std::io;
use std::path::Path;

use serde::Deserialize;

// ── Default helpers for serde ─────────────────────────────────────────────────

fn default_mode_hint() -> String {
	"beat-7k".to_owned()
}

fn default_judge_rank() -> f64 {
	100.0
}

fn default_total() -> f64 {
	100.0
}

fn default_resolution() -> u64 {
	240
}

// ── Top-level object ──────────────────────────────────────────────────────────

/// A parsed `.bmson` file.
#[derive(Debug, Clone, Deserialize)]
pub struct Bmson {
	/// bmson format version string (e.g. `"1.0.0"`). Present in v1.0.0+; absent
	/// in legacy v0.21 files. Never used for version-specific branching.
	pub version: Option<String>,

	/// Song/chart metadata.
	pub info: BmsonInfo,

	/// Bar-line positions (pulses). `null`/absent → 4/4 time assumed.
	#[serde(default)]
	pub lines: Vec<BarLine>,

	/// Tempo change events.
	#[serde(default)]
	pub bpm_events: Vec<BpmEvent>,

	/// Stop (scroll pause) events.
	#[serde(default)]
	pub stop_events: Vec<StopEvent>,

	/// Scroll-speed change events (beatoraja extension).
	#[serde(default)]
	pub scroll_events: Vec<ScrollEvent>,

	/// Keysound channels — each entry names an audio file and lists notes that
	/// trigger (slices of) that file.
	#[serde(default)]
	pub sound_channels: Vec<SoundChannel>,

	/// Hidden keysound channels — sound files without visible notes.
	/// Present in beatoraja-authored BMSON files.
	#[serde(default)]
	pub key_channels: Vec<SoundChannel>,

	/// Mine/landmine channels — sound files for mine notes.
	/// Present in beatoraja-authored BMSON files.
	#[serde(default)]
	pub mine_channels: Vec<SoundChannel>,

	/// Background animation data.
	pub bga: Option<Bga>,
}

// ── Info ──────────────────────────────────────────────────────────────────────

/// Song-level metadata (`info` object inside a [`Bmson`]).
#[derive(Debug, Clone, Deserialize)]
pub struct BmsonInfo {
	/// Song title.
	#[serde(default)]
	pub title: String,

	/// Song subtitle. Default `""`.
	#[serde(default)]
	pub subtitle: String,

	/// Primary artist.
	#[serde(default)]
	pub artist: String,

	/// Additional contributing artists (array of `"key:value"` strings).
	#[serde(default)]
	pub subartists: Vec<String>,

	/// Genre.
	#[serde(default)]
	pub genre: String,

	/// Game-mode hint (e.g. `"beat-7k"`, `"popn-9k"`). Default `"beat-7k"`.
	#[serde(default = "default_mode_hint")]
	pub mode_hint: String,

	/// Difficulty/chart name (e.g. `"HYPER"`, `"ANOTHER"`).
	#[serde(default)]
	pub chart_name: String,

	/// Numeric difficulty level.
	#[serde(default)]
	pub level: u64,

	/// Starting BPM. Required by the spec.
	pub init_bpm: f64,

	/// Judgment-window width relative to 100% default. Default `100.0`.
	#[serde(default = "default_judge_rank")]
	pub judge_rank: f64,

	/// Lifebar gain rate relative to 100% default. Default `100.0`.
	#[serde(default = "default_total")]
	pub total: f64,

	/// Static background image filename (exact path — no extension fallback).
	pub back_image: Option<String>,

	/// Eyecatch/loading-screen image filename (exact path).
	pub eyecatch_image: Option<String>,

	/// Title image filename, displayed before song starts (exact path).
	pub title_image: Option<String>,

	/// Banner image filename (exact path).
	pub banner_image: Option<String>,

	/// Short audio preview filename (extension fallback applies).
	pub preview_music: Option<String>,

	/// Pulses per quarter note in 4/4 time. Default `240`.
	#[serde(default = "default_resolution")]
	pub resolution: u64,

	/// Long-note type (1–3). Beatoraja extension; absent in vanilla BMSON.
	pub ln_type: Option<u64>,
}

// ── Sound channels ────────────────────────────────────────────────────────────

/// A named audio track with its associated notes.
#[derive(Debug, Clone, Deserialize)]
pub struct SoundChannel {
	/// Relative path to the audio file. Extension fallback applies at runtime.
	pub name: String,

	/// Notes that trigger (slices of) this sound channel.
	#[serde(default)]
	pub notes: Vec<Note>,
}

/// A note inside a [`SoundChannel`].
#[derive(Debug, Clone, Deserialize)]
pub struct Note {
	/// Lane (1-based). `0` or absent → BGM note (not player-visible).
	/// Per beatoraja: playable iff `x > 0 && x <= mode.key_count`.
	#[serde(default)]
	pub x: i64,

	/// Time of the note in pulses.
	pub y: u64,

	/// Length in pulses. `0` = tap note; `> 0` = long note ending at `y + l`.
	#[serde(default)]
	pub l: u64,

	/// Continuation flag. `true` = do not restart the audio at this note.
	#[serde(default)]
	pub c: bool,
}

// ── BGA ───────────────────────────────────────────────────────────────────────

/// Background animation data.
#[derive(Debug, Clone, Deserialize)]
pub struct Bga {
	/// Image/video file declarations.
	#[serde(default)]
	pub bga_header: Vec<BgaHeader>,

	/// Main BGA event sequence.
	#[serde(default)]
	pub bga_events: Vec<BgaEvent>,

	/// Layer events (overlaid on top of main BGA).
	#[serde(default)]
	pub layer_events: Vec<BgaEvent>,

	/// Events shown on miss.
	#[serde(default)]
	pub poor_events: Vec<BgaEvent>,
}

/// A picture/video file referenced by BGA events.
#[derive(Debug, Clone, Deserialize)]
pub struct BgaHeader {
	/// Unique integer identifier.
	pub id: u64,

	/// Relative path to the image or video file.
	pub name: String,
}

/// A timed BGA event (maps a pulse position to a [`BgaHeader`] ID).
#[derive(Debug, Clone, Deserialize)]
pub struct BgaEvent {
	/// Pulse position.
	pub y: u64,

	/// ID referencing a [`BgaHeader`].
	pub id: u64,
}

// ── Timing events ─────────────────────────────────────────────────────────────

/// A bar-line position.
#[derive(Debug, Clone, Deserialize)]
pub struct BarLine {
	/// Pulse position of this bar line.
	pub y: u64,
}

/// A BPM change event.
#[derive(Debug, Clone, Deserialize)]
pub struct BpmEvent {
	/// Pulse position.
	pub y: u64,
	/// New BPM value.
	pub bpm: f64,
}

/// A stop (scroll pause) event.
#[derive(Debug, Clone, Deserialize)]
pub struct StopEvent {
	/// Pulse position.
	pub y: u64,
	/// Duration of the stop in pulses.
	pub duration: u64,
}

/// A scroll-speed change event (beatoraja extension).
#[derive(Debug, Clone, Deserialize)]
pub struct ScrollEvent {
	/// Pulse position.
	pub y: u64,
	/// Scroll speed multiplier.
	pub rate: f64,
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors that can occur when parsing a BMSON file.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
	/// The JSON could not be parsed or a required field was missing.
	#[error("JSON parse error: {0}")]
	Json(#[from] serde_json::Error),
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// Parse a BMSON file from its raw bytes.
pub fn from_bytes(bytes: &[u8]) -> Result<Bmson, LoadError> {
	Ok(serde_json::from_slice(bytes)?)
}

/// Read a `.bmson` file from disk and parse it.
///
/// The outer `io::Error` indicates a file-read failure; the inner [`LoadError`]
/// indicates a JSON parse failure — matching the double-`Result` pattern used
/// by `rg_formats::bms::from_file`.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Result<Bmson, LoadError>> {
	let bytes = std::fs::read(path)?;
	Ok(from_bytes(&bytes))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	fn minimal_bmson(extra: &str) -> String {
		format!(
			r#"{{"version":"1.0.0","info":{{"title":"Test","artist":"Artist","init_bpm":120.0{extra}}},"sound_channels":[]}}"#
		)
	}

	#[test]
	fn parses_minimal_bmson() {
		let b: Bmson = from_bytes(minimal_bmson("").as_bytes()).unwrap();
		assert_eq!(b.info.title, "Test");
		assert_eq!(b.info.artist, "Artist");
		assert!((b.info.init_bpm - 120.0).abs() < f64::EPSILON);
	}

	#[test]
	fn defaults_applied() {
		let b: Bmson = from_bytes(minimal_bmson("").as_bytes()).unwrap();
		assert_eq!(b.info.mode_hint, "beat-7k");
		assert!((b.info.judge_rank - 100.0).abs() < f64::EPSILON);
		assert!((b.info.total - 100.0).abs() < f64::EPSILON);
		assert_eq!(b.info.resolution, 240);
		assert_eq!(b.info.subtitle, "");
		assert!(b.sound_channels.is_empty());
		assert!(b.key_channels.is_empty());
		assert!(b.mine_channels.is_empty());
	}

	#[test]
	fn parses_sound_channels() {
		let json = r#"{
			"version":"1.0.0",
			"info":{"title":"T","artist":"A","init_bpm":130.0},
			"sound_channels":[
				{"name":"kick.wav","notes":[{"x":1,"y":240,"l":0,"c":false}]},
				{"name":"bgm.ogg","notes":[{"x":0,"y":0,"l":0,"c":false}]}
			]
		}"#;
		let b: Bmson = from_bytes(json.as_bytes()).unwrap();
		assert_eq!(b.sound_channels.len(), 2);
		assert_eq!(b.sound_channels[0].name, "kick.wav");
		assert_eq!(b.sound_channels[0].notes[0].x, 1);
		assert_eq!(b.sound_channels[1].notes[0].x, 0);
	}

	#[test]
	fn parses_bga_headers() {
		let json = r#"{
			"version":"1.0.0",
			"info":{"title":"T","artist":"A","init_bpm":130.0},
			"bga":{"bga_header":[{"id":1,"name":"bg.png"}],"bga_events":[],"layer_events":[],"poor_events":[]}
		}"#;
		let b: Bmson = from_bytes(json.as_bytes()).unwrap();
		let bga = b.bga.unwrap();
		assert_eq!(bga.bga_header.len(), 1);
		assert_eq!(bga.bga_header[0].name, "bg.png");
	}

	#[test]
	fn unknown_fields_ignored() {
		let json = r#"{
			"version":"1.0.0",
			"info":{"title":"T","artist":"A","init_bpm":130.0,"unknown_field":"ignored"},
			"future_extension":42
		}"#;
		assert!(from_bytes(json.as_bytes()).is_ok());
	}

	#[test]
	fn invalid_json_returns_error() {
		assert!(from_bytes(b"not json at all").is_err());
	}

	#[test]
	fn note_x_defaults_to_zero() {
		let json = r#"{
			"version":"1.0.0",
			"info":{"title":"T","artist":"A","init_bpm":130.0},
			"sound_channels":[{"name":"bgm.wav","notes":[{"y":240}]}]
		}"#;
		let b: Bmson = from_bytes(json.as_bytes()).unwrap();
		assert_eq!(b.sound_channels[0].notes[0].x, 0);
	}
}
