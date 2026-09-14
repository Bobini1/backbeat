use backbeat_core::{AssetId, BundleId, ChartId};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A "DataId" is a way of referring to an individual "piece" of data
/// in backbeat. This does not include collections.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum DataId {
	Chart(ChartId),
	Bundle(BundleId),
	Asset(AssetId),
}

impl Display for DataId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Chart(chart_id) => write!(f, "Chart {chart_id}"),
			Self::Bundle(bundle_id) => write!(f, "Bundle {bundle_id}"),
			Self::Asset(asset_id) => write!(f, "Asset {asset_id}"),
		}
	}
}
