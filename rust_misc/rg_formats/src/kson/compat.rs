//! Editor and KSH compatibility data types (`editor` and `compat` objects).

use std::collections::HashMap;

use serde::Deserialize;

use super::common::ByPulse;

// ── Editor ────────────────────────────────────────────────────────────────────

/// Editor-only data (`editor` object, OPTIONAL SUPPORT).
///
/// Clients that do not support editing may ignore this section entirely.
#[derive(Debug, Clone, Deserialize)]
pub struct EditorInfo {
	/// Editor application name (OPTIONAL SUPPORT).
	#[serde(default)]
	pub app_name: Option<String>,
	/// Editor application version (OPTIONAL SUPPORT).
	#[serde(default)]
	pub app_version: Option<String>,
	/// In-editor comments placed at specific pulse positions (OPTIONAL SUPPORT).
	#[serde(default)]
	pub comment: Option<Vec<ByPulse<String>>>,
}

// ── Compat ────────────────────────────────────────────────────────────────────

/// Unrecognized KSH data preserved during ksh→kson conversion (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KSHUnknownInfo {
	/// Unrecognized header option lines (key → value).
	#[serde(default)]
	pub meta: Option<HashMap<String, String>>,
	/// Unrecognized body option lines by pulse (key → `[[y, value], ...]`).
	#[serde(default)]
	pub option: Option<HashMap<String, Vec<ByPulse<String>>>>,
	/// Unrecognized non-option body lines by pulse.
	#[serde(default)]
	pub line: Option<Vec<ByPulse<String>>>,
}

/// KSH compatibility data (`compat` object, OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct CompatInfo {
	/// KSH `ver` field value (OPTIONAL SUPPORT).
	#[serde(default)]
	pub ksh_version: Option<String>,
	/// Unrecognized KSH data preserved during conversion (OPTIONAL SUPPORT).
	#[serde(default)]
	pub ksh_unknown: Option<KSHUnknownInfo>,
}
