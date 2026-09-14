//! Parsing of `.dtx` (DTXMania) chart files.
//!
//! DTX is structurally very similar to BMS: `#KEY: value` header lines for
//! metadata and resource tables, plus `#MMMCC: objectdata` chart rows that place
//! objects into a measure-and-tick timeline.
//!
//! ## Measure / tick model
//!
//! Each measure spans **384 ticks**. Chart rows distribute their objects evenly
//! across the measure: for `n` objects, object `i` lands at tick
//! `(384 * i) / n`. Bar-length multipliers (channel `02`) can stretch or shrink
//! a measure's real-time duration but do not change the tick representation
//! here.
//!
//! ## Encoding
//!
//! DTX files are **Shift-JIS** encoded. [`from_bytes`] tries UTF-8 first and
//! falls back to Shift-JIS automatically.
//!
//! ## Object encoding
//!
//! Objects in chart rows are two-character **base-36** pairs (`0`–`9`, `A`–`Z`,
//! case-insensitive), giving 1296 possible indices. The sole exception is
//! channel `03` (legacy inline BPM), which encodes the BPM value directly as a
//! **hex** number.
//!
//! ## Channel numbering
//!
//! Channels are two **hex** digits. Key drums: `0x11` hi-hat closed, `0x12`
//! snare, `0x13` kick, `0x14`–`0x17` toms, `0x18` hi-hat open, `0x19` ride,
//! `0x1A` left crash, `0x1B` hi-hat pedal, `0x1C` left kick. Guitar lanes:
//! `0x20`–`0x2F`. Bass lanes: `0xA0`–`0xAF`. System: `0x01` BGM, `0x02` bar
//! length, `0x03` BPM (hex), `0x08` BPMEx (base-36 index into `#BPMzz` table).

use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

use encoding_rs::SHIFT_JIS;

/// Number of ticks per measure at bar-length 1.0.
pub const TICKS_PER_MEASURE: u16 = 384;

/// A parsed DTX chart.
#[derive(Debug, Clone)]
pub struct Chart {
	/// Song metadata and resource tables.
	pub metadata: Metadata,
	/// All note/object events in chart order.
	pub events: Vec<Event>,
}

/// Metadata and resource tables parsed from DTX header lines.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
	/// `#TITLE`
	pub title: Option<String>,
	/// `#ARTIST`
	pub artist: Option<String>,
	/// `#BPM` / `#BPM00` — starting BPM.
	pub bpm: Option<f64>,
	/// `#PREVIEW` — preview audio filename.
	pub preview: Option<String>,
	/// `#PREIMAGE` — preview image filename.
	pub preimage: Option<String>,
	/// `#DLEVEL` / `#PLAYLEVEL` — drums difficulty (0–100, or /10 when ≥100).
	pub dlevel: Option<u32>,
	/// `#GLEVEL` — guitar difficulty.
	pub glevel: Option<u32>,
	/// `#BLEVEL` — bass difficulty.
	pub blevel: Option<u32>,
	/// `#WAVzz` → filename. Index is a base-36 pair (0–1295).
	pub wav: HashMap<u16, String>,
	/// `#BMPzz` → filename.
	pub bmp: HashMap<u16, String>,
	/// `#AVIzz` → filename.
	pub avi: HashMap<u16, String>,
	/// Named BPM table (`#BPMzz`). Used by channel `08` events.
	pub bpm_table: HashMap<u16, f64>,
	/// `#BGMWAV` — which WAV slot is the BGM track.
	pub bgmwav: Option<u16>,
}

/// A single object placed in the chart timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event {
	/// Measure number (0-based after the +1 offset DTXMania applies internally
	/// is skipped here — `#000` in the file → measure 0 in this struct).
	pub measure: u32,
	/// Tick within the measure (0..=383).
	pub tick: u16,
	/// Two hex-digit channel code, e.g. `0x13` for bass drum.
	pub channel: u8,
	/// Base-36 object index referencing a `#WAVzz` / `#BMPzz` / `#BPMzz`
	/// entry. For channel `0x03` (legacy BPM) this is a raw BPM value encoded
	/// as hex.
	pub value: u16,
}

/// Parse a DTX chart from its raw bytes.
///
/// The bytes are decoded from Shift-JIS (UTF-8 is tried first). Malformed or
/// unrecognised lines are silently ignored, matching the reference
/// implementation's behaviour.
pub fn from_bytes(bytes: &[u8]) -> Chart {
	let text = decode_dtx_text(bytes);
	parse_text(&text)
}

