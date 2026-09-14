use backbeat_core::AssetId;

use crate::Result;
use crate::store::{Backbeat, StoreError};
use crate::util::BLOCK;

pub fn store_asset_unchecked(store: &Backbeat, asset: AssetId, data: &[u8]) -> Result<()> {
	BLOCK(store_asset_unchecked_async(store, asset, data))
}

async fn store_asset_unchecked_async(store: &Backbeat, asset: AssetId, data: &[u8]) -> Result<()> {
	let size = data.len() as i64;
	let inline = store
		.config
		.read()
		.store
		.inline
		.as_usize()
		.map_err(|_| StoreError::Corrupt("store.inline exceeds platform limit".into()))?;

	if data.len() <= inline {
		sqlx::query!(
			"INSERT OR IGNORE INTO downloaded_asset (sha256, size, inline_data) VALUES (?1, ?2, ?3)",
			asset,
			size,
			data,
		)
		.execute(&store.pool)
		.await?;
	} else {
		store.assets.store(asset, data)?;
		sqlx::query!(
			"INSERT OR IGNORE INTO downloaded_asset (sha256, size, inline_data) VALUES (?1, ?2, NULL)",
			asset,
			size,
		)
		.execute(&store.pool)
		.await?;
	}

	Ok(())
}
