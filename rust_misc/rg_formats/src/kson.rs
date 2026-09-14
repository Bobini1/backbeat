//! Parser for the `.kson` (K-Shoot MANIA) chart format.
//!
//! KSON is a JSON-based rhythm game chart format (version 1.0.0). Unlike the
//! older text-based KSH format, one `.kson` file represents exactly one chart
//! and encodes all timing, note, and effect data in a single structured JSON
//! object.
//!
//! # Usage
//!
//! ```rust,ignore
//! let chart = rg_formats::kson::from_file("song.kson")
//!     .expect("failed to read file")
//!     .expect("invalid kson");
//! ```
//!
//! # Pulse resolution
//!
//! The `y` (pulse number) field has a resolution of **240 per beat** (960 per
//! 4/4 measure). Relative pulse numbers (`ry`) inside graph sections are
//! offsets from the section start.
//!
//! # Module layout
//!
//! | Submodule | Contents |
//! |-----------|----------|
//! | [`common`] | [`ByPulse`], [`ByMeasureIdx`], [`DefKeyValuePair`], graph primitives, [`TimeSig`] |
//! | [`meta`] | [`MetaInfo`], [`Difficulty`] |
//! | [`beat`] | [`BeatInfo`], [`GaugeInfo`] |
//! | [`note`] | [`NoteInfo`], [`ButtonNoteEntry`], [`LaserSection`] |
//! | [`audio`] | [`AudioInfo`] and all nested audio types |
//! | [`camera`] | [`CameraInfo`], [`TiltValue`], CAM graphs and patterns |
//! | [`bg`] | [`BGInfo`] and KSH-style background types |
//! | [`compat`] | [`EditorInfo`], [`CompatInfo`], [`KSHUnknownInfo`] |
//!
//! All public types are re-exported directly from this module.
//!
//! # References
//!
//! - KSON format specification (`docs/specs/from-elsewhere/kson_format.md`)
//! - [ksm-chart-format](https://github.com/kshootmania/ksm-chart-format)

use std::io;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

// ── Submodules ────────────────────────────────────────────────────────────────

pub mod audio;
pub mod beat;
pub mod bg;
pub mod camera;
pub mod common;
pub mod compat;
pub mod meta;
pub mod note;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use audio::*;
pub use beat::*;
pub use bg::*;
pub use camera::*;
pub use common::*;
pub use compat::*;
pub use meta::*;
pub use note::*;

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can occur when parsing a KSON file.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
	/// The JSON could not be parsed or a required field was missing.
	#[error("JSON parse error: {0}")]
	Json(#[from] serde_json::Error),

	/// The file's `format_version` is not supported.
	///
	/// Only `format_version: 1` (KSON 0.9.0 / 1.0.0) is supported.
	#[error("unsupported format_version: {0}")]
	UnsupportedFormatVersion(u64),
}

// ── Top-level chart ───────────────────────────────────────────────────────────

/// A fully-parsed `.kson` chart file.
#[derive(Debug, Clone, Deserialize)]
pub struct Kson {
	/// KSON format version number (`1` for KSON 0.9.0 / 1.0.0).
	pub format_version: u64,
	/// Chart metadata (title, artist, difficulty, level, …).
	pub meta: MetaInfo,
	/// Beat and timing data (BPM, time signatures, scroll speed).
	pub beat: BeatInfo,
	/// Gauge settings.
	#[serde(default)]
	pub gauge: Option<GaugeInfo>,
	/// Note data (BT, FX, and laser notes).
	#[serde(default)]
	pub note: Option<NoteInfo>,
	/// Audio data (BGM, key sounds, audio effects).
	#[serde(default)]
	pub audio: Option<AudioInfo>,
	/// Camera data (tilt, zoom, rotation, patterns).
	#[serde(default)]
	pub camera: Option<CameraInfo>,
	/// Background graphics data.
	#[serde(default)]
	pub bg: Option<BGInfo>,
	/// Editor-only data (OPTIONAL SUPPORT).
	#[serde(default)]
	pub editor: Option<EditorInfo>,
	/// KSH compatibility data (OPTIONAL SUPPORT).
	#[serde(default)]
	pub compat: Option<CompatInfo>,
	/// Client-specific data (OPTIONAL SUPPORT; free-form JSON).
	#[serde(rename = "impl", default)]
	pub impl_data: Option<Value>,
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// Parse a KSON chart from its raw UTF-8 bytes.
pub fn from_bytes(bytes: &[u8]) -> Result<Kson, LoadError> {
	let kson: Kson = serde_json::from_slice(bytes)?;
	if kson.format_version != 1 {
		return Err(LoadError::UnsupportedFormatVersion(kson.format_version));
	}
	Ok(kson)
}

/// Read a `.kson` file from disk and parse it.
///
/// The outer [`io::Error`] indicates a file-read failure; the inner
/// [`LoadError`] indicates a JSON parse or structural error — matching the
/// double-`Result` pattern used by other `rg_formats` entry points.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Result<Kson, LoadError>> {
	let bytes = std::fs::read(path)?;
	Ok(from_bytes(&bytes))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	// ── Helpers ───────────────────────────────────────────────────────────────

