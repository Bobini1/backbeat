use std::future::Future;

use backbeat_core::{
	AssetId, BackbeatFile, BundleId, ChartId, CollectionKind, CombinedAssetsId, Course,
	IdAlgorithm, Pack, Sha256, Table,
};
use backbeat_server_client::{
	BackbeatServerClient, RemoteAssetReturn, RemoteError, RemotePrecombinedAssetsReturn,
};
use bytes::Bytes;

use crate::{
	Backbeat, Result, StoreError,
	collections::{CollectionClientError, CollectionUpsertStatus, ensure_url_kind},
	remote::do_not_try_another_server,
	util::{BLOCK, encode_json},
};

impl Backbeat {
	pub(crate) async fn run<F, T>(&self, op: impl FnOnce(Self) -> F + Send + 'static) -> T
	where
		F: Future<Output = T> + Send + 'static,
		T: Send + 'static,
	{
		let store = self.clone();
		let task = tokio_util::task::AbortOnDropHandle::new(
			crate::util::runtime().spawn(async move { op(store).await }),
		);
		match task.await {
			Ok(value) => value,
			Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
			Err(error) => panic!("backbeat runtime task stopped: {error}"),
		}
	}

	pub(crate) async fn increment_refresh(connection: &mut sqlx::SqliteConnection) -> Result<()> {
		let result = sqlx::query!(
			r#"
			UPDATE
				refresh
			SET
				revision = CASE WHEN revision >= 9000000000000000000
					THEN 0
					ELSE revision + 1
				END
			WHERE
				id = 1
			"#
		)
		.execute(connection)
		.await?;
		if result.rows_affected() != 1 {
			return Err(StoreError::Corrupt("refresh row is missing".into()));
		}
		Ok(())
	}

	pub(crate) async fn import_inline_asset(
		&self,
		asset_id: AssetId,
		size: i64,
		data: Vec<u8>,
	) -> Result<()> {
		let mut tx = self.pool.begin().await?;
		let result = sqlx::query!(
			"INSERT OR IGNORE INTO downloaded_asset (sha256, size, inline_data) VALUES (?1, ?2, ?3)",
			asset_id,
			size,
			data,
		)
		.execute(&mut *tx)
		.await?;
		if result.rows_affected() > 0 {
			Self::increment_refresh(&mut tx).await?;
		}
		tx.commit().await?;
		Ok(())
	}

	pub(crate) async fn import_file_asset(&self, asset_id: AssetId, size: i64) -> Result<()> {
		let mut tx = self.pool.begin().await?;
		let result = sqlx::query!(
			"INSERT OR IGNORE INTO downloaded_asset (sha256, size, inline_data) VALUES (?1, ?2, NULL)",
			asset_id,
			size,
		)
		.execute(&mut *tx)
		.await?;
		if result.rows_affected() > 0 {
			Self::increment_refresh(&mut tx).await?;
		}
		tx.commit().await?;
		Ok(())
	}

	pub(crate) async fn collection_upsert_status(
		&self,
		url: &str,
		kind: CollectionKind,
		updated: chrono::DateTime<chrono::Utc>,
	) -> Result<CollectionUpsertStatus> {
		match self.collection_kind_for_url_async(url).await {
			Ok(existing) if existing != kind => Err(StoreError::Parse(format!(
				"collection URL {url:?} is already installed as {existing}"
			))),
			Ok(_) => {
				let stored = match kind {
					CollectionKind::Table => {
						sqlx::query_scalar!("SELECT updated FROM difftable WHERE url = ?1", url)
							.fetch_one(&self.pool)
							.await?
					}
					CollectionKind::Course => {
						sqlx::query_scalar!("SELECT updated FROM course WHERE url = ?1", url)
							.fetch_one(&self.pool)
							.await?
					}
					CollectionKind::Pack => {
						sqlx::query_scalar!("SELECT updated FROM pack WHERE url = ?1", url)
							.fetch_one(&self.pool)
							.await?
					}
				};
				let stored = crate::util::parse_iso8601(&stored)?;
				if updated > stored {
					Ok(CollectionUpsertStatus::Updated)
				} else {
					Ok(CollectionUpsertStatus::TimestampUnchanged)
				}
			}
			Err(StoreError::NotFound(_)) => Ok(CollectionUpsertStatus::Inserted),
			Err(err) => Err(err),
		}
	}

	pub(crate) async fn course_put(&self, url: &str, course: &Course) -> Result<()> {
		let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
		ensure_url_kind(&mut tx, url, CollectionKind::Course).await?;
		let tags = encode_json(&course.tags)?;
		let updated = course.updated.to_rfc3339();
		let gamemode = course.gamemode.as_str();
		sqlx::query!("INSERT INTO course (url, name, updated, gamemode, tags) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(url) DO UPDATE SET name = excluded.name, updated = excluded.updated, gamemode = excluded.gamemode, tags = excluded.tags", url, course.name, updated, gamemode, tags)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM course_asset WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM course_chart WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		for (path, asset) in &course.assets {
			sqlx::query!(
				"INSERT INTO course_asset (url, path, sha256) VALUES (?1, ?2, ?3)",
				url,
				path,
				asset
			)
			.execute(&mut *tx)
			.await?;
		}
		for (entry, chart) in course.charts.iter().enumerate() {
			let entry = (entry + 1) as i64;
			let tags = encode_json(&chart.tags)?;
			let id = chart.id.to_string();
			let desc = chart.desc.clone();
			sqlx::query!(
				"INSERT INTO course_chart (url, entry, id, desc, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
				url,
				entry,
				id,
				desc,
				tags
			)
			.execute(&mut *tx)
			.await?;
		}
		Self::increment_refresh(&mut tx).await?;
		tx.commit().await?;
		Ok(())
	}

	pub(crate) async fn pack_put(&self, url: &str, pack: &Pack) -> Result<()> {
		let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
		ensure_url_kind(&mut tx, url, CollectionKind::Pack).await?;
		let tags = encode_json(&pack.tags)?;
		let updated = pack.updated.to_rfc3339();
		let gamemode = pack.gamemode.as_str();
		sqlx::query!("INSERT INTO pack (url, name, gamemode, updated, tags) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(url) DO UPDATE SET name = excluded.name, gamemode = excluded.gamemode, updated = excluded.updated, tags = excluded.tags", url, pack.name, gamemode, updated, tags)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM pack_asset WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM pack_entry WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		for (path, asset) in &pack.assets {
			sqlx::query!(
				"INSERT INTO pack_asset (url, path, sha256) VALUES (?1, ?2, ?3)",
				url,
				path,
				asset
			)
			.execute(&mut *tx)
			.await?;
		}
		for (entry, bundle) in pack.bundles.iter().enumerate() {
			let entry = (entry + 1) as i64;
			let tags = encode_json(&bundle.tags)?;
			let bundle_id = bundle.id.to_string();
			let desc = bundle.desc.clone();
			sqlx::query!(
				"INSERT INTO pack_entry (url, entry, bundle_id, desc, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
				url,
				entry,
				bundle_id,
				desc,
				tags
			)
			.execute(&mut *tx)
			.await?;
		}
		Self::increment_refresh(&mut tx).await?;
		tx.commit().await?;
		Ok(())
	}

	pub(crate) async fn table_put(&self, url: &str, table: &Table) -> Result<()> {
		let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
		ensure_url_kind(&mut tx, url, CollectionKind::Table).await?;
		let tags = encode_json(&table.tags)?;
		let updated = table.updated.to_rfc3339();
		let gamemode = table.gamemode.as_str();
		sqlx::query!(
			"INSERT INTO difftable (url, name, symbol, updated, gamemode, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(url) DO UPDATE SET name = excluded.name, symbol = excluded.symbol, updated = excluded.updated, gamemode = excluded.gamemode, tags = excluded.tags",
			url, table.name, table.symbol, updated, gamemode, tags
		)
		.execute(&mut *tx)
		.await?;
		sqlx::query!("DELETE FROM difftable_asset WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM difftable_chart WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM difftable_level WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		sqlx::query!("DELETE FROM difftable_folder WHERE url = ?1", url)
			.execute(&mut *tx)
			.await?;
		for (path, asset) in &table.assets {
			sqlx::query!(
				"INSERT INTO difftable_asset (url, path, sha256) VALUES (?1, ?2, ?3)",
				url,
				path,
				asset
			)
			.execute(&mut *tx)
			.await?;
		}
		for (level_order, level) in table.levels.iter().enumerate() {
			let order = (level_order + 1) as i64;
			let tags = encode_json(&level.tags)?;
			let level_name = level.level.as_str();
			sqlx::query!(
				"INSERT INTO difftable_level (url, level, level_order, tags) VALUES (?1, ?2, ?3, ?4)",
				url,
				level_name,
				order,
				tags
			)
			.execute(&mut *tx)
			.await?;
			for (chart_order, chart) in level.charts.iter().enumerate() {
				let chart_order = (chart_order + 1) as i64;
				let tags = encode_json(&chart.tags)?;
				let id = chart.id.to_string();
				let desc = chart.desc.as_str();
				sqlx::query!("INSERT INTO difftable_chart (url, level, chart_order, id, desc, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", url, level_name, chart_order, id, desc, tags)
					.execute(&mut *tx)
					.await?;
			}
		}
		for (folder_order, folder) in table.folders.iter().enumerate() {
			let tags = encode_json(&folder.tags)?;
			let order = (folder_order + 1) as i64;
			sqlx::query!(
				"INSERT INTO difftable_folder (url, folder_order, name, query, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
				url,
				order,
				folder.name,
				folder.query,
				tags
			)
			.execute(&mut *tx)
			.await?;
		}
		Self::increment_refresh(&mut tx).await?;
		tx.commit().await?;
		Ok(())
	}

	/// Do a remote operation for each remote you have installed.
	pub(crate) async fn for_all_remotes<T, F, Fut>(
		&self,
		mut op: F,
	) -> std::result::Result<T, RemoteError>
	where
		F: FnMut(BackbeatServerClient) -> Fut,
		Fut: std::future::Future<Output = std::result::Result<T, RemoteError>>,
	{
		let clients = self.servers.read().clone();
		if clients.is_empty() {
			return Err(RemoteError::NoServers);
		}

		let mut error_to_eventually_return = None;

		for remote in clients {
			match op(remote).await {
				Ok(value) => return Ok(value),
				Err(err) => {
					if do_not_try_another_server(&err) {
						return Err(err);
					}

					error_to_eventually_return = Some(err);
				}
			}
		}

		// this unwrap_or should never be hit :P
		Err(error_to_eventually_return.unwrap_or(RemoteError::NoServers))
	}

	pub(crate) async fn has_asset_async(&self, asset_id: AssetId) -> Result<bool> {
		Ok(sqlx::query_scalar!(
			"SELECT 1 AS \"exists!: i64\" FROM downloaded_asset WHERE sha256 = ?1 LIMIT 1",
			asset_id
		)
		.fetch_optional(&self.pool)
		.await?
		.is_some())
	}

	pub(crate) async fn import_bb_async(&self, bb: &BackbeatFile) -> Result<BundleId> {
		let raw_bytes = bb.chart.decompress();

		let sha256 = bb.chart_sha256();
		let uncompressed_size = raw_bytes.len() as i64;

		let description = bb.desc.clone();
		let combined_assets_id = bb.combined_assets_id();
		let data = bb.chart.as_compressed().to_vec();
		let filename = &bb.filename;
		let extension = filename.extension();

		let bundle_id = BundleId::compute(&bb.filename, sha256, &bb.assets);
		let bundle_id_str = bundle_id.to_string();

		let mut tx = self.pool.begin().await?;

		sqlx::query!(
			r#"
			INSERT OR IGNORE INTO
				chart_data(sha256, gzip_data, uncompressed_size)
			VALUES
				(?1, ?2, ?3)
			"#,
			sha256,
			data,
			uncompressed_size,
		)
		.execute(&mut *tx)
		.await?;

		for (path, asset_id) in &bb.assets {
			sqlx::query!(
				"INSERT OR IGNORE INTO asset_map (combined_assets_id, path, sha256) VALUES (?1, ?2, ?3)",
				combined_assets_id,
				path,
				asset_id,
			)
			.execute(&mut *tx)
			.await?;
		}

		sqlx::query!(
			r#"
			INSERT INTO
				bundle(id, chart_sha256, filename, extension, description, combined_assets_id)
			VALUES
				(?1, ?2, ?3, ?4, ?5, ?6)
			ON CONFLICT(id) DO NOTHING
			"#,
			bundle_id_str,
			sha256,
			filename,
			extension,
			description,
			combined_assets_id,
		)
		.execute(&mut *tx)
		.await?;

		let chart_ids = bb.additional_chart_ids();

		for id in chart_ids {
			sqlx::query!(
				"INSERT OR IGNORE INTO chart_id (chart_sha256, id) VALUES (?1, ?2)",
				sha256,
				id.to_string(),
			)
			.execute(&mut *tx)
			.await?;
		}

		Self::increment_refresh(&mut tx).await?;
		tx.commit().await?;

		Ok(bundle_id)
	}

	pub(crate) async fn has_chart_async(&self, chart_id: &ChartId) -> Result<bool> {
		let exists = match &chart_id.alg {
			IdAlgorithm::Sha256 => {
				let sha256: Sha256 = chart_id.val.parse().map_err(|err| {
					StoreError::Parse(format!("invalid sha256 chart ID {chart_id}: {err}"))
				})?;
				sqlx::query_scalar!(
					r#"
					SELECT
						1 AS "exists!: i64"
					FROM
						chart_data
					WHERE
						sha256 = ?1
					LIMIT 1
					"#,
					sha256,
				)
				.fetch_optional(&self.pool)
				.await?
			}
			IdAlgorithm::Custom(_) => {
				sqlx::query_scalar!(
					r#"
						SELECT
							1 AS "exists!: i64"
						FROM
							chart_data
						JOIN
							chart_id ON chart_id.chart_sha256 = chart_data.sha256
						WHERE
							chart_id.id = ?1
						LIMIT
							1
						"#,
					chart_id.to_string(),
				)
				.fetch_optional(&self.pool)
				.await?
			}
		};

		Ok(exists.is_some())
	}

	pub(crate) async fn has_bundle_async(&self, bundle_id: BundleId) -> Result<bool> {
		let bundle_id = bundle_id.to_string();

		Ok(sqlx::query_scalar!(
			r#"
				SELECT
					1 AS "exists!: i64"
				FROM
					bundle
				WHERE
					id = ?1
				LIMIT 1
				"#,
			bundle_id,
		)
		.fetch_optional(&self.pool)
		.await?
		.is_some())
	}

	/// Given an arbitrary url, just fetch the bytes under it.
	pub(crate) async fn http_fetch_bytes(&self, url: &str) -> Result<Bytes> {
		let response = self
			.general_http_client
			.get(url)
			.send()
			.await
			.map_err(CollectionClientError::Request)?;

		if !response.status().is_success() {
			return Err(CollectionClientError::HttpStatus {
				url: response.url().to_string(),
				status: response.status().as_u16(),
			}
			.into());
		}

		let bytes = response
			.bytes()
			.await
			.map_err(CollectionClientError::Request)?;
		Ok(bytes)
	}

	pub(crate) fn collection_kind_for_url(&self, url: &str) -> Result<CollectionKind> {
		BLOCK(self.collection_kind_for_url_async(url))
	}

	pub(crate) async fn collection_kind_for_url_async(&self, url: &str) -> Result<CollectionKind> {
		let mut kinds = Vec::new();
		if sqlx::query!("SELECT url FROM difftable WHERE url = ?1", url)
			.fetch_optional(&self.pool)
			.await?
			.is_some()
		{
			kinds.push(CollectionKind::Table);
		}
		if sqlx::query!("SELECT url FROM course WHERE url = ?1", url)
			.fetch_optional(&self.pool)
			.await?
			.is_some()
		{
			kinds.push(CollectionKind::Course);
		}
		if sqlx::query!("SELECT url FROM pack WHERE url = ?1", url)
			.fetch_optional(&self.pool)
			.await?
			.is_some()
		{
			kinds.push(CollectionKind::Pack);
		}
		match kinds.as_slice() {
			[] => Err(StoreError::NotFound(url.to_owned())),
			[kind] => Ok(*kind),
			_ => Err(StoreError::Corrupt(format!(
				"collection URL {url:?} is installed under multiple kinds"
			))),
		}
	}

	/// Using your configured data servers, attempt to download this bundle_id.
	pub(crate) async fn server_get_bundle(&self, bundle_id: BundleId) -> Result<BackbeatFile> {
		self.for_all_remotes(|remote| async move { remote.get_bundle(bundle_id).await })
			.await
			.map_err(StoreError::Remote)
	}

	pub(crate) async fn server_get_chart(&self, chart_id: &ChartId) -> Result<BackbeatFile> {
		self.for_all_remotes(|remote| {
			let id = chart_id.clone();
			async move { remote.get_chart(&id).await }
		})
		.await
		.map_err(StoreError::Remote)
	}

	pub(crate) async fn server_get_asset(&self, asset_id: AssetId) -> Result<RemoteAssetReturn> {
		self.for_all_remotes(|remote| async move { remote.get_asset(asset_id).await })
			.await
			.map_err(StoreError::Remote)
	}

	pub(crate) async fn server_get_precombined_assets(
		&self,
		combined_assets_id: CombinedAssetsId,
	) -> Result<RemotePrecombinedAssetsReturn> {
		self.for_all_remotes(|remote| async move {
			remote.get_precombined_assets(combined_assets_id).await
		})
		.await
		.map_err(StoreError::Remote)
	}
}

#[cfg(test)]
mod tests {
	use backbeat_core::{AssetId, Sha256};

	#[test]
	fn refresh_revision_tracks_store_mutations() {
		let (_tmp, store) = crate::test_util::new_test_store("refresh_revision");
		let observer =
			crate::Backbeat::open_with_overridden_config_dir(store.config_dir()).unwrap();
		assert_eq!(observer.should_refresh(-1).unwrap(), (true, 0));
		assert_eq!(observer.should_refresh(0).unwrap(), (false, 0));

		let data = b"asset".to_vec();
		let asset_id = AssetId(Sha256::checksum_bytes(&data));
		crate::util::BLOCK(store.import_inline_asset(asset_id, data.len() as i64, data.clone()))
			.unwrap();
		assert_eq!(observer.should_refresh(0).unwrap(), (true, 1));

		crate::util::BLOCK(store.import_inline_asset(asset_id, data.len() as i64, data)).unwrap();
		assert_eq!(observer.should_refresh(1).unwrap(), (false, 1));
	}
}
