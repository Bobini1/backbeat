//! The `bundle_id` algorithm. Deterministically identify a `.bb` file.

use serde_with::{DeserializeFromStr, SerializeDisplay};

use super::Assets;
use crate::asset_id::AssetId;
use crate::{ChartFilename, Sha256};

/// A deterministic identifier for a `.bb` file. This ID includes the chart
/// and all of its dependencies.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct BundleId(pub Sha256);

impl From<Sha256> for BundleId {
	fn from(sha256: Sha256) -> Self {
		Self(sha256)
	}
}

/// We prefix bundle ids with `b-` to easily distinguish them from other sha256s
const BUNDLE_ID_PREFIX: &str = "b-";

impl std::fmt::Display for BundleId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{BUNDLE_ID_PREFIX}{}", self.0)
	}
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("expected a lowercase sha256 checksum with the prefix 'b-'")]
pub struct ParseBundleIdError;

impl std::str::FromStr for BundleId {
	type Err = ParseBundleIdError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let hex = s.strip_prefix(BUNDLE_ID_PREFIX).ok_or(ParseBundleIdError)?;
		Sha256::from_str(hex)
			.map(Self)
			.map_err(|_| ParseBundleIdError)
	}
}

impl BundleId {
	/// Compute a `BundleId`.
	pub fn compute(filename: &ChartFilename, chart_sha256: Sha256, assets: &Assets) -> Self {
		Self(Sha256::checksum_bytes(&encode_bundle_frame(
			filename,
			chart_sha256,
			assets,
		)))
	}
}

/// Magic prefix for the v1 bundle id framing. Bump this if the framing
/// scheme ever changes, so that old and new bundle ids can never collide.
const BUNDLE_FRAME_MAGIC_V1: &[u8] = b"b1";

/// Append a length-prefixed byte string to `buf`: a little-endian `u32`
/// length, followed by the bytes themselves.
fn write_field(buf: &mut Vec<u8>, bytes: &[u8]) {
	let len: u32 = bytes
		.len()
		.try_into()
		.expect("field too large to encode in a bundle frame");
	buf.extend_from_slice(&len.to_le_bytes());
	buf.extend_from_slice(bytes);
}

/// Append a sorted, length-prefixed list of `(filename, asset sha256)` pairs
/// to `buf`: a little-endian `u32` count, followed by each entry as
/// `write_field(filename) ++ sha256 bytes`.
///
/// Sorting by filename is what makes this deterministic despite the asset
/// map being backed by a `HashMap` (whose iteration order is otherwise
/// unspecified).
fn write_asset_map(buf: &mut Vec<u8>, assets: &Assets) {
	let mut entries: Vec<(&str, &AssetId)> = assets.iter().map(|(k, v)| (k.as_str(), v)).collect();
	entries.sort_unstable_by_key(|(filename, _)| *filename);

	let count: u32 = entries
		.len()
		.try_into()
		.expect("too many assets to encode in a bundle frame");
	buf.extend_from_slice(&count.to_le_bytes());

	for (filename, asset_id) in entries {
		write_field(buf, filename.as_bytes());
		buf.extend_from_slice(asset_id.0.as_bytes());
	}
}

