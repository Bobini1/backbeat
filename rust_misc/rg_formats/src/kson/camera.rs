//! Camera data types (`camera` object): tilt, CAM graphs, and patterns.

use serde::de;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::common::{ByPulse, GraphPoint};

// ── Default helpers ───────────────────────────────────────────────────────────

fn default_swing_scale() -> f64 {
	250.0
}
fn default_swing_repeat() -> u64 {
	3
}
fn default_swing_decay() -> u64 {
	2
}

fn default_tilt() -> Vec<ByPulse<TiltValue>> {
	vec![ByPulse {
		y: 0,
		v: TiltValue::Auto("normal".to_owned()),
	}]
}

// ── TiltValue ─────────────────────────────────────────────────────────────────

/// Lane tilt value, covering all variants defined in the KSON spec.
///
/// The JSON representation varies by variant (string, number, or nested arrays),
/// so this type uses a custom deserializer via an intermediate [`Value`].
#[derive(Debug, Clone)]
pub enum TiltValue {
	/// Named auto-tilt mode: `"normal"`, `"bigger"`, `"biggest"`,
	/// `"keep_normal"`, `"keep_bigger"`, `"keep_biggest"`, or `"zero"`.
	Auto(String),
	/// Manual tilt amount interpolated linearly to the next point.
	Manual(f64),
	/// Manual tilt with an immediate jump from `v` to `vf` at this pulse,
	/// then interpolates from `vf`.
	ManualImmediate {
		/// Tilt value before the jump.
		v: f64,
		/// Tilt value after the jump.
		vf: f64,
	},
	/// Manual tilt value that immediately transitions to a named auto-tilt mode.
	ManualToAuto {
		/// Manual tilt value before the transition.
		v: f64,
		/// Auto-tilt type name (e.g. `"normal"`, `"bigger"`).
		vf: String,
	},
	/// Manual tilt with a Bézier curve shaping interpolation to the next point.
	ManualWithCurve {
		/// Tilt value at this point.
		v: f64,
		/// Curve control point `[a, b]` (both in `[0.0, 1.0]`).
		curve: [f64; 2],
	},
	/// Manual tilt with an immediate jump and a Bézier curve to the next point.
	ManualImmediateWithCurve {
		/// Tilt value before the jump.
		v: f64,
		/// Tilt value after the jump (interpolates from here to the next point).
		vf: f64,
		/// Curve control point `[a, b]` (both in `[0.0, 1.0]`).
		curve: [f64; 2],
	},
}

impl<'de> Deserialize<'de> for TiltValue {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let val = Value::deserialize(deserializer)?;
		tilt_from_value(&val).map_err(de::Error::custom)
	}
}

fn tilt_from_value(val: &Value) -> Result<TiltValue, String> {
	match val {
		Value::String(s) => Ok(TiltValue::Auto(s.clone())),
		Value::Number(n) => n
			.as_f64()
			.map(TiltValue::Manual)
			.ok_or_else(|| "TiltValue number must be representable as f64".to_owned()),
		Value::Array(arr) => {
			if arr.len() < 2 {
				return Err("TiltValue array must have at least 2 elements".to_owned());
			}
			match &arr[0] {
				// [double, ...] → ManualImmediate, ManualToAuto, or ManualWithCurve
				Value::Number(n0) => {
					let v = n0
						.as_f64()
						.ok_or_else(|| "TiltValue[0] must be f64".to_owned())?;
					match &arr[1] {
						Value::Number(n1) => {
							let vf = n1
								.as_f64()
								.ok_or_else(|| "TiltValue[1] (vf) must be f64".to_owned())?;
							Ok(TiltValue::ManualImmediate { v, vf })
						}
						Value::String(s) => Ok(TiltValue::ManualToAuto { v, vf: s.clone() }),
						Value::Array(curve_arr) => {
							let curve = parse_curve(curve_arr)?;
							Ok(TiltValue::ManualWithCurve { v, curve })
						}
						_ => Err("TiltValue[1] must be number, string, or array".to_owned()),
					}
				}
				// [[double, double], [double, double]] → ManualImmediateWithCurve
				Value::Array(pair) => {
					if pair.len() != 2 {
						return Err(
							"TiltValue immediate pair must have exactly 2 elements".to_owned()
						);
					}
					let v = pair[0]
						.as_f64()
						.ok_or_else(|| "TiltValue[0][0] (v) must be f64".to_owned())?;
					let vf = pair[1]
						.as_f64()
						.ok_or_else(|| "TiltValue[0][1] (vf) must be f64".to_owned())?;
					let curve_arr = arr[1]
						.as_array()
						.ok_or_else(|| "TiltValue[1] (curve) must be an array".to_owned())?;
					let curve = parse_curve(curve_arr)?;
					Ok(TiltValue::ManualImmediateWithCurve { v, vf, curve })
				}
				_ => Err("TiltValue array must start with a number or an array".to_owned()),
			}
		}
		_ => Err("TiltValue must be a string, number, or array".to_owned()),
	}
}

fn parse_curve(arr: &[Value]) -> Result<[f64; 2], String> {
	if arr.len() != 2 {
		return Err(format!(
			"curve must be exactly 2 elements, got {}",
			arr.len()
		));
	}
	let a = arr[0]
		.as_f64()
		.ok_or_else(|| "curve[0] must be f64".to_owned())?;
	let b = arr[1]
		.as_f64()
		.ok_or_else(|| "curve[1] must be f64".to_owned())?;
	Ok([a, b])
}

