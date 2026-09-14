use std::hash::Hash;

use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::{AssetId, Assets, Sha256};

/// A deterministic identifier for **all** the assets in a bb file, combined deterministically.
///
/// In short, an id for the complete list of assets this bundle refers to.
///
/// Used to group charts up - charts with the same `assets` id can be put in the same folder, usually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct CombinedAssetsId(pub Sha256);

/// We prefix combined assets ids with `a-` to easily distinguish them from other sha256s
const COMBINED_ASSETS_ID_PREFIX: &str = "a-";

impl std::fmt::Display for CombinedAssetsId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{COMBINED_ASSETS_ID_PREFIX}{}", self.0)
	}
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("expected a lowercase sha256 checksum with the prefix 'a-'")]
pub struct ParseCombinedAssetsIdError;

impl std::str::FromStr for CombinedAssetsId {
	type Err = ParseCombinedAssetsIdError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let hex = s
			.strip_prefix(COMBINED_ASSETS_ID_PREFIX)
			.ok_or(ParseCombinedAssetsIdError)?;
		Sha256::from_str(hex)
			.map(Self)
			.map_err(|_| ParseCombinedAssetsIdError)
	}
}

impl CombinedAssetsId {
	pub fn compute(assets: &Assets) -> Self {
		fn write_field(buf: &mut Vec<u8>, bytes: &[u8]) {
			let len: u32 = bytes
				.len()
				.try_into()
				.expect("field too large to encode in a bundle frame");
			buf.extend_from_slice(&len.to_le_bytes());
			buf.extend_from_slice(bytes);
		}

		let mut buf = vec![];

		// str Ord impl is just lexicographic utf8 encoding
		let mut entries: Vec<(&str, &AssetId)> =
			assets.iter().map(|(k, v)| (k.as_str(), v)).collect();
		entries.sort_unstable_by_key(|(filename, _)| *filename);

		let count: u32 = entries
			.len()
			.try_into()
			.expect("too many assets to encode in a bundle frame");
		buf.extend_from_slice(&count.to_le_bytes());

		for (filename, asset_id) in entries {
			write_field(&mut buf, filename.as_bytes());
			buf.extend_from_slice(asset_id.0.as_bytes());
		}

		Self(Sha256::checksum_bytes(&buf))
	}
}
