use backbeat_core::{AssetId, BundleId, ChartId, bb::CombinedAssetsId};

/// A static struct that has all the paths you need for
/// interfacing with a server.
pub struct ServerPaths;

impl ServerPaths {
	pub fn backbeat_info() -> &'static str {
		"/backbeat/info"
	}

	pub fn backbeat_up() -> &'static str {
		"/backbeat/up"
	}

	pub fn bundle(bundle_id: BundleId) -> String {
		format!("/bundles/{bundle_id}")
	}

	pub fn chart(chart_id: &ChartId) -> String {
		format!("/charts/{chart_id}")
	}

	pub fn asset(asset_id: AssetId) -> String {
		format!("/assets/{asset_id}")
	}

	pub fn precombined_assets(combined_assets_id: CombinedAssetsId) -> String {
		format!("/precombined-assets/{combined_assets_id}")
	}
}
