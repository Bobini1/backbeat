//! The `.bb` (Backbeat) file format.
//!
//! A Backbeat file wraps exactly one rhythm game chart. It stores the raw chart
//! bytes (base64-gzipped), the list of associated assets keyed by their filenames,
//! and the filename of the chart itself.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::path::Path;
use std::str::FromStr;

use base64_simd::STANDARD as BASE64;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

mod bundle_id;
mod chart_filename;
mod combined_assets_id;

pub use bundle_id::BundleId;
pub use chart_filename::{ChartFilename, ChartFilenameError};
pub use combined_assets_id::CombinedAssetsId;

use crate::asset_id::AssetId;
use crate::{AssetPath, ChartId, IdAlgorithm, Sha256};

/// A map of asset paths to asset ids.
pub type Assets = HashMap<AssetPath, AssetId>;

/// A Backbeat file (`.bb`). This is a packaging format for exactly one chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackbeatFile {
	/// The filename of the chart this package contains. This is packager-set
	/// metadata, and most importantly has the chart's file extension. For
	/// example: `"0x1311.sm"`.
	pub filename: ChartFilename,

	/// Assets for this chart, keyed by chart-relative filename.
	pub assets: Assets,

	/// A description for this chart, so that users can see what it is, even if it's not installed.
	///
	/// This is **NOT** part of the BundleID algorithm.
	///
	/// This is a mandatory part of the packaging process - the person packaging a chart already needs
	/// to know how to parse the chart to figure out its dependencies, so they should also be able
	/// to write a convincing description for it.
	///
	/// These are usually in the form "Artist - Title (Charter CHART DIFFICULTY)".
	pub desc: ChartDesc,

	/// The packaged chart's bytes, stored as base64-encoded gzip.
	pub chart: ChartData,
}

impl BackbeatFile {
	pub const EXTENSION: &str = "bb";

	pub fn from_file(path: impl AsRef<Path>) -> Result<Self, BackbeatFileError> {
		let bytes = crate::fs::read(path)?;
		Self::from_json(&bytes)
	}

	/// Parse a `.bb` file from its JSON bytes.
	///
	/// Enforces that `chart` is valid base64, gzip, and within the chart size
	/// limit.
	pub fn from_json(bytes: &[u8]) -> Result<Self, BackbeatFileError> {
		#[derive(Deserialize)]
		#[serde(deny_unknown_fields)]
		struct Wire {
			filename: ChartFilename,
			assets: Assets,
			chart: String,
			desc: String,
		}

		// this is entirely to preserve the notbase64/notgzip errors
		let wire: Wire = serde_json::from_slice(bytes)?;
		let chart = ChartData::from_base64(&wire.chart)?;
		let desc = ChartDesc::new(&wire.desc)?;

		Ok(Self {
			filename: wire.filename,
			assets: wire.assets,
			desc,
			chart,
		})
	}

	/// Serialize this file to JSON bytes.
	pub fn to_json(&self) -> Vec<u8> {
		serde_json::to_vec(self).expect("must ser")
	}

	pub fn chart_sha256(&self) -> Sha256 {
		Sha256::checksum_bytes(&self.chart.decompress())
	}

	/// Compute this bundle's [`BundleId`]: a deterministic identifier for "this
	/// chart, with this filename, and exactly this set of dependencies".
	pub fn bundle_id(&self) -> BundleId {
		BundleId::compute(&self.filename, self.chart_sha256(), &self.assets)
	}

	/// Compute this bundle's [`CombinedAssetsId`]: a deterministic identifier for the
	/// exact set of assets this bundle makes reference to.
	pub fn combined_assets_id(&self) -> CombinedAssetsId {
		CombinedAssetsId::compute(&self.assets)
	}

	/// Evaluate a referenced path using this bundle, getting what asset it corresponds
	/// to.
	///
	/// This applies some windows-like path resolution on the assets, for cross-OS
	/// compatibility.
	pub fn resolve_path(&self, path: &str) -> Option<AssetId> {
		let path = AssetPath::from_path(path).ok()?;
		self.assets.get(&path).copied().or_else(|| {
			self.assets
				.iter()
				.find(|(asset_path, _)| asset_path.as_str().eq_ignore_ascii_case(path.as_str()))
				.map(|(_, asset_id)| *asset_id)
		})
	}

