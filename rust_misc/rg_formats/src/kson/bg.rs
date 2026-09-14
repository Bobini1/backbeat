//! Background data types (`bg` object): KSH-style backgrounds, layers, movies.

use serde::Deserialize;

// ── Default helpers ───────────────────────────────────────────────────────────

fn bool_true() -> bool {
	true
}

// ── KSH legacy background types ───────────────────────────────────────────────

/// A KSH-style background image entry (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KSHBGInfo {
	/// Background image filename (may be a preset name such as `"desert"`).
	#[serde(default)]
	pub filename: Option<String>,
}

/// KSH-style layer rotation conditions (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KSHLayerRotationInfo {
	/// Whether lane tilts affect BG/layer rotation (default `true`).
	#[serde(default = "bool_true")]
	pub tilt: bool,
	/// Whether lane spins affect BG/layer rotation (default `true`).
	#[serde(default = "bool_true")]
	pub spin: bool,
}

/// A KSH-style animation layer (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KSHLayerInfo {
	/// Layer animation filename (may be a preset name such as `"arrow"`).
	#[serde(default)]
	pub filename: Option<String>,
	/// One-loop duration in milliseconds. `0` = tempo-synced; negative = reverse.
	#[serde(default)]
	pub duration: i64,
	/// Rotation behaviour.
	#[serde(default)]
	pub rotation: Option<KSHLayerRotationInfo>,
}

/// A KSH-style video/movie background (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KSHMovieInfo {
	/// Video filename.
	#[serde(default)]
	pub filename: Option<String>,
	/// Video start offset in milliseconds (default `0`).
	#[serde(default)]
	pub offset: i64,
}

/// Legacy KSH-style background data (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyBGInfo {
	/// Background images indexed by gauge threshold.
	///
	/// Index `0` = gauge < 70 %, index `1` = gauge ≥ 70 %. If only one element
	/// is present it is always used.
	#[serde(default)]
	pub bg: Option<Vec<KSHBGInfo>>,
	/// Animation layer.
	#[serde(default)]
	pub layer: Option<KSHLayerInfo>,
	/// Video/movie background.
	#[serde(default)]
	pub movie: Option<KSHMovieInfo>,
}

// ── BGInfo ────────────────────────────────────────────────────────────────────

/// Background data (`bg` object).
#[derive(Debug, Clone, Deserialize)]
pub struct BGInfo {
	/// Background graphics filename (OPTIONAL SUPPORT; reserved for future use).
	#[serde(default)]
	pub filename: Option<String>,
	/// Legacy KSH-style background data (OPTIONAL SUPPORT).
	#[serde(default)]
	pub legacy: Option<LegacyBGInfo>,
}
