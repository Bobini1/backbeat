//! Audio data types (`audio` object): BGM, key sounds, and audio effects.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::common::{ByPulse, DefKeyValuePair};

/// `{effect_name: {param: [[y, value], ...]}}` — parameter changes by pulse.
pub type AudioEffectParamChange = HashMap<String, HashMap<String, Vec<ByPulse<String>>>>;

// ── Default helpers ───────────────────────────────────────────────────────────

pub(super) fn one_f64() -> f64 {
	1.0
}

fn default_laser_vol() -> Vec<ByPulse<f64>> {
	vec![ByPulse { y: 0, v: 0.5 }]
}

fn default_preview_duration() -> u64 {
	15000
}

// ── Key sounds ────────────────────────────────────────────────────────────────

/// Volume for a chip FX key sound invocation.
#[derive(Debug, Clone, Deserialize)]
pub struct KeySoundInvokeFX {
	/// Key sound volume (default `1.0`).
	#[serde(default = "one_f64")]
	pub vol: f64,
}

/// A single entry in a key sound event lane: bare pulse (default volume) or
/// an explicit `[y, {vol}]` invocation.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeySoundInvokeEntry {
	/// Bare pulse number; volume uses the default (`1.0`).
	Pulse(u64),
	/// Explicit invocation with a specific volume.
	WithVol(ByPulse<KeySoundInvokeFX>),
}

/// Key sound events for chip FX notes (OPTIONAL SUPPORT).
///
/// Each named field is a 2-lane array `[left, right]` of per-lane event lists.
/// The `custom` field catches any additional entries keyed by audio filename.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeySoundInvokeListFX {
	/// Clap key sound events per lane.
	#[serde(default)]
	pub clap: Option<[Vec<KeySoundInvokeEntry>; 2]>,
	/// Impact clap key sound events per lane.
	#[serde(default)]
	pub clap_impact: Option<[Vec<KeySoundInvokeEntry>; 2]>,
	/// Punchy clap key sound events per lane.
	#[serde(default)]
	pub clap_punchy: Option<[Vec<KeySoundInvokeEntry>; 2]>,
	/// Snare key sound events per lane.
	#[serde(default)]
	pub snare: Option<[Vec<KeySoundInvokeEntry>; 2]>,
	/// Low snare key sound events per lane.
	#[serde(default)]
	pub snare_lo: Option<[Vec<KeySoundInvokeEntry>; 2]>,
	/// Custom key sounds keyed by audio filename (e.g. `"my_sound.ogg"`).
	#[serde(flatten)]
	pub custom: HashMap<String, Value>,
}

/// Key sound events for laser slam notes (OPTIONAL SUPPORT).
///
/// The `custom` field catches any additional entries keyed by audio filename.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeySoundInvokeListLaser {
	/// Pulse numbers of upward laser slams that trigger this sound.
	#[serde(default)]
	pub slam_up: Option<Vec<u64>>,
	/// Pulse numbers of downward laser slams that trigger this sound.
	#[serde(default)]
	pub slam_down: Option<Vec<u64>>,
	/// Pulse numbers of swing laser slams that trigger this sound.
	#[serde(default)]
	pub slam_swing: Option<Vec<u64>>,
	/// Pulse numbers of mute laser slams that trigger this sound.
	#[serde(default)]
	pub slam_mute: Option<Vec<u64>>,
	/// Custom key sounds keyed by audio filename.
	#[serde(flatten)]
	pub custom: HashMap<String, Value>,
}

/// Legacy laser key sound settings (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct KeySoundLaserLegacyInfo {
	/// Auto-reduce key sound volume based on laser slam width (`chokkakuautovol`).
	#[serde(default)]
	pub vol_auto: bool,
}

/// Key sound settings for laser notes.
#[derive(Debug, Clone, Deserialize)]
pub struct KeySoundLaserInfo {
	/// Laser slam volume changes. Default `[[0, 0.5]]`.
	#[serde(default = "default_laser_vol")]
	pub vol: Vec<ByPulse<f64>>,
	/// Key sound invocations by laser slam (OPTIONAL SUPPORT).
	#[serde(default)]
	pub slam_event: Option<KeySoundInvokeListLaser>,
	/// Legacy information (OPTIONAL SUPPORT).
	#[serde(default)]
	pub legacy: Option<KeySoundLaserLegacyInfo>,
}

/// Key sound settings for chip FX notes.
#[derive(Debug, Clone, Deserialize)]
pub struct KeySoundFXInfo {
	/// Key sound event lists for chip FX notes.
	pub chip_event: KeySoundInvokeListFX,
}

/// Key sound settings (`audio.key_sound` object).
#[derive(Debug, Clone, Deserialize)]
pub struct KeySoundInfo {
	/// Key sounds triggered by chip FX notes.
	#[serde(default)]
	pub fx: Option<KeySoundFXInfo>,
	/// Key sounds triggered by laser slam notes.
	#[serde(default)]
	pub laser: Option<KeySoundLaserInfo>,
}