	/// Calculate any additional chart IDs that are valid for this bundle.
	pub fn additional_chart_ids(&self) -> Vec<ChartId> {
		let mut out = vec![];
		let bytes = self.chart.decompress();
		for id in self.filename.id_algorithms() {
			match id.compute(&bytes, &self.filename) {
				Ok(val) => out.push(ChartId {
					alg: IdAlgorithm::Custom(id.into_custom()),
					val,
				}),
				Err(err) => {
					// i can't stop it
					tracing::warn!("Could not calculate chart ID for {id}: {err}");
				}
			}
		}

		out
	}
}

/// The file payload stored inside a `.bb` file.
///
/// On the wire this is a base64-encoded gzip stream. Use [`ChartData::decompress`]
/// to recover the original chart bytes.
///
/// It is critical that consumers treat this as opaque binary data and not as a
/// string - the wrapping exists precisely to enforce that!
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartData(Vec<u8>);

impl ChartData {
	/// Maximum size of a chart after decompression.
	pub const MAX_CHART_SIZE: u64 = 1024 * 1024 * 1024;

	/// Gzip-compress `bytes` and store the result.
	///
	/// This function fails if amount of bytes you're trying to compress exceeds [`ChartData::MAX_CHART_SIZE`].
	pub fn compress(bytes: &[u8]) -> Result<Self, BackbeatFileError> {
		Self::check_size(bytes.len() as u64)?;
		let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
		encoder.write_all(bytes)?;
		Ok(Self(encoder.finish()?))
	}

	/// Decode a base64-encoded compressed chart payload.
	pub fn from_base64(s: &str) -> Result<Self, BackbeatFileError> {
		let bytes = BASE64
			.decode_to_vec(s.as_bytes())
			.map_err(|_| BackbeatFileError::ChartNotBase64)?;
		Self::from_compressed(bytes)
	}

	/// Decompress and return the original chart bytes.
	pub fn decompress(&self) -> Vec<u8> {
		let mut decoder = GzDecoder::new(self.0.as_slice());
		let mut out = Vec::new();
		decoder
			.read_to_end(&mut out)
			.expect("ChartData must contain valid gzip");
		out
	}

	/// View the compressed bytes without decompressing.
	pub fn as_compressed(&self) -> &[u8] {
		&self.0
	}

	/// Validate and wrap already-compressed bytes without re-compressing.
	pub fn from_compressed(bytes: Vec<u8>) -> Result<Self, BackbeatFileError> {
		let chart = Self(bytes);
		chart.validate_compressed()?;
		Ok(chart)
	}

	fn validate_compressed(&self) -> Result<(), BackbeatFileError> {
		let mut decoder = GzDecoder::new(self.0.as_slice());
		let decompressed_size = io::copy(
			&mut decoder.by_ref().take(Self::MAX_CHART_SIZE + 1),
			&mut io::sink(),
		)
		.map_err(BackbeatFileError::ChartNotGzip)?;
		Self::check_size(decompressed_size)
	}

	const fn check_size(size: u64) -> Result<(), BackbeatFileError> {
		if size > Self::MAX_CHART_SIZE {
			return Err(BackbeatFileError::ChartTooLarge {
				max_bytes: Self::MAX_CHART_SIZE,
			});
		}
		Ok(())
	}
}

impl Serialize for ChartData {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&BASE64.encode_to_string(&self.0))
	}
}

impl<'de> Deserialize<'de> for ChartData {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Self::from_base64(&s).map_err(serde::de::Error::custom)
	}
}

/// A chart description is a string that is capped at 10,000 bytes.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
pub struct ChartDesc(String);

impl ChartDesc {
	pub const MAX_LENGTH: usize = 10_000;

	pub fn new(value: &str) -> Result<Self, BackbeatFileError> {
		if value.len() > Self::MAX_LENGTH {
			return Err(BackbeatFileError::DescTooLarge);
		}

		Ok(Self(value.to_owned()))
	}

	pub fn new_truncate(value: &str) -> Self {
		let mut value = value.to_owned();
		if value.len() > Self::MAX_LENGTH {
			value.truncate(value.floor_char_boundary(Self::MAX_LENGTH));
		}

		Self(value)
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn into_string(self) -> String {
		self.0
	}
}

impl fmt::Display for ChartDesc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl FromStr for ChartDesc {
	type Err = BackbeatFileError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		Self::new(value)
	}
}