// ── CAM graphs ────────────────────────────────────────────────────────────────

/// Camera graph values for the whole chart (`camera.cam.body`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CamGraphs {
	/// Rotate the upper edge of the highway around the judgment line.
	#[serde(default)]
	pub zoom_top: Option<Vec<GraphPoint>>,
	/// Move the bottom edge of the highway toward the camera.
	#[serde(default)]
	pub zoom_bottom: Option<Vec<GraphPoint>>,
	/// Move the highway horizontally.
	#[serde(default)]
	pub zoom_side: Option<Vec<GraphPoint>>,
	/// Rotation of both highway and judgment line (degrees).
	#[serde(default)]
	pub rotation_deg: Option<Vec<GraphPoint>>,
	/// Split the highway at the center.
	#[serde(default)]
	pub center_split: Option<Vec<GraphPoint>>,
}

// ── Camera patterns ───────────────────────────────────────────────────────────

/// Swing camera pattern parameters (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct CamPatternInvokeSwingValue {
	/// Swing scale (default `250.0`).
	#[serde(default = "default_swing_scale")]
	pub scale: f64,
	/// Number of swing repetitions (default `3`).
	#[serde(default = "default_swing_repeat")]
	pub repeat: u64,
	/// Decay order 0–2 (default `2`).
	#[serde(default = "default_swing_decay")]
	pub decay_order: u64,
}

/// A spin (or half-spin) camera pattern invocation: `[y, direction, length]`.
#[derive(Debug, Clone)]
pub struct CamPatternInvokeSpin {
	/// Pulse number (must match an existing laser slam).
	pub y: u64,
	/// Laser slam direction: `-1` (left) or `1` (right).
	pub direction: i64,
	/// Spin duration in pulses.
	pub length: u64,
}

impl<'de> Deserialize<'de> for CamPatternInvokeSpin {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let (y, direction, length) = <(u64, i64, u64)>::deserialize(deserializer)?;
		Ok(Self {
			y,
			direction,
			length,
		})
	}
}

/// A swing camera pattern invocation: `[y, direction, length, v?]` (OPTIONAL SUPPORT).
#[derive(Debug, Clone)]
pub struct CamPatternInvokeSwing {
	/// Pulse number (must match an existing laser slam).
	pub y: u64,
	/// Laser slam direction: `-1` (left) or `1` (right).
	pub direction: i64,
	/// Swing duration in pulses.
	pub length: u64,
	/// Optional swing value parameters.
	pub v: Option<CamPatternInvokeSwingValue>,
}

impl<'de> Deserialize<'de> for CamPatternInvokeSwing {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let arr = Value::deserialize(deserializer)?;
		let arr = arr
			.as_array()
			.ok_or_else(|| de::Error::custom("CamPatternInvokeSwing must be an array"))?;
		if arr.len() < 3 || arr.len() > 4 {
			return Err(de::Error::custom(
				"CamPatternInvokeSwing must have 3 or 4 elements",
			));
		}
		let y = arr[0]
			.as_u64()
			.ok_or_else(|| de::Error::custom("CamPatternInvokeSwing[0] (y) must be uint"))?;
		let direction = arr[1]
			.as_i64()
			.ok_or_else(|| de::Error::custom("CamPatternInvokeSwing[1] (direction) must be int"))?;
		let length = arr[2]
			.as_u64()
			.ok_or_else(|| de::Error::custom("CamPatternInvokeSwing[2] (length) must be uint"))?;
		let v = if arr.len() == 4 {
			Some(serde_json::from_value(arr[3].clone()).map_err(de::Error::custom)?)
		} else {
			None
		};
		Ok(Self {
			y,
			direction,
			length,
			v,
		})
	}
}

/// Camera pattern events triggered by laser slam notes.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CamPatternLaserInvokeList {
	/// Full-spin events.
	#[serde(default)]
	pub spin: Option<Vec<CamPatternInvokeSpin>>,
	/// Half-spin events.
	#[serde(default)]
	pub half_spin: Option<Vec<CamPatternInvokeSpin>>,
	/// Swing events (OPTIONAL SUPPORT).
	#[serde(default)]
	pub swing: Option<Vec<CamPatternInvokeSwing>>,
}

/// Camera pattern settings for laser slams.
#[derive(Debug, Clone, Deserialize)]
pub struct CamPatternLaserInfo {
	/// Camera pattern events for laser slams.
	#[serde(default)]
	pub slam_event: Option<CamPatternLaserInvokeList>,
}

/// Camera pattern settings.
#[derive(Debug, Clone, Deserialize)]
pub struct CamPatternInfo {
	/// Pattern settings for laser slams.
	#[serde(default)]
	pub laser: Option<CamPatternLaserInfo>,
}

/// CAM (camera animation) data.
#[derive(Debug, Clone, Deserialize)]
pub struct CamInfo {
	/// Camera value graph changes (zoom, rotation, etc.).
	#[serde(default)]
	pub body: Option<CamGraphs>,
	/// Camera pattern settings.
	#[serde(default)]
	pub pattern: Option<CamPatternInfo>,
}

// ── CameraInfo ────────────────────────────────────────────────────────────────

/// Camera data (`camera` object).
#[derive(Debug, Clone, Deserialize)]
pub struct CameraInfo {
	/// Tilt value changes. Default `[[0, "normal"]]`.
	#[serde(default = "default_tilt")]
	pub tilt: Vec<ByPulse<TiltValue>>,
	/// CAM animation data.
	#[serde(default)]
	pub cam: Option<CamInfo>,
}
