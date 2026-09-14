//! MD5-based chart ID.
//!
//! Used by the BMS family (`.bms`, `.bme`, `.bml`, `.pms`) and BMSON.

use md5::{Digest as _, Md5};

/// Compute the MD5 hash of `chart_bytes` and return it as a lowercase hex string.
pub(crate) fn compute(chart_bytes: &[u8]) -> String {
	let hash = Md5::digest(chart_bytes);
	format!("{hash:x}")
}
