/// Statistics about your [`crate::Backbeat`] store.
#[derive(serde::Serialize)]
pub struct StoreStats {
	pub charts: u64,
	pub tables: u64,
	pub courses: u64,
	pub packs: u64,
	pub asset_count: u64,
	pub asset_bytes: u64,
	pub db_bytes: u64,
}

impl StoreStats {
	/// Total disk space used by backbeat.
	pub fn total_bytes(&self) -> u64 {
		self.asset_bytes + self.db_bytes
	}
}
