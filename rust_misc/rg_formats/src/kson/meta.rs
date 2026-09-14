//! Chart metadata types (`meta` object).

use serde::Deserialize;

/// Chart difficulty: a numeric index (0–3) or a custom name string.
///
/// Numeric indices: `0` = light, `1` = challenge, `2` = extended, `3` = infinite.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Difficulty {
	/// Standard difficulty index.
	Index(u64),
	/// Custom difficulty name (e.g. `"gravity"`, `"maximum"`).
	Name(String),
}

/// Song and chart metadata (`meta` object).
#[derive(Debug, Clone, Deserialize)]
pub struct MetaInfo {
	/// Song title.
	pub title: String,
	/// Transliterated title (OPTIONAL SUPPORT).
	#[serde(default)]
	pub title_translit: Option<String>,
	/// Image filename to use instead of title text (OPTIONAL SUPPORT).
	#[serde(default)]
	pub title_img_filename: Option<String>,
	/// Song artist.
	pub artist: String,
	/// Transliterated artist name (OPTIONAL SUPPORT).
	#[serde(default)]
	pub artist_translit: Option<String>,
	/// Image filename to use instead of artist text (OPTIONAL SUPPORT).
	#[serde(default)]
	pub artist_img_filename: Option<String>,
	/// Chart author.
	pub chart_author: String,
	/// Chart difficulty.
	pub difficulty: Difficulty,
	/// Numeric chart level (1–20).
	pub level: u64,
	/// Displayed BPM string (may use `"-"` for ranges, e.g. `"120-200"`).
	pub disp_bpm: String,
	/// Standard BPM for hi-speed calculations (OPTIONAL SUPPORT).
	#[serde(default)]
	pub std_bpm: Option<f64>,
	/// Jacket image filename.
	#[serde(default)]
	pub jacket_filename: Option<String>,
	/// Jacket image author.
	#[serde(default)]
	pub jacket_author: Option<String>,
	/// Icon image filename shown on music selection (OPTIONAL SUPPORT).
	#[serde(default)]
	pub icon_filename: Option<String>,
	/// Optional informational text shown in song selection (OPTIONAL SUPPORT).
	#[serde(default)]
	pub information: Option<String>,
}