// ── BGM ───────────────────────────────────────────────────────────────────────

/// Legacy BGM filenames with pre-rendered audio effects (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyBGMInfo {
	/// Prerendered BGM variant filenames (e.g. `["song_f.ogg"]`).
	#[serde(default)]
	pub fp_filenames: Vec<String>,
}

/// BGM audio preview settings.
#[derive(Debug, Clone, Deserialize)]
pub struct BGMPreviewInfo {
	/// Preview start offset in milliseconds (default `0`).
	#[serde(default)]
	pub offset: u64,
	/// Preview playback duration in milliseconds (default `15000`).
	#[serde(default = "default_preview_duration")]
	pub duration: u64,
}

/// Background music settings (`audio.bgm` object).
#[derive(Debug, Clone, Deserialize)]
pub struct BGMInfo {
	/// BGM audio filename.
	#[serde(default)]
	pub filename: Option<String>,
	/// BGM volume multiplier (default `1.0`).
	#[serde(default = "one_f64")]
	pub vol: f64,
	/// Audio start offset in milliseconds (default `0`; negative = delay).
	#[serde(default)]
	pub offset: i64,
	/// Preview playback settings.
	#[serde(default)]
	pub preview: Option<BGMPreviewInfo>,
	/// Legacy BGM filenames (OPTIONAL SUPPORT).
	#[serde(default)]
	pub legacy: Option<LegacyBGMInfo>,
}

// ── Audio effects ─────────────────────────────────────────────────────────────

/// An audio effect definition.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioEffectDef {
	/// Effect type name (e.g. `"flanger"`, `"retrigger"`, `"tapestop"`).
	#[serde(rename = "type")]
	pub effect_type: String,
	/// Effect parameter values as strings.
	#[serde(default)]
	pub v: Option<HashMap<String, String>>,
}

/// Audio effect settings for FX notes (`audio.audio_effect.fx`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AudioEffectFXInfo {
	/// Ordered audio effect definitions.
	#[serde(default)]
	pub def: Option<Vec<DefKeyValuePair<AudioEffectDef>>>,
	/// Parameter changes by pulse: `{effect_name: {param: [[y, value], ...]}}`.
	#[serde(default)]
	pub param_change: Option<AudioEffectParamChange>,
	/// Audio effect invocations by long note position (complex structure).
	///
	/// Stored as raw JSON to accommodate the variable-type entries
	/// `(uint | ByPulse<dictionary<string>>)[][2]`.
	#[serde(default)]
	pub long_event: Option<HashMap<String, Value>>,
}

/// Legacy laser audio effect settings (OPTIONAL SUPPORT).
#[derive(Debug, Clone, Deserialize)]
pub struct AudioEffectLaserLegacyInfo {
	/// Filter gain values (`pfiltergain`). Default `[[0, 0.5]]`.
	#[serde(default = "default_laser_vol")]
	pub filter_gain: Vec<ByPulse<f64>>,
}

/// Audio effect settings for laser notes (`audio.audio_effect.laser`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AudioEffectLaserInfo {
	/// Ordered audio effect definitions.
	#[serde(default)]
	pub def: Option<Vec<DefKeyValuePair<AudioEffectDef>>>,
	/// Parameter changes by pulse: `{effect_name: {param: [[y, value], ...]}}`.
	#[serde(default)]
	pub param_change: Option<AudioEffectParamChange>,
	/// Audio effect invocations by pulse: `{effect_name: [y, ...]}`.
	#[serde(default)]
	pub pulse_event: Option<HashMap<String, Vec<u64>>>,
	/// Peaking filter delay in milliseconds (OPTIONAL SUPPORT, default `0`).
	#[serde(default)]
	pub peaking_filter_delay: u64,
	/// Legacy information (OPTIONAL SUPPORT).
	#[serde(default)]
	pub legacy: Option<AudioEffectLaserLegacyInfo>,
}

/// Audio effect settings (`audio.audio_effect` object).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AudioEffectInfo {
	/// Effects applied to FX notes.
	#[serde(default)]
	pub fx: Option<AudioEffectFXInfo>,
	/// Effects applied to laser notes.
	#[serde(default)]
	pub laser: Option<AudioEffectLaserInfo>,
}

// ── AudioInfo ─────────────────────────────────────────────────────────────────

/// Audio data (`audio` object).
#[derive(Debug, Clone, Deserialize)]
pub struct AudioInfo {
	/// Background music settings.
	#[serde(default)]
	pub bgm: Option<BGMInfo>,
	/// Key sound settings.
	#[serde(default)]
	pub key_sound: Option<KeySoundInfo>,
	/// Audio effect settings.
	#[serde(default)]
	pub audio_effect: Option<AudioEffectInfo>,
}
