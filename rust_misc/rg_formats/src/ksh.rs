//! Parser for the `.ksh` (K-Shoot MANIA) chart format.
//!
//! KSH is a text-based rhythm game chart format. A chart file has three
//! sections:
//!
//! - **Header**: `key=value` option lines before the first `--` bar line.
//! - **Body**: interleaved chart rows and option lines, separated by `--` bar
//!   lines (one per measure boundary).
//! - **Footer**: `#define_fx` and `#define_filter` definition lines (may appear
//!   anywhere after the first `--` in practice, but are conventionally placed
//!   at the end).
//!
//! # Encoding
//!
//! KSH files should be UTF-8 with BOM. [`from_bytes`] detects UTF-8 from the
//! presence of a BOM and uses Shift-JIS otherwise.
//!
//! # References
//!
//! - KSH format specification (`docs/specs/from-elsewhere/ksh_format.md`)

use std::io;
use std::path::Path;

use encoding_rs::SHIFT_JIS;

mod body;
mod chart;
mod definition;
mod header;
mod raw_headers;
mod row;

#[cfg(test)]
mod tests;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use body::{BodyOption, Measure, MeasureEvent, TiltValue};
pub use chart::Chart;
pub use definition::{Definition, DefinitionKind};
pub use header::{Difficulty, Header};
pub use raw_headers::{RawHeaders, RawHeadersIter};
pub use row::{BtNote, FxNote, LaneSpin, LaneSpinKind, LaserCell, Row};

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors that can occur when parsing a KSH file.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LoadError {
	/// The file has a UTF-8 BOM but its remaining bytes are not valid UTF-8.
	#[error("KSH file has a UTF-8 BOM but is not valid UTF-8")]
	InvalidUtf8,

	/// The file has no UTF-8 BOM and its bytes are not valid Shift-JIS.
	#[error("KSH file is not valid Shift-JIS")]
	InvalidShiftJis,

	/// No `--` bar line was found. A valid KSH file must have at least one.
	#[error("no bar line (--) found; not a valid KSH file")]
	NoBarLine,

	/// A chart line did not have exactly two `|` pipes or the expected field
	/// lengths (4 BT chars, 2 FX chars, ≥2 laser chars).
	#[error("invalid chart line at line {line}")]
	InvalidChartLine { line: usize },
}

// ── Entry points ─────────────────────────────────────────────────────────────

/// Parse a KSH chart from raw bytes.
///
/// Handles UTF-8 with a BOM and Shift-JIS (legacy ANSI encoding).
pub fn from_bytes(bytes: &[u8]) -> Result<Chart, LoadError> {
	Chart::parse(bytes)
}

/// Read a `.ksh` file from disk and parse it.
///
/// The outer [`io::Error`] indicates a file-read failure; the inner
/// [`LoadError`] indicates a format parse failure.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Result<Chart, LoadError>> {
	let bytes = std::fs::read(path)?;
	Ok(from_bytes(&bytes))
}

// ── Encoding ──────────────────────────────────────────────────────────────────

/// The encoding used by a KSH file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KshEncoding {
	/// UTF-8, identified by a leading UTF-8 BOM.
	Utf8,
	/// Shift-JIS, used when no UTF-8 BOM is present.
	ShiftJis,
}

impl KshEncoding {
	/// Detect the KSH encoding from the file's leading bytes.
	pub fn detect(bytes: &[u8]) -> Self {
		if bytes.starts_with(b"\xef\xbb\xbf") {
			Self::Utf8
		} else {
			Self::ShiftJis
		}
	}

	pub(super) fn decode(self, bytes: &[u8]) -> Result<String, LoadError> {
		match self {
			Self::Utf8 => {
				let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
				std::str::from_utf8(bytes)
					.map(str::to_owned)
					.map_err(|_| LoadError::InvalidUtf8)
			}
			Self::ShiftJis => {
				let (decoded, _, had_errors) = SHIFT_JIS.decode(bytes);
				if had_errors {
					Err(LoadError::InvalidShiftJis)
				} else {
					Ok(decoded.into_owned())
				}
			}
		}
	}
}

/// Decode KSH bytes using the encoding indicated by the file's BOM.
pub fn decode(bytes: &[u8]) -> Result<String, LoadError> {
	KshEncoding::detect(bytes).decode(bytes)
}

// ── Pure utilities ────────────────────────────────────────────────────────────
//
// Used in multiple places with no single owning type. Private, but accessible
// to child modules via `super::`.

fn parse_beat(s: &str) -> Option<(u32, u32)> {
	let (n, d) = s.split_once('/')?;
	Some((n.parse().ok()?, d.parse().ok()?))
}

fn parse_initial_bpm(s: &str) -> f64 {
	// Header BPM may be a range like "120-220"; take everything before the
	// first '-' that follows digits (spec says BPM is 0.001-65535).
	let first = s.split('-').next().unwrap_or(s);
	first.trim().parse().unwrap_or(0.0)
}