/// Read and parse a DTX chart from disk.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Chart> {
	let bytes = fs::read(path)?;
	Ok(from_bytes(&bytes))
}

// ---------------------------------------------------------------------------
// Internal parsing
// ---------------------------------------------------------------------------

fn decode_dtx_text(bytes: &[u8]) -> String {
	if let Ok(s) = std::str::from_utf8(bytes) {
		return s.to_owned();
	}
	let (cow, _enc, _errors) = SHIFT_JIS.decode(bytes);
	cow.into_owned()
}

fn parse_text(text: &str) -> Chart {
	let mut metadata = Metadata::default();
	let mut events: Vec<Event> = Vec::new();

	for raw_line in text.lines() {
		// Strip inline comments (`;` anywhere on the line).
		let line = match raw_line.find(';') {
			Some(pos) => &raw_line[..pos],
			None => raw_line,
		};
		let line = line.trim();

		if !line.starts_with('#') {
			continue;
		}

		// Split `#COMMAND: value` or `#COMMAND value`.
		let without_hash = &line[1..];

		// Find the split point: first `:` or first whitespace.
		let (command, value) =
			if let Some(pos) = without_hash.find(|c: char| c == ':' || c.is_ascii_whitespace()) {
				let cmd = &without_hash[..pos];
				let rest = without_hash[pos..]
					.trim_start_matches(|c: char| c == ':' || c.is_ascii_whitespace());
				(cmd, rest.trim())
			} else {
				// No separator — command only, no value.
				(without_hash, "")
			};

		if command.is_empty() {
			continue;
		}

		// Detect chart rows: exactly 5 ASCII alphanumeric chars where:
		// - char 0 is a base-36 digit (measure hundreds)
		// - chars 1-2 are decimal digits 0-9 (measure tens/ones — DTXManiaNX format)
		// - chars 3-4 are valid hex digits (channel)
		// This correctly rejects headers like WAV01 (A and V are not decimal digits
		// in positions 1-2) and BPM01 (P and M are not decimal digits).
		let cmd_bytes = command.as_bytes();
		if cmd_bytes.len() == 5
			&& cmd_bytes.iter().all(|b| b.is_ascii_alphanumeric())
			&& from_base36_char(cmd_bytes[0]).is_some()
			&& cmd_bytes[1].is_ascii_digit()
			&& cmd_bytes[2].is_ascii_digit()
			&& decode_hex_pair(cmd_bytes[3], cmd_bytes[4]).is_some()
		{
			let measure = decode_measure_number(cmd_bytes);
			let channel = decode_hex_pair(cmd_bytes[3], cmd_bytes[4]).unwrap();
			parse_chart_row(measure, channel, value.as_bytes(), &mut events);
		} else {
			parse_header(command, value, &mut metadata);
		}
	}

	events.sort_unstable();
	Chart { metadata, events }
}

fn parse_chart_row(measure: u32, channel: u8, data: &[u8], events: &mut Vec<Event>) {
	// Skip data that is not even-length (last byte is dropped like DTXManiaNX).
	if data.len() < 2 {
		return;
	}
	let n_objects = data.len() / 2;

	for i in 0..n_objects {
		let hi = data[i * 2];
		let lo = data[i * 2 + 1];

		// Skip whitespace/underscores used as padding.
		if hi == b'_' || hi == b' ' || lo == b'_' || lo == b' ' {
			continue;
		}

		// Channel 03 (legacy BPM): values are plain hex, not base-36.
		let value = if channel == 0x03 {
			match decode_hex_pair(hi, lo) {
				Some(v) => v as u16,
				None => continue,
			}
		} else {
			match decode_base36_pair(hi, lo) {
				Some(v) => v,
				None => continue,
			}
		};

		// Object 00 means "no chip" in both encodings.
		if value == 0 {
			continue;
		}

		let tick = ((TICKS_PER_MEASURE as usize * i) / n_objects) as u16;
		events.push(Event {
			measure,
			tick,
			channel,
			value,
		});
	}
}

