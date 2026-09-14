//! Parsing of `.tja` (TJAPlayer3) chart files.
//!
//! TJA is a text-based Taiko chart format. One file can contain up to seven
//! difficulty "COURSE" blocks (Easy/Normal/Hard/Oni/Edit/Tower/Dan), each with
//! its own `#START` / `#END` note rows.
//!
//! ## Encoding
//!
//! TJA files are **Shift-JIS** encoded. [`from_bytes`] always decodes as
//! Shift-JIS (no UTF-8 probe), exactly matching TJAPlayer3's `StreamReader`
//! with `Encoding.GetEncoding("Shift_JIS")`.
//!
//! ## Preprocessing pipeline
//!
//! Before parsing, the text goes through TJAPlayer3's `t入力_V4` pipeline:
//!
//! 1. Normalise `\r\n` → `\n`, `\t` → space; append trailing `\n`.
//! 2. Comma-line fix: multiline `^,` → `0,`.
//! 3. Split at the first `#START`; strip non-whitelisted lines from the header
//!    portion; rejoin.
//! 4. Line-split (dropping empty entries); strip `" *//.*"` comment suffix
//!    from each line; drop lines that are now empty.
//!
//! ## COURSE splitting
//!
//! The preprocessed text is split on the **literal string `"COURSE:"`**
//! (case-sensitive, not a regex). Each segment beyond the first is a
//! difficulty block. If no `"COURSE:"` appears, the whole file is treated as
//! Oni (index 3).
//!
//! ## Measure / tick model
//!
//! Each measure spans [`TICKS_PER_MEASURE`] = 384 ticks. Note at position `i`
//! in a measure row of width `w` lands at tick `measure * 384 + 384 * i / w`.

use std::path::Path;
use std::{fs, io};

use encoding_rs::SHIFT_JIS;

mod header;
mod notes;
mod parse;
mod preprocess;
mod types;

pub use types::*;

/// Ticks per measure (384, matching TJAPlayer3's `n小節の解像度`).
pub const TICKS_PER_MEASURE: u32 = 384;

/// Default BPM when no `BPM:` header is present.
pub const DEFAULT_BPM: f64 = 120.0;

/// Parse a TJA chart from its raw bytes.
///
/// Always decodes as Shift-JIS (no UTF-8 probe). Malformed or unrecognised
/// lines are silently skipped, matching TJAPlayer3's lenient behaviour.
pub fn from_bytes(bytes: &[u8]) -> Chart {
	let text = decode_tja_text(bytes);
	parse::parse_text(&text)
}

/// Read and parse a TJA chart from disk.
pub fn from_file(path: impl AsRef<Path>) -> io::Result<Chart> {
	let bytes = fs::read(path)?;
	Ok(from_bytes(&bytes))
}

/// Decode TJA bytes as Shift-JIS.
///
/// Exact CDTX.cs behaviour: `StreamReader(path, Encoding.GetEncoding("Shift_JIS"))`.
/// No UTF-8 fallback.
fn decode_tja_text(bytes: &[u8]) -> String {
	let (cow, _enc, _errors) = SHIFT_JIS.decode(bytes);
	cow.into_owned()
}
