use std::path::{Path, PathBuf};

use anyhow::Context as _;
use backbeat_core::{AssetId, Sha256};
use backbeat_packager::SeenCache;
use backbeat_sdk::Backbeat;
use clap::Args;
use futures::StreamExt as _;

use crate::{fmt, store_util, style};

/// Convert a source chart and import it and its assets into the store.
#[derive(Debug, Args)]
pub struct ConvertAndImportCommand {
	/// Path to the chart file or directory to convert and import.
	pub path: PathBuf,

	/// Recursively convert and import all supported charts under the given directory.
	#[arg(short, long)]
	pub recursive: bool,
}

impl ConvertAndImportCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;

		if self.recursive {
			self.convert_recursive(&store).await
		} else {
			self.convert_one(&store)
		}
	}

	fn convert_one(&self, store: &Backbeat) -> anyhow::Result<()> {
		Self::convert_chart(store, &self.path, &SeenCache::new())
			.with_context(|| format!("failed to convert and import {}", self.path.display()))?;

		println!(
			"{} converted and imported {}",
			style::check_mark(),
			self.path.display()
		);
		Ok(())
	}

	async fn convert_recursive(&self, store: &Backbeat) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.path.is_dir(),
			"--recursive requires a directory, got: {}",
			self.path.display()
		);

		let concurrency = std::thread::available_parallelism().map_or(4, |n| n.get());
		let cache = SeenCache::new();
		let (tx, rx) = tokio::sync::mpsc::channel::<PathBuf>(concurrency * 4);

		let spinner = style::new_spinner(format!("Scanning {}…", self.path.display()));
		let root = self.path.clone();
		let pb_walk = spinner.clone();
		let walk_task = tokio::task::spawn_blocking(move || {
			let mut chart_count = 0usize;
			for entry in walkdir::WalkDir::new(root)
				.follow_links(true)
				.into_iter()
				.filter_map(|entry| entry.ok())
				.filter(|entry| entry.file_type().is_file())
			{
				if backbeat_packager::should_be_packaged(entry.path()) {
					chart_count += 1;
					pb_walk.set_message(format!(
						"Scanning… {} chart{} found",
						fmt::count(chart_count as u64),
						if chart_count == 1 { "" } else { "s" }
					));
					if tx.blocking_send(entry.into_path()).is_err() {
						break;
					}
				}
			}
		});

		let chart_stream = futures::stream::unfold(rx, |mut rx| async move {
			rx.recv().await.map(|item| (item, rx))
		});

		struct Acc {
			total: usize,
			errors: usize,
		}

		let pb_import = spinner.clone();
		let acc = chart_stream
			.map(|path| {
				let store = store.clone();
				let cache = cache.clone();
				tokio::task::spawn_blocking(move || {
					let result = Self::convert_chart(&store, &path, &cache);
					(path, result)
				})
			})
			.buffer_unordered(concurrency)
			.filter_map(|join| async move { join.ok() })
			.fold(
				Acc {
					total: 0,
					errors: 0,
				},
				|mut acc, (path, result)| {
					let pb = pb_import.clone();
					async move {
						match result {
							Ok(()) => {
								acc.total += 1;
								pb.set_message(format!(
									"Converting and importing… {}",
									fmt::count(acc.total as u64)
								));
							}
							Err(err) => {
								pb.println(format!(
									"  {} skipped {}: {err:#}",
									style::warn_label(),
									path.display()
								));
								acc.errors += 1;
							}
						}
						acc
					}
				},
			)
			.await;

		walk_task.await.context("walkdir task panicked")?;
		spinner.finish_and_clear();
		println!(
			"{} Converted and imported {} chart{}, {} skipped",
			style::check_mark(),
			fmt::count(acc.total as u64),
			if acc.total == 1 { "" } else { "s" },
			fmt::count(acc.errors as u64),
		);

		if acc.errors > 0 && acc.total == 0 {
			anyhow::bail!("all files failed to convert and import");
		}

		Ok(())
	}

	fn convert_chart(store: &Backbeat, path: &Path, cache: &SeenCache) -> anyhow::Result<()> {
		let chart_dir = path.parent().unwrap_or(Path::new("."));
		let bundles = backbeat_packager::package_with_cache(path, cache)?;

		if let Some(bundle) = bundles.first() {
			for (filename, &asset_id) in &bundle.assets {
				let asset_path = chart_dir.join(filename);
				let data = fs_err::read(&asset_path).with_context(|| {
					format!("failed to read chart asset {}", asset_path.display())
				})?;

				let actual = AssetId(Sha256::checksum_bytes(&data));
				anyhow::ensure!(actual == asset_id, "asset hash changed while importing");

				store.import_asset(&asset_path)?;
			}
		}

		for bundle in &bundles {
			store.import_bundle(bundle)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use backbeat_store_config::BackbeatConfig;

	use super::*;

	fn write_sm(dir: &Path, name: &str) -> PathBuf {
		let path = dir.join(name);
		fs_err::write(
			&path,
			"#TITLE:Test;\n\
			 #ARTIST:Test;\n\
			 #MUSIC:song.ogg;\n\
			 #NOTES:\n\
			     dance-single:\n\
			     Author:\n\
			     Hard:\n\
			     10:\n\
			     0,0,0,0,0:\n\
			0000\n\
			;\n",
		)
		.unwrap();
		path
	}

	#[test]
	fn missing_cached_asset_does_not_import_bundle_metadata() {
		let temp = tempfile::TempDir::new().unwrap();
		let config_dir = temp.path().join("config");
		let source_dir = temp.path().join("source");
		fs_err::create_dir_all(&source_dir).unwrap();

		let mut config = BackbeatConfig::default();
		config.store.path = temp.path().join("store");
		config.write_to_dir(&config_dir).unwrap();
		let store = Backbeat::open_with_overridden_config_dir(&config_dir).unwrap();

		let song_a = write_sm(&source_dir, "song-a.sm");
		let song_b = write_sm(&source_dir, "song-b.sm");
		let asset = source_dir.join("song.ogg");
		fs_err::write(&asset, b"fake ogg bytes").unwrap();

		let cache = SeenCache::new();
		ConvertAndImportCommand::convert_chart(&store, &song_a, &cache).unwrap();
		let charts_before = store.stats().unwrap().charts;

		fs_err::remove_file(asset).unwrap();
		ConvertAndImportCommand::convert_chart(&store, &song_b, &cache)
			.expect_err("missing asset must fail conversion before importing metadata");

		assert_eq!(store.stats().unwrap().charts, charts_before);
	}
}
