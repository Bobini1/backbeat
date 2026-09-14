//! Beat, timing, and gauge data (`beat` and `gauge` objects).

use serde::Deserialize;

use super::common::{ByMeasureIdx, ByPulse, GraphCurveValue, GraphPoint, GraphValueKind, TimeSig};

// ── Default helpers ───────────────────────────────────────────────────────────

fn default_time_sig() -> Vec<ByMeasureIdx<TimeSig>> {
	vec![ByMeasureIdx {
		idx: 0,
		v: TimeSig { n: 4, d: 4 },
	}]
}

fn default_scroll_speed() -> Vec<GraphPoint> {
	vec![GraphPoint {
		y: 0,
		v: GraphValueKind::Single(1.0),
		curve: GraphCurveValue::default(),
	}]
}

// ── Beat ──────────────────────────────────────────────────────────────────────

/// Beat and timing data (`beat` object).
#[derive(Debug, Clone, Deserialize)]
pub struct BeatInfo {
	/// BPM change events: `[y, bpm]`. Must contain at least one entry.
	pub bpm: Vec<ByPulse<f64>>,
	/// Time signature changes: `[measure_idx, [n, d]]`. Default `[[0, [4, 4]]]`.
	#[serde(default = "default_time_sig")]
	pub time_sig: Vec<ByMeasureIdx<TimeSig>>,
	/// Scroll speed multiplier changes. Default `[[0, 1.0]]`.
	#[serde(default = "default_scroll_speed")]
	pub scroll_speed: Vec<GraphPoint>,
	/// Stop events: `[y, duration_in_pulses]`.
	///
	/// Stops freeze the scroll for the given pulse duration. Takes priority over
	/// `scroll_speed` at the same position.
	#[serde(default)]
	pub stop: Option<Vec<ByPulse<u64>>>,
}

// ── Gauge ─────────────────────────────────────────────────────────────────────

/// Gauge-related data (`gauge` object).
#[derive(Debug, Clone, Deserialize)]
pub struct GaugeInfo {
	/// Total gauge ascension over the chart as a percentage (0 = auto-calculated).
	#[serde(default)]
	pub total: u64,
}