fn parse_header(command: &str, value: &str, meta: &mut Metadata) {
	let cmd_upper = command.to_ascii_uppercase();
	let cmd = cmd_upper.as_str();

	// Indexed resource tables: WAVzz, BMPzz, AVIzz, BPMzz — 3-char prefix + 2 base-36 chars.
	if cmd.len() == 5 {
		let prefix = &cmd[..3];
		let idx_bytes = &cmd.as_bytes()[3..];
		if let Some(idx) = decode_base36_pair(idx_bytes[0], idx_bytes[1]) {
			match prefix {
				"WAV" => {
					meta.wav.insert(idx, value.to_owned());
					return;
				}
				"BMP" => {
					meta.bmp.insert(idx, value.to_owned());
					return;
				}
				"AVI" => {
					meta.avi.insert(idx, value.to_owned());
					return;
				}
				"BPM" => {
					if let Ok(bpm) = value.trim().parse::<f64>() {
						meta.bpm_table.insert(idx, bpm);
						// BPM00 is also the starting BPM.
						if idx == 0 {
							meta.bpm = Some(bpm);
						}
					}
					return;
				}
				_ => {}
			}
		}
	}

	// VOLUME/PAN/SIZE are per-WAV modifiers; we don't store them.
	if matches!(&cmd[..cmd.len().min(6)], "VOLUME" | "SIZE__" | "PAN___") {
		return;
	}
	// Editor-specific tags.
	if cmd.starts_with("DTXC_") {
		return;
	}
	// Conditional flow — ignored (we parse unconditionally).
	if matches!(cmd, "RANDOM" | "IF" | "ENDIF" | "IFDEF" | "IFNDEF" | "ELSE") {
		return;
	}

	match cmd {
		"TITLE" => meta.title = Some(value.to_owned()),
		"ARTIST" => meta.artist = Some(value.to_owned()),
		"BPM" => {
			if let Ok(bpm) = value.trim().parse::<f64>() {
				meta.bpm = Some(bpm);
				meta.bpm_table.insert(0, bpm);
			}
		}
		"PREVIEW" => meta.preview = Some(value.to_owned()),
		"PREIMAGE" => meta.preimage = Some(value.to_owned()),
		"DLEVEL" | "PLAYLEVEL" => {
			if let Ok(v) = value.trim().parse::<u32>() {
				meta.dlevel = Some(v);
			}
		}
		"GLEVEL" => {
			if let Ok(v) = value.trim().parse::<u32>() {
				meta.glevel = Some(v);
			}
		}
		"BLEVEL" => {
			if let Ok(v) = value.trim().parse::<u32>() {
				meta.blevel = Some(v);
			}
		}
		"BGMWAV" => {
			let v_bytes = value.trim().as_bytes();
			if v_bytes.len() >= 2 {
				meta.bgmwav = decode_base36_pair(v_bytes[0], v_bytes[1]);
			}
		}
		_ => {
			// Unrecognised header; ignore.
		}
	}
}

// ---------------------------------------------------------------------------
// Codec helpers
// ---------------------------------------------------------------------------

fn from_base36_char(c: u8) -> Option<u16> {
	match c {
		b'0'..=b'9' => Some((c - b'0') as u16),
		b'A'..=b'Z' => Some((c - b'A' + 10) as u16),
		b'a'..=b'z' => Some((c - b'a' + 10) as u16),
		_ => None,
	}
}

fn decode_base36_pair(hi: u8, lo: u8) -> Option<u16> {
	Some(from_base36_char(hi)? * 36 + from_base36_char(lo)?)
}

/// Decode a 3-byte measure number: first byte is base-36 (0–35), last two are
/// decimal (0–9 each). Range: 0 (`000`) to 3599 (`Z99`).
fn decode_measure_number(cmd: &[u8]) -> u32 {
	let hundreds = from_base36_char(cmd[0]).unwrap_or(0) as u32;
	let tens = (cmd[1] - b'0') as u32;
	let ones = (cmd[2] - b'0') as u32;
	hundreds * 100 + tens * 10 + ones
}

fn decode_hex_pair(hi: u8, lo: u8) -> Option<u8> {
	let hi_val = match hi {
		b'0'..=b'9' => hi - b'0',
		b'A'..=b'F' => hi - b'A' + 10,
		b'a'..=b'f' => hi - b'a' + 10,
		_ => return None,
	};
	let lo_val = match lo {
		b'0'..=b'9' => lo - b'0',
		b'A'..=b'F' => lo - b'A' + 10,
		b'a'..=b'f' => lo - b'a' + 10,
		_ => return None,
	};
	Some(hi_val * 16 + lo_val)
}