	fn kson_json(meta_extra: &str, beat_extra: &str, extra: &str) -> String {
		format!(
			r#"{{
				"format_version": 1,
				"meta": {{
					"title": "Test Song",
					"artist": "Test Artist",
					"chart_author": "Test Charter",
					"difficulty": 3,
					"level": 10,
					"disp_bpm": "180"
					{meta_extra}
				}},
				"beat": {{
					"bpm": [[0, 180.0]]
					{beat_extra}
				}}
				{extra}
			}}"#
		)
	}

	fn parse(src: &str) -> Kson {
		from_bytes(src.as_bytes()).expect("parse failed")
	}

	// ── ByPulse ───────────────────────────────────────────────────────────────

	#[test]
	fn by_pulse_parses_number_and_value() {
		let v: ByPulse<f64> = serde_json::from_str("[960, 180.0]").unwrap();
		assert_eq!(v.y, 960);
		assert!((v.v - 180.0).abs() < f64::EPSILON);
	}

	#[test]
	fn by_pulse_parses_string_value() {
		let v: ByPulse<String> = serde_json::from_str(r#"[480, "1/2"]"#).unwrap();
		assert_eq!(v.y, 480);
		assert_eq!(v.v, "1/2");
	}

	// ── ByMeasureIdx ──────────────────────────────────────────────────────────

	#[test]
	fn by_measure_idx_parses() {
		let v: ByMeasureIdx<TimeSig> = serde_json::from_str("[2, [3, 4]]").unwrap();
		assert_eq!(v.idx, 2);
		assert_eq!(v.v.n, 3);
		assert_eq!(v.v.d, 4);
	}

	// ── DefKeyValuePair ───────────────────────────────────────────────────────

	#[test]
	fn def_key_value_pair_parses() {
		let v: DefKeyValuePair<AudioEffectDef> =
			serde_json::from_str(r#"["myEffect", {"type": "flanger"}]"#).unwrap();
		assert_eq!(v.name, "myEffect");
		assert_eq!(v.v.effect_type, "flanger");
	}

	// ── GraphValueKind ────────────────────────────────────────────────────────

	#[test]
	fn graph_value_kind_single() {
		let v: GraphValueKind = serde_json::from_str("1.5").unwrap();
		assert!((v.v() - 1.5).abs() < f64::EPSILON);
		assert!((v.vf() - 1.5).abs() < f64::EPSILON);
	}

	#[test]
	fn graph_value_kind_pair() {
		let v: GraphValueKind = serde_json::from_str("[0.5, 0.8]").unwrap();
		assert!((v.v() - 0.5).abs() < f64::EPSILON);
		assert!((v.vf() - 0.8).abs() < f64::EPSILON);
	}

	// ── GraphPoint ────────────────────────────────────────────────────────────

	#[test]
	fn graph_point_two_elements() {
		let p: GraphPoint = serde_json::from_str("[960, 1.0]").unwrap();
		assert_eq!(p.y, 960);
		assert!((p.v.v() - 1.0).abs() < f64::EPSILON);
		assert_eq!(p.curve.0, [0.0, 0.0]);
	}

	#[test]
	fn graph_point_three_elements_with_curve() {
		let p: GraphPoint = serde_json::from_str("[480, 0.5, [0.3, 0.7]]").unwrap();
		assert_eq!(p.y, 480);
		assert!((p.v.v() - 0.5).abs() < f64::EPSILON);
		assert!((p.curve.0[0] - 0.3).abs() < f64::EPSILON);
		assert!((p.curve.0[1] - 0.7).abs() < f64::EPSILON);
	}

	#[test]
	fn graph_point_with_immediate_change() {
		let p: GraphPoint = serde_json::from_str("[0, [0.0, 1.0]]").unwrap();
		assert!((p.v.v() - 0.0).abs() < f64::EPSILON);
		assert!((p.v.vf() - 1.0).abs() < f64::EPSILON);
	}

	// ── GraphSectionPoint ─────────────────────────────────────────────────────

	#[test]
	fn graph_section_point_basic() {
		let p: GraphSectionPoint = serde_json::from_str("[0, 0.0]").unwrap();
		assert_eq!(p.ry, 0);
		assert!((p.v.v()).abs() < f64::EPSILON);
	}

	// ── TimeSig ───────────────────────────────────────────────────────────────

	#[test]
	fn time_sig_parses() {
		let t: TimeSig = serde_json::from_str("[3, 8]").unwrap();
		assert_eq!(t.n, 3);
		assert_eq!(t.d, 8);
	}

	// ── ButtonNoteEntry ───────────────────────────────────────────────────────

	#[test]
	fn button_note_chip() {
		let n: ButtonNoteEntry = serde_json::from_str("960").unwrap();
		assert_eq!(n.y(), 960);
		assert_eq!(n.length(), 0);
		assert!(matches!(n, ButtonNoteEntry::Chip(_)));
	}

	#[test]
	fn button_note_long() {
		let n: ButtonNoteEntry = serde_json::from_str("[480, 240]").unwrap();
		assert_eq!(n.y(), 480);
		assert_eq!(n.length(), 240);
		assert!(matches!(n, ButtonNoteEntry::Long(_)));
	}

	// ── LaserSection ──────────────────────────────────────────────────────────

	#[test]
	fn laser_section_two_elements_default_width() {
		let s: LaserSection = serde_json::from_str(r#"[0, [[0, 0.0], [480, 1.0]]]"#).unwrap();
		assert_eq!(s.y, 0);
		assert_eq!(s.v.len(), 2);
		assert_eq!(s.w, 1);
	}

	#[test]
	fn laser_section_three_elements_custom_width() {
		let s: LaserSection = serde_json::from_str(r#"[960, [[0, 0.5]], 2]"#).unwrap();
		assert_eq!(s.y, 960);
		assert_eq!(s.w, 2);
	}

	// ── Difficulty ────────────────────────────────────────────────────────────

	#[test]
	fn difficulty_index() {
		let d: Difficulty = serde_json::from_str("3").unwrap();
		assert!(matches!(d, Difficulty::Index(3)));
	}

	#[test]
	fn difficulty_name() {
		let d: Difficulty = serde_json::from_str(r#""gravity""#).unwrap();
		assert!(matches!(d, Difficulty::Name(ref s) if s == "gravity"));
	}

	// ── TiltValue ─────────────────────────────────────────────────────────────

	#[test]
	fn tilt_value_auto_string() {
		let t: TiltValue = serde_json::from_str(r#""normal""#).unwrap();
		assert!(matches!(t, TiltValue::Auto(ref s) if s == "normal"));
	}

	#[test]
	fn tilt_value_manual_float() {
		let t: TiltValue = serde_json::from_str("0.5").unwrap();
		assert!(matches!(t, TiltValue::Manual(v) if (v - 0.5).abs() < f64::EPSILON));
	}

	#[test]
	fn tilt_value_manual_immediate() {
		let t: TiltValue = serde_json::from_str("[0.5, 0.8]").unwrap();
		assert!(matches!(t, TiltValue::ManualImmediate { v, vf }
				if (v - 0.5).abs() < f64::EPSILON && (vf - 0.8).abs() < f64::EPSILON));
	}

	#[test]
	fn tilt_value_manual_to_auto() {
		let t: TiltValue = serde_json::from_str(r#"[0.8, "normal"]"#).unwrap();
		assert!(matches!(t, TiltValue::ManualToAuto { v, ref vf }
				if (v - 0.8).abs() < f64::EPSILON && vf == "normal"));
	}

	#[test]
	fn tilt_value_manual_with_curve() {
		let t: TiltValue = serde_json::from_str("[0.5, [0.3, 0.7]]").unwrap();
		assert!(matches!(t, TiltValue::ManualWithCurve { v, curve }
				if (v - 0.5).abs() < f64::EPSILON
				   && (curve[0] - 0.3).abs() < f64::EPSILON
				   && (curve[1] - 0.7).abs() < f64::EPSILON));
	}

	#[test]
	fn tilt_value_manual_immediate_with_curve() {
		let t: TiltValue = serde_json::from_str("[[0.5, 0.8], [0.3, 0.7]]").unwrap();
		assert!(
			matches!(t, TiltValue::ManualImmediateWithCurve { v, vf, curve }
			if (v - 0.5).abs() < f64::EPSILON
			   && (vf - 0.8).abs() < f64::EPSILON
			   && (curve[0] - 0.3).abs() < f64::EPSILON)
		);
	}

	// ── CamPatternInvokeSpin ──────────────────────────────────────────────────

	#[test]
	fn cam_pattern_spin_parses() {
		let s: CamPatternInvokeSpin = serde_json::from_str("[960, 1, 480]").unwrap();
		assert_eq!(s.y, 960);
		assert_eq!(s.direction, 1);
		assert_eq!(s.length, 480);
	}

	#[test]
	fn cam_pattern_spin_left_direction() {
		let s: CamPatternInvokeSpin = serde_json::from_str("[0, -1, 240]").unwrap();
		assert_eq!(s.direction, -1);
	}

	// ── CamPatternInvokeSwing ─────────────────────────────────────────────────

	#[test]
	fn cam_pattern_swing_three_elements() {
		let s: CamPatternInvokeSwing = serde_json::from_str("[480, -1, 240]").unwrap();
		assert_eq!(s.y, 480);
		assert_eq!(s.direction, -1);
		assert_eq!(s.length, 240);
		assert!(s.v.is_none());
	}

	#[test]
	fn cam_pattern_swing_four_elements() {
		let s: CamPatternInvokeSwing = serde_json::from_str(
			r#"[480, 1, 240, {"scale": 300.0, "repeat": 2, "decay_order": 1}]"#,
		)
		.unwrap();
		let v = s.v.unwrap();
		assert!((v.scale - 300.0).abs() < f64::EPSILON);
		assert_eq!(v.repeat, 2);
		assert_eq!(v.decay_order, 1);
	}

	// ── Top-level defaults ────────────────────────────────────────────────────

	#[test]
	fn minimal_kson_parses() {
		let chart = parse(&kson_json("", "", ""));
		assert_eq!(chart.format_version, 1);
		assert_eq!(chart.meta.title, "Test Song");
		assert_eq!(chart.meta.artist, "Test Artist");
		assert!(matches!(chart.meta.difficulty, Difficulty::Index(3)));
		assert_eq!(chart.meta.level, 10);
		assert_eq!(chart.meta.disp_bpm, "180");
	}

	#[test]
	fn beat_defaults_applied() {
		let chart = parse(&kson_json("", "", ""));
		assert_eq!(chart.beat.time_sig.len(), 1);
		assert_eq!(chart.beat.time_sig[0].v.n, 4);
		assert_eq!(chart.beat.time_sig[0].v.d, 4);
		assert_eq!(chart.beat.scroll_speed.len(), 1);
		assert!((chart.beat.scroll_speed[0].v.v() - 1.0).abs() < f64::EPSILON);
	}

	#[test]
	fn optional_sections_absent() {
		let chart = parse(&kson_json("", "", ""));
		assert!(chart.gauge.is_none());
		assert!(chart.note.is_none());
		assert!(chart.audio.is_none());
		assert!(chart.camera.is_none());
		assert!(chart.bg.is_none());
		assert!(chart.editor.is_none());
		assert!(chart.compat.is_none());
		assert!(chart.impl_data.is_none());
	}

	#[test]
	fn difficulty_name_variant() {
		let src = r#"{
			"format_version": 1,
			"meta": {
				"title": "T", "artist": "A", "chart_author": "C",
				"difficulty": "gravity", "level": 10, "disp_bpm": "180"
			},
			"beat": { "bpm": [[0, 180.0]] }
		}"#;
		let chart = parse(src);
		assert!(matches!(chart.meta.difficulty, Difficulty::Name(ref s) if s == "gravity"));
	}

	#[test]
	fn note_section_with_bt_lanes() {
		let chart = parse(&kson_json("", "", r#","note":{"bt":[[960],[1920],[],[]]}"#));
		let bt = chart.note.unwrap().bt.unwrap();
		assert_eq!(bt[0].len(), 1);
		assert_eq!(bt[0][0].y(), 960);
		assert!(bt[2].is_empty());
	}

	#[test]
	fn note_section_with_long_bt() {
		let chart = parse(&kson_json(
			"",
			"",
			r#","note":{"bt":[[[480,960]],[],[],[]]}"#,
		));
		let bt = chart.note.unwrap().bt.unwrap();
		assert_eq!(bt[0][0].y(), 480);
		assert_eq!(bt[0][0].length(), 960);
	}

	#[test]
	fn note_section_with_laser() {
		// laser: [left_lane, right_lane]; each lane is Vec<LaserSection>
		// LaserSection: [y, [GraphSectionPoint, ...], w?]
		// GraphSectionPoint: [ry, value]
		let chart = parse(&kson_json(
			"",
			"",
			r#","note":{"laser":[[],[[0, [[0, 0.0], [960, 1.0]]]]]}"#,
		));
		let laser = chart.note.unwrap().laser.unwrap();
		assert!(laser[0].is_empty());
		assert_eq!(laser[1].len(), 1);
		assert_eq!(laser[1][0].y, 0);
		assert_eq!(laser[1][0].v.len(), 2);
	}

	#[test]
	fn audio_bgm_defaults() {
		let chart = parse(&kson_json(
			"",
			"",
			r#","audio":{"bgm":{"filename":"song.ogg"}}"#,
		));
		let bgm = chart.audio.unwrap().bgm.unwrap();
		assert_eq!(bgm.filename.as_deref(), Some("song.ogg"));
		assert!((bgm.vol - 1.0).abs() < f64::EPSILON);
		assert_eq!(bgm.offset, 0);
	}

	#[test]
	fn bpm_sequence_parsed() {
		let src = r#"{
			"format_version": 1,
			"meta": {
				"title": "T", "artist": "A", "chart_author": "C",
				"difficulty": 3, "level": 10, "disp_bpm": "120-180"
			},
			"beat": { "bpm": [[0, 120.0], [960, 180.0]] }
		}"#;
		let chart = parse(src);
		assert_eq!(chart.beat.bpm.len(), 2);
		assert_eq!(chart.beat.bpm[0].y, 0);
		assert!((chart.beat.bpm[0].v - 120.0).abs() < f64::EPSILON);
		assert_eq!(chart.beat.bpm[1].y, 960);
		assert!((chart.beat.bpm[1].v - 180.0).abs() < f64::EPSILON);
	}

	#[test]
	fn invalid_json_returns_error() {
		assert!(from_bytes(b"not json at all").is_err());
	}

	#[test]
	fn missing_required_field_returns_error() {
		let json = r#"{"format_version":1,"beat":{"bpm":[[0,120.0]]}}"#;
		assert!(from_bytes(json.as_bytes()).is_err());
	}

	// ── Integration ───────────────────────────────────────────────────────────

	#[test]
	fn real_kson_frums() {
		let kson_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../../fixtures/charts/usc/frumschart/mxm.kson");
		let bytes = match std::fs::read(&kson_path) {
			Ok(b) => b,
			Err(_) => return, // not present in this build environment
		};
		let chart = from_bytes(&bytes).expect("real kson file should parse without error");
		assert!(!chart.meta.title.is_empty(), "title should be non-empty");
		assert!(!chart.meta.artist.is_empty(), "artist should be non-empty");
		assert!(
			chart.format_version >= 1,
			"format_version should be at least 1"
		);
		assert!(
			!chart.beat.bpm.is_empty(),
			"should have at least one BPM entry"
		);
	}
}
