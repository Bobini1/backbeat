//! Packs are sets of bundles.

use std::io;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Assets, BundleId, ValidGamemodeIdentifier};

define_tags!(PackTags);
define_tags!(PackBundleTags);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
	pub name: String,
	/// Main gamemode this pack is organised around.
	pub gamemode: ValidGamemodeIdentifier,
	pub updated: DateTime<Utc>,
	pub tags: PackTags,
	/// Collection-level assets, keyed by collection-relative filepath.
	pub assets: Assets,
	pub bundles: Vec<PackBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackBundle {
	pub id: BundleId,
	pub desc: String,
	pub tags: PackBundleTags,
}

impl Pack {
	pub const EXTENSION: &str = "bbpack";

	pub fn to_json(&self) -> Vec<u8> {
		serde_json::to_vec_pretty(self).expect("must ser")
	}

	pub fn from_json(s: &[u8]) -> io::Result<Self> {
		Ok(serde_json::from_slice(s)?)
	}
}
