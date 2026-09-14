//! Common array-encoded primitive types shared across the KSON spec.
//!
//! All of these types are encoded as fixed-length JSON arrays rather than
//! objects, so they require custom [`Deserialize`] implementations.

use serde::de;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

// ── Pulse / measure event wrappers ────────────────────────────────────────────

/// An event at a specific pulse: `[y, v]`.
///
/// Pulse numbers have a resolution of 240 per beat (960 per 4/4 measure).
#[derive(Debug, Clone)]
pub struct ByPulse<T> {
	/// Absolute pulse number.
	pub y: u64,
	/// Event value.
	pub v: T,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ByPulse<T> {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let (y, v) = <(u64, T)>::deserialize(deserializer)?;
		Ok(Self { y, v })
	}
}

/// An event at a specific measure index: `[idx, v]`.
#[derive(Debug, Clone)]
pub struct ByMeasureIdx<T> {
	/// Zero-based measure index.
	pub idx: u64,
	/// Event value.
	pub v: T,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ByMeasureIdx<T> {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let (idx, v) = <(u64, T)>::deserialize(deserializer)?;
		Ok(Self { idx, v })
	}
}

/// A named definition entry: `[name, v]`.
///
/// Used for audio effect definition lists, which are arrays of key-value pairs
/// (rather than objects) to preserve insertion order.
#[derive(Debug, Clone)]
pub struct DefKeyValuePair<T> {
	/// Definition name (key).
	pub name: String,
	/// Definition value.
	pub v: T,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for DefKeyValuePair<T> {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let (name, v) = <(String, T)>::deserialize(deserializer)?;
		Ok(Self { name, v })
	}
}

// ── Graph primitives ──────────────────────────────────────────────────────────

/// A graph value: either a single `f64` (smooth interpolation) or a pair
/// `[v, vf]` representing an immediate change at the same pulse.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GraphValueKind {
	/// Single value; start and end are identical (no immediate change).
	Single(f64),
	/// Immediate change: the value jumps from `v` to `vf` at this pulse.
	Pair([f64; 2]),
}

impl GraphValueKind {
	/// Returns the start value `v`.
	pub fn v(&self) -> f64 {
		match self {
			Self::Single(v) | Self::Pair([v, _]) => *v,
		}
	}

	/// Returns the end value `vf` (equals `v` for [`Single`](Self::Single)).
	pub fn vf(&self) -> f64 {
		match self {
			Self::Single(v) | Self::Pair([_, v]) => *v,
		}
	}
}

/// Bézier curve control point `[a, b]`, both coordinates in `[0.0, 1.0]`.
///
/// Used to shape the interpolation curve from one graph point to the next.
/// The default `[0.0, 0.0]` produces linear interpolation.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GraphCurveValue(pub [f64; 2]);

/// A timed point in a whole-chart graph: `[y, v, curve?]`.
///
/// Arrays of `GraphPoint` must be ordered by `y`.
#[derive(Debug, Clone)]
pub struct GraphPoint {
	/// Absolute pulse number.
	pub y: u64,
	/// Graph value at this point (may include an immediate change).
	pub v: GraphValueKind,
	/// Bézier curve shaping interpolation to the next point. Default `[0.0, 0.0]`.
	pub curve: GraphCurveValue,
}

impl<'de> Deserialize<'de> for GraphPoint {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let arr = Value::deserialize(deserializer)?;
		let arr = arr
			.as_array()
			.ok_or_else(|| de::Error::custom("GraphPoint must be an array"))?;
		if arr.len() < 2 || arr.len() > 3 {
			return Err(de::Error::custom("GraphPoint must have 2 or 3 elements"));
		}
		let y = arr[0]
			.as_u64()
			.ok_or_else(|| de::Error::custom("GraphPoint[0] (y) must be a non-negative integer"))?;
		let v: GraphValueKind =
			serde_json::from_value(arr[1].clone()).map_err(de::Error::custom)?;
		let curve = if arr.len() == 3 {
			serde_json::from_value(arr[2].clone()).map_err(de::Error::custom)?
		} else {
			GraphCurveValue::default()
		};
		Ok(Self { y, v, curve })
	}
}

/// A timed point within a graph section (e.g. a laser segment): `[ry, v, curve?]`.
///
/// `ry` is a *relative* pulse offset from the section start. The first point
/// in a section must have `ry == 0`.
#[derive(Debug, Clone)]
pub struct GraphSectionPoint {
	/// Relative pulse number from the section start (0 for the first point).
	pub ry: u64,
	/// Graph value at this point (may include an immediate change).
	pub v: GraphValueKind,
	/// Bézier curve shaping interpolation to the next point. Default `[0.0, 0.0]`.
	pub curve: GraphCurveValue,
}

impl<'de> Deserialize<'de> for GraphSectionPoint {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let arr = Value::deserialize(deserializer)?;
		let arr = arr
			.as_array()
			.ok_or_else(|| de::Error::custom("GraphSectionPoint must be an array"))?;
		if arr.len() < 2 || arr.len() > 3 {
			return Err(de::Error::custom(
				"GraphSectionPoint must have 2 or 3 elements",
			));
		}
		let ry = arr[0].as_u64().ok_or_else(|| {
			de::Error::custom("GraphSectionPoint[0] (ry) must be a non-negative integer")
		})?;
		let v: GraphValueKind =
			serde_json::from_value(arr[1].clone()).map_err(de::Error::custom)?;
		let curve = if arr.len() == 3 {
			serde_json::from_value(arr[2].clone()).map_err(de::Error::custom)?
		} else {
			GraphCurveValue::default()
		};
		Ok(Self { ry, v, curve })
	}
}

// ── Time signature ────────────────────────────────────────────────────────────

/// A time signature: `[numerator, denominator]`.
#[derive(Debug, Clone)]
pub struct TimeSig {
	/// Beats per measure.
	pub n: u32,
	/// Beat unit (e.g. 4 = quarter note, 8 = eighth note).
	pub d: u32,
}

impl<'de> Deserialize<'de> for TimeSig {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let (n, d) = <(u32, u32)>::deserialize(deserializer)?;
		Ok(Self { n, d })
	}
}
