//! Note data types (`note` object): BT, FX, and laser notes.

use serde::de;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::common::GraphSectionPoint;

// ── Button notes ──────────────────────────────────────────────────────────────

/// A BT or FX note entry within a lane.
///
/// A bare `uint` in the JSON is a chip note (pulse position only). A
/// two-element array `[y, length]` is a long/hold note.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ButtonNoteEntry {
	/// Chip note: the value is the pulse position `y`.
	Chip(u64),
	/// Long (hold) note: `[y, length]` where `length > 0`.
	Long([u64; 2]),
}

impl ButtonNoteEntry {
	/// Returns the pulse position of this note.
	pub fn y(&self) -> u64 {
		match self {
			Self::Chip(y) => *y,
			Self::Long([y, _]) => *y,
		}
	}

	/// Returns the length of this note in pulses (0 for chip notes).
	pub fn length(&self) -> u64 {
		match self {
			Self::Chip(_) => 0,
			Self::Long([_, len]) => *len,
		}
	}
}

// ── Laser sections ────────────────────────────────────────────────────────────

/// A laser knob section: `[y, points[], w?]`.
///
/// The `v` field is a graph of laser cursor positions in the range `[0.0, 1.0]`
/// (left edge to right edge).
#[derive(Debug, Clone)]
pub struct LaserSection {
	/// Absolute pulse number where this section starts.
	pub y: u64,
	/// Laser position graph points. The first point must have `ry == 0`.
	pub v: Vec<GraphSectionPoint>,
	/// X-axis scale: `1` = normal width, `2` = double-width. Default `1`.
	pub w: u64,
}

impl<'de> Deserialize<'de> for LaserSection {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let arr = Value::deserialize(deserializer)?;
		let arr = arr
			.as_array()
			.ok_or_else(|| de::Error::custom("LaserSection must be an array"))?;
		if arr.len() < 2 || arr.len() > 3 {
			return Err(de::Error::custom("LaserSection must have 2 or 3 elements"));
		}
		let y = arr[0].as_u64().ok_or_else(|| {
			de::Error::custom("LaserSection[0] (y) must be a non-negative integer")
		})?;
		let v: Vec<GraphSectionPoint> =
			serde_json::from_value(arr[1].clone()).map_err(de::Error::custom)?;
		let w = if arr.len() == 3 {
			arr[2].as_u64().ok_or_else(|| {
				de::Error::custom("LaserSection[2] (w) must be a non-negative integer")
			})?
		} else {
			1
		};
		Ok(Self { y, v, w })
	}
}

// ── NoteInfo ──────────────────────────────────────────────────────────────────

/// Note data (`note` object).
#[derive(Debug, Clone, Deserialize)]
pub struct NoteInfo {
	/// BT button lanes: `[lane_A, lane_B, lane_C, lane_D]`.
	#[serde(default)]
	pub bt: Option<[Vec<ButtonNoteEntry>; 4]>,
	/// FX button lanes: `[lane_left, lane_right]`.
	#[serde(default)]
	pub fx: Option<[Vec<ButtonNoteEntry>; 2]>,
	/// Laser knob lanes: `[left_knob, right_knob]`.
	#[serde(default)]
	pub laser: Option<[Vec<LaserSection>; 2]>,
}