/// Build the canonical byte framing that [`super::BackbeatFile::bundle_id`] hashes.
///
/// Layout (all integers little-endian `u32`):
///
/// ```text
/// b"b1"
/// u32 path_len            ++ path bytes (UTF-8)
/// 32 bytes                   chart_sha256 raw bytes
/// u32 asset_count
///   repeated, sorted by filename ascending:
///     u32 filename_len    ++ filename bytes (UTF-8)
///     32 bytes               asset sha256 raw bytes
/// ```
///
/// I designed this before I designed [`CombinedAssetsId`], which is why this code
/// replicates the combined assets id logic inline. By the time I'd made that decision,
/// I'd already ingested 1.5TiB of charts. Sorry, but we'll all live.
fn encode_bundle_frame(filename: &ChartFilename, chart_sha256: Sha256, assets: &Assets) -> Vec<u8> {
	let mut buf = Vec::new();

	buf.extend_from_slice(BUNDLE_FRAME_MAGIC_V1);
	write_field(&mut buf, filename.as_str().as_bytes());
	buf.extend_from_slice(chart_sha256.as_bytes());
	write_asset_map(&mut buf, assets);

	buf
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use crate::AssetPath;

	use super::*;

	fn example_chart_sha256() -> Sha256 {
		"87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"
			.parse()
			.unwrap()
	}

	fn other_sha256() -> Sha256 {
		"db5a1a5b0cdabec3e45d4c3613276f28c141d612cd7008560efc741c04cfb99f"
			.parse()
			.unwrap()
	}

	fn assets(entries: &[(&str, AssetId)]) -> Assets {
		entries
			.iter()
			.map(|&(path, asset_id)| (AssetPath::from_path(path).expect("illegal path"), asset_id))
			.collect()
	}

	fn fname(s: &str) -> ChartFilename {
		ChartFilename::from_path(s).unwrap()
	}

	#[test]
	fn bundle_id_deterministic_regardless_of_insertion_order() {
		let assets_a = assets(&[
			("a.png", AssetId(example_chart_sha256())),
			("b.png", AssetId(other_sha256())),
		]);
		// same entries, inserted in a different order: HashMap iteration
		// order is unspecified, so this exercises the sorting step.
		let assets_b = assets(&[
			("b.png", AssetId(other_sha256())),
			("a.png", AssetId(example_chart_sha256())),
		]);

		let frame_a = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_a);
		let frame_b = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_b);

		assert_eq!(frame_a, frame_b);
	}

	#[test]
	fn bundle_id_changes_with_path() {
		let assets = Assets::new();

		let frame_a = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets);
		let frame_b = encode_bundle_frame(&fname("other.bms"), example_chart_sha256(), &assets);

		assert_ne!(frame_a, frame_b);
	}

	#[test]
	fn bundle_id_changes_with_chart_sha256() {
		let assets = Assets::new();

		let frame_a = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets);
		let frame_b = encode_bundle_frame(&fname("file.bms"), other_sha256(), &assets);

		assert_ne!(frame_a, frame_b);
	}

	#[test]
	fn bundle_id_changes_with_asset_filename() {
		let assets_a = assets(&[("a.png", AssetId(other_sha256()))]);
		let assets_b = assets(&[("b.png", AssetId(other_sha256()))]);

		let frame_a = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_a);
		let frame_b = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_b);

		assert_ne!(frame_a, frame_b);
	}

	#[test]
	fn bundle_id_changes_with_asset_sha256() {
		let assets_a = assets(&[("a.png", AssetId(example_chart_sha256()))]);
		let assets_b = assets(&[("a.png", AssetId(other_sha256()))]);

		let frame_a = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_a);
		let frame_b = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &assets_b);

		assert_ne!(frame_a, frame_b);
	}

	#[test]
	fn bundle_frame_starts_with_version_magic() {
		let frame = encode_bundle_frame(&fname("file.bms"), example_chart_sha256(), &Assets::new());
		assert!(frame.starts_with(BUNDLE_FRAME_MAGIC_V1));
	}

	#[test]
	fn display_emits_b_prefix() {
		let id = BundleId::compute(&fname("file.bms"), example_chart_sha256(), &Assets::new());
		let s = id.to_string();
		assert_eq!(&s[..BUNDLE_ID_PREFIX.len()], BUNDLE_ID_PREFIX);
		let hex = &s[BUNDLE_ID_PREFIX.len()..];
		assert_eq!(hex.len(), 64);
		assert!(
			hex.bytes()
				.all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
		);
	}

	#[test]
	fn from_str_round_trips_display() {
		let id = BundleId::compute(&fname("file.bms"), example_chart_sha256(), &Assets::new());
		let parsed: BundleId = id.to_string().parse().unwrap();
		assert_eq!(parsed, id);
	}

	#[test]
	fn from_str_rejects_bare_hex() {
		// A bare 64-char hex sha256 is not a valid bundle id string; the
		// `b-` prefix is required.
		let bare = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7";
		assert!(bare.parse::<BundleId>().is_err());
	}

	#[test]
	fn from_str_rejects_wrong_prefix() {
		let s = format!(
			"c-{}",
			"87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"
		);
		assert!(s.parse::<BundleId>().is_err());
	}

	#[test]
	fn serde_json_uses_b_prefixed_display_form() {
		let id = BundleId::compute(&fname("file.bms"), example_chart_sha256(), &Assets::new());

		let json = serde_json::to_string(&id).unwrap();
		assert_eq!(json, format!("{:?}", id.to_string()));
		assert!(json.contains(BUNDLE_ID_PREFIX));

		let parsed: BundleId = serde_json::from_str(&json).unwrap();
		assert_eq!(parsed, id);
	}

	#[test]
	fn serde_json_rejects_bare_hex() {
		let bare_hex_json = format!(
			"\"{}\"",
			"87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"
		);
		assert!(serde_json::from_str::<BundleId>(&bare_hex_json).is_err());
	}

	#[test]
	fn serde_json_map_key_uses_b_prefix() {
		let id = BundleId::compute(&fname("file.bms"), example_chart_sha256(), &Assets::new());
		let map = HashMap::from([(id, "folder")]);

		let json = serde_json::to_string(&map).unwrap();
		assert!(json.contains(&id.to_string()));
	}
}