/// Errors that can occur when reading or writing a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum BackbeatFileError {
	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("chart payload is not valid base64")]
	ChartNotBase64,

	#[error("decompressed chart exceeds the maximum size of {max_bytes} bytes")]
	ChartTooLarge { max_bytes: u64 },

	#[error("chart payload is not valid gzip: {0}")]
	ChartNotGzip(#[source] io::Error),

	#[error("A chart description cannot be longer than 10000 bytes.")]
	DescTooLarge,

	#[error("compression error: {0}")]
	Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Sha256;

	fn example_filename() -> ChartFilename {
		ChartFilename::from_path("file.bms").unwrap()
	}

	fn example_desc() -> ChartDesc {
		ChartDesc::new("Artist - Title (zk's Example)").unwrap()
	}

	fn example_assets() -> Assets {
		let sha = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"
			.parse::<Sha256>()
			.unwrap();
		let mut assets = HashMap::new();
		assets.insert(AssetPath::from_path("song.ogg").unwrap(), AssetId(sha));
		assets
	}

	#[test]
	fn chart_description_accepts_10_000_bytes() {
		let value = "e".repeat(ChartDesc::MAX_LENGTH);
		let description = ChartDesc::new(&value).unwrap();

		assert_eq!(value.len(), ChartDesc::MAX_LENGTH);
		assert_eq!(description.as_str(), value);
		assert_eq!(description.into_string(), value);
	}

	#[test]
	fn chart_description_rejects_more_than_10_000_bytes() {
		let value = format!("{}a", "e".repeat(ChartDesc::MAX_LENGTH));

		assert!(matches!(
			ChartDesc::new(&value),
			Err(BackbeatFileError::DescTooLarge)
		));
	}

	#[test]
	fn chart_description_serde_roundtrips_as_a_string() {
		let description = ChartDesc::new("Artist - Title").unwrap();
		let json = serde_json::to_string(&description).unwrap();

		assert_eq!(json, r#""Artist - Title""#);
		assert_eq!(
			serde_json::from_str::<ChartDesc>(&json).unwrap(),
			description
		);
	}

	#[test]
	fn chart_data_roundtrip() {
		let original = b"this is some chart data\nwith multiple lines";
		let compressed = ChartData::compress(original).unwrap();

		let json = serde_json::to_string(&compressed).unwrap();
		assert!(json.starts_with('"'));

		let decoded: ChartData = serde_json::from_str(&json).unwrap();
		assert_eq!(decoded.decompress(), original);
	}

	#[test]
	fn parses_zero_byte_chart_fixture() {
		let bytes = fixture_bytes!("bb/zero-byte-chart.bb");
		let file = BackbeatFile::from_json(bytes).expect("parse zero-byte-chart.bb");

		assert!(file.chart.decompress().is_empty());
		assert_eq!(
			file.chart_sha256().to_string(),
			"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
		);
		assert_eq!(
			BackbeatFile::from_json(&file.to_json())
				.unwrap()
				.chart
				.decompress(),
			b""
		);
	}

	#[test]
	fn chart_data_rejects_oversized_compressed_payload() {
		let chunk = [0; 1024 * 1024];
		let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
		for _ in 0..=1024 {
			encoder.write_all(&chunk).unwrap();
		}

		assert!(matches!(
			ChartData::from_compressed(encoder.finish().unwrap()),
			Err(BackbeatFileError::ChartTooLarge { max_bytes })
				if max_bytes == ChartData::MAX_CHART_SIZE
		));
	}

	#[test]
	fn chart_data_rejects_invalid_compressed_bytes() {
		assert!(matches!(
			ChartData::from_compressed(b"not-gzip".to_vec()),
			Err(BackbeatFileError::ChartNotGzip(_))
		));
	}

	#[test]
	fn bb_file_roundtrip() {
		let file = BackbeatFile {
			filename: example_filename(),
			assets: example_assets(),
			desc: example_desc(),
			chart: ChartData::compress(b"#TITLE:Test;\n").unwrap(),
		};

		let json = file.to_json();
		let parsed = BackbeatFile::from_json(&json).unwrap();

		assert_eq!(file, parsed);
		assert_eq!(parsed.chart.decompress(), b"#TITLE:Test;\n");
	}

	#[test]
	fn resolves_asset_paths_with_windows_like_semantics() {
		let asset_id = AssetId(Sha256::checksum_bytes(b"asset"));
		let file = BackbeatFile {
			filename: example_filename(),
			assets: HashMap::from([(AssetPath::from_path("Audio/Song.ogg").unwrap(), asset_id)]),
			desc: example_desc(),
			chart: ChartData::compress(b"chart").unwrap(),
		};

		assert_eq!(file.resolve_path("Audio/Song.ogg"), Some(asset_id));
		assert_eq!(file.resolve_path(r"audio\song.OGG"), Some(asset_id));
		assert_eq!(file.resolve_path("  AUDIO/SONG.OGG  "), Some(asset_id));
		assert_eq!(file.resolve_path("Audio/Missing.ogg"), None);
	}

	#[test]
	fn filename_on_wire() {
		let file = BackbeatFile {
			filename: example_filename(),
			assets: example_assets(),
			desc: example_desc(),
			chart: ChartData::compress(b"data").unwrap(),
		};
		let json = String::from_utf8(file.to_json()).unwrap();

		assert!(json.contains(r#""filename":"file.bms""#));
		assert!(json.contains("87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"));
	}

	#[test]
	fn from_json_rejects_extra_fields() {
		let chart = ChartData::compress(b"chart").unwrap();
		let chart = BASE64.encode_to_string(chart.as_compressed());
		let json = format!(
			r#"{{"filename":"song.sm","assets":{{}},"desc":"Test chart","chart":"{chart}","extra":true}}"#
		);

		assert!(matches!(
			BackbeatFile::from_json(json.as_bytes()),
			Err(BackbeatFileError::Json(_))
		));
	}

	#[test]
	fn from_json_rejects_invalid_base64() {
		let json = br#"{"filename":"song.sm","assets":{},"desc":"test","chart":"@@@"}"#;
		assert!(matches!(
			BackbeatFile::from_json(json),
			Err(BackbeatFileError::ChartNotBase64)
		));
	}

	#[test]
	fn from_json_accepts_unrecognised_filename() {
		let chart = ChartData::compress(b"chart").unwrap();
		let chart = BASE64.encode_to_string(chart.as_compressed());
		let json =
			format!(r#"{{"filename":"chart.txt","assets":{{}},"desc":"test","chart":"{chart}"}}"#);

		let file = BackbeatFile::from_json(json.as_bytes()).unwrap();
		assert_eq!(file.filename.as_str(), "chart.txt");
		assert_eq!(file.chart.decompress(), b"chart");
	}

	#[test]
	fn from_json_rejects_invalid_asset_paths() {
		let chart = ChartData::compress(b"chart").unwrap();
		let chart = BASE64.encode_to_string(chart.as_compressed());
		let asset_id = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7";

		for asset_path in [
			" song.ogg",
			"song.ogg ",
			r"folder\song.ogg",
			"/song.ogg",
			"folder/",
		] {
			let asset_path = serde_json::to_string(asset_path).unwrap();
			let json = format!(
				r#"{{"filename":"file.bms","assets":{{{asset_path}:"{asset_id}"}},"desc":"test","chart":"{chart}"}}"#
			);

			assert!(
				matches!(
					BackbeatFile::from_json(json.as_bytes()),
					Err(BackbeatFileError::Json(_))
				),
				"asset path {asset_path} should be rejected"
			);
		}
	}

	#[test]
	fn from_json_rejects_invalid_gzip() {
		let b64 = BASE64.encode_to_string(b"not-gzip");
		let json =
			format!(r#"{{"filename":"song.sm","assets":{{}},"desc":"test","chart":"{b64}"}}"#);
		assert!(matches!(
			BackbeatFile::from_json(json.as_bytes()),
			Err(BackbeatFileError::ChartNotGzip(_))
		));
	}

	#[test]
	fn parses_example_bb_fixture() {
		let bytes = fixture_bytes!("bb/0x1311.bb");
		let file = BackbeatFile::from_json(bytes).expect("parse 0x1311.bb");
		assert_eq!(
			file.filename,
			ChartFilename::from_path("0x1311.sm").unwrap()
		);
	}

	#[test]
	fn bundle_id_stable_for_fixture() {
		let bytes = fixture_bytes!("bb/0x1311.bb");
		let file = BackbeatFile::from_json(bytes).expect("parse 0x1311.bb");

		let bundle_id = file.bundle_id();

		assert_eq!(
			bundle_id.to_string(),
			"b-f44d255c5c5c4be8a92a9c532d05e148648e82451e1b90a9e25be96e218cf937"
		);
	}
}
