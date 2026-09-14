use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseSha256Error;
use crate::Sha256;

/// A content-addressed reference to a Backbeat asset: a SHA-256 checksum of the
/// asset bytes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AssetId(pub Sha256);

impl From<Sha256> for AssetId {
	fn from(sha256: Sha256) -> Self {
		Self(sha256)
	}
}

impl fmt::Display for AssetId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for AssetId {
	type Err = ParseSha256Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Sha256::from_str(s).map(Self)
	}
}

/// How many subfolders we should have in our filesystem. This corresponds to the
/// amount of subfolders created from a hash, i.e.:
///
/// `12/34/4567890abcdef`
///
/// has 2 buckets.
///
/// We use 2 buckets, with a size of 2. This gives us 16^4 subfolders.
/// Since a hash function is uniformly distributed, this means we can expect to get
/// 65536 * 10_000 files (half a billion) stored before we start hitting read issues.
const FAN_BUCKETS: usize = 2;

/// How large should a bucket be?
const FAN_BUCKET_SIZE: usize = 2;

impl AssetId {
	/// Turn this asset id into a "fanned out" path; placing subfolders at
	/// certain points.
	///
	/// I.e. -> `ae/12/42/37restofhash`
	pub fn fanned_path(&self) -> PathBuf {
		let mut path = PathBuf::new();

		let str = self.0.to_string();

		for k in 0..FAN_BUCKETS {
			let start = k * FAN_BUCKET_SIZE;
			let end = (k + 1) * FAN_BUCKET_SIZE;

			path.push(&str[start..end]);
		}

		path.push(&str[(FAN_BUCKETS * FAN_BUCKET_SIZE)..]);

		path
	}

	/// Invert [`AssetId::fanned_path`]; Fails if this path is not a valid fanned out
	/// path.
	///
	/// # Warning
	///
	/// This doesn't read the bytes stored at this file; this operation merely looks
	/// at the path `ae/b3/cd/...` and turns it into sha256 `aeb3cd...`
	pub fn from_fanned_path(path: impl AsRef<Path>) -> Option<Self> {
		let path = path.as_ref();
		let path = path.with_extension("");

		// reserve the amt of bytes we expect to hit for performance.
		// having non-sha256 checksums in here is unlikely anyway
		let mut vec = Vec::with_capacity(crate::sha256::SHA256_BYTES);

		let mut paths_traversed = 0;

		for part in &path {
			let bytes = part.as_encoded_bytes();

			if paths_traversed < FAN_BUCKETS {
				// all buckets should have BUCKET_SIZE amount of bytes in their names.
				if bytes.len() != FAN_BUCKET_SIZE {
					return None;
				}
			}

			for byte in bytes {
				if !(byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)) {
					// non sha256-valid-char in path
					return None;
				}

				vec.push(*byte);
			}

			paths_traversed += 1;
		}

		if paths_traversed != FAN_BUCKETS + 1 {
			// too many paths.
			return None;
		}

		let str = String::from_utf8(vec).expect("all bytes should be ascii");

		Sha256::from_str(&str).ok().map(Self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample_asset() -> AssetId {
		let sha = Sha256::checksum_bytes(b"hello world");
		AssetId(sha)
	}

	#[test]
	fn asset_roundtrip_display_parse() {
		let asset = sample_asset();
		let s = asset.to_string();
		let parsed: AssetId = s.parse().unwrap();
		assert_eq!(asset, parsed);
	}

	#[test]
	fn asset_serde_roundtrip() {
		let asset = sample_asset();
		let json = serde_json::to_string(&asset).unwrap();
		let parsed: AssetId = serde_json::from_str(&json).unwrap();
		assert_eq!(asset, parsed);
	}

	#[test]
	fn asset_parse_errors() {
		assert!("nohash".parse::<AssetId>().is_err());
		assert!(
			"D1BC8D3BA4AFC7E109612CB73ACBDDDAC052C93025AA1F82942EDABB7DEB82A1"
				.parse::<AssetId>()
				.is_err()
		);
		assert!("badshahere".parse::<AssetId>().is_err());
	}

	#[test]
	fn path_fanning() {
		assert_eq!(
			&AssetId(Sha256::null()).fanned_path().to_string_lossy(),
			"00/00/000000000000000000000000000000000000000000000000000000000000"
		)
	}

	#[test]
	fn path_unfanning() {
		let asset = AssetId(Sha256::null());

		let fanned = asset.fanned_path();

		assert_eq!(AssetId::from_fanned_path(fanned), Some(asset))
	}
}
