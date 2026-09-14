use std::collections::HashMap;

use super::raw_headers::RawHeaders;
use super::{parse_beat, parse_initial_bpm};

// ── Difficulty ────────────────────────────────────────────────────────────────

/// Chart difficulty level.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Difficulty {
	Light,
	Challenge,
	Extended,
	/// Catch-all for `"infinite"` and any unrecognised value.
	#[default]
	Infinite,
}

impl Difficulty {
	fn from_str(s: &str) -> Self {
		match s {
			"light" => Self::Light,
			"challenge" => Self::Challenge,
			"extended" => Self::Extended,
			_ => Self::Infinite,
		}
	}
}

// ── Header ────────────────────────────────────────────────────────────────────

/// All options from the KSH header section (lines before the first `--`).
#[derive(Debug, Clone)]
pub struct Header {
	// ── Song info ─────────────────────────────────────────────────────────────
	pub title: String,
	pub title_translit: String,
	pub title_img: String,
	pub artist: String,
	pub artist_translit: String,
	pub artist_img: String,
	/// Chart author (`effect=`).
	pub effect: String,
	pub illustrator: String,
	pub information: String,

	// ── Chart info ────────────────────────────────────────────────────────────
	pub jacket: String,
	pub difficulty: Difficulty,
	/// Chart level (1–20).
	pub level: u32,

	// ── Timing ───────────────────────────────────────────────────────────────
	/// Raw `t=` value from the header (`"120"` or `"120-220"` for a BPM range).
	pub bpm: String,
	/// Parsed initial BPM (`0.0` if the header value could not be parsed).
	pub bpm_init: f64,
	/// Standard BPM for Hi-speed calculation (`to=`; `0.0` = auto).
	pub standard_bpm: f64,
	/// Time signature `(numerator, denominator)` (default `(4, 4)`).
	pub beat: (u32, u32),

	// ── Audio ─────────────────────────────────────────────────────────────────
	/// Audio filename(s) from `m=`, split on `;`. Up to 4 variants (default,
	/// FX, laser, FX+laser).
	pub audio: Vec<String>,
	/// Song volume percentage (`mvol=`; default 100).
	pub music_vol: u32,
	/// Song start offset in milliseconds (`o=`).
	pub offset: i32,
	/// Preview start offset in milliseconds (`po=`).
	pub preview_offset: u32,
	/// Preview length in milliseconds (`plength=`).
	pub preview_length: u32,

	// ── Visual ────────────────────────────────────────────────────────────────
	/// Background image(s) (`bg=`), split on `;`. Preset names kept as-is.
	pub bg: Vec<String>,
	/// Layer animation (`layer=`), raw value (preset name or filename, with
	/// optional speed/rotation parameters retained).
	pub layer: String,
	/// Video filename (`v=`).
	pub video: String,
	/// Video offset in milliseconds (`vo=`).
	pub video_offset: i32,

	// ── Gameplay ──────────────────────────────────────────────────────────────
	pub total: u32,
	pub chokkaku_vol: u8,
	pub chokkaku_auto_vol: bool,
	pub filter_type: String,
	pub pfilter_gain: u8,
	pub pfilter_delay: u32,

	// ── Version ──────────────────────────────────────────────────────────────
	pub version: String,
	pub version_compat: String,

	// ── Catch-all ────────────────────────────────────────────────────────────
	/// Any header keys not recognised by this parser.
	pub unknown: HashMap<String, String>,
}

impl Default for Header {
	fn default() -> Self {
		Self {
			title: String::new(),
			title_translit: String::new(),
			title_img: String::new(),
			artist: String::new(),
			artist_translit: String::new(),
			artist_img: String::new(),
			effect: String::new(),
			illustrator: String::new(),
			information: String::new(),
			jacket: String::new(),
			difficulty: Difficulty::default(),
			level: 1,
			bpm: String::new(),
			bpm_init: 0.0,
			standard_bpm: 0.0,
			beat: (4, 4),
			audio: Vec::new(),
			music_vol: 100,
			offset: 0,
			preview_offset: 0,
			preview_length: 0,
			bg: Vec::new(),
			layer: String::new(),
			video: String::new(),
			video_offset: 0,
			total: 0,
			chokkaku_vol: 50,
			chokkaku_auto_vol: true,
			filter_type: "peak".to_owned(),
			pfilter_gain: 50,
			pfilter_delay: 40,
			version: String::new(),
			version_compat: String::new(),
			unknown: HashMap::new(),
		}
	}
}

impl Header {
	pub(super) fn parse(raw_headers: &RawHeaders) -> Self {
		let mut h = Self::default();

		for (key, value) in raw_headers {
			let value = value.to_owned();
			match key {
				"title" => h.title = value,
				"title_translit" => h.title_translit = value,
				"title_img" => h.title_img = value,
				"artist" => h.artist = value,
				"artist_translit" => h.artist_translit = value,
				"artist_img" => h.artist_img = value,
				"effect" => h.effect = value,
				"jacket" => h.jacket = value,
				"illustrator" => h.illustrator = value,
				"difficulty" => h.difficulty = Difficulty::from_str(&value),
				"level" => h.level = value.parse().unwrap_or(1),
				"t" => {
					h.bpm = value.clone();
					h.bpm_init = parse_initial_bpm(&value);
				}
				"to" => h.standard_bpm = value.parse().unwrap_or(0.0),
				"beat" => h.beat = parse_beat(&value).unwrap_or((4, 4)),
				"m" => h.audio = split_paths(&value),
				"mvol" => h.music_vol = value.parse().unwrap_or(100),
				"o" => h.offset = value.parse().unwrap_or(0),
				"bg" => h.bg = split_paths(&value),
				"layer" => h.layer = value,
				"po" => h.preview_offset = value.parse().unwrap_or(0),
				"plength" => h.preview_length = value.parse().unwrap_or(0),
				"total" => h.total = value.parse().unwrap_or(0),
				"chokkakuvol" => h.chokkaku_vol = value.parse().unwrap_or(50),
				"chokkakuautovol" => h.chokkaku_auto_vol = value != "0",
				"filtertype" => h.filter_type = value,
				"pfiltergain" => h.pfilter_gain = value.parse().unwrap_or(50),
				"pfilterdelay" => h.pfilter_delay = value.parse().unwrap_or(40),
				"v" => h.video = value,
				"vo" => h.video_offset = value.parse().unwrap_or(0),
				"ver" => h.version = value,
				"ver_compat" => h.version_compat = value,
				"information" => h.information = value,
				_ => {
					h.unknown.insert(key.to_owned(), value);
				}
			}
		}

		h
	}
}

fn split_paths(value: &str) -> Vec<String> {
	value
		.split(';')
		.filter(|value| !value.is_empty())
		.map(str::to_owned)
		.collect()
}
