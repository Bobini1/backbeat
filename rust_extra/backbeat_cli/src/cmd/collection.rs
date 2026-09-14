use crate::fmt::{self, Table};
use crate::store_util;
use crate::style;
use anyhow::Context;
use backbeat_core::{CollectionHeader, CollectionKind, Course, Pack, Table as TableCollection};
use backbeat_sdk::StoreError;
use backbeat_sdk::collections::{
	CollectionDownloadDataReport, CollectionDownloadProgress, CollectionUpsertStatus,
};
use clap::Subcommand;

/// Low-level utilities for working with collection files directly.
#[derive(Debug, Subcommand)]
pub enum CollectionCommand {
	/// Download a collection and store it locally, or update it when already installed.
	#[command(visible_alias = "update")]
	Add {
		/// Collection URL (e.g. `https://col.example.com/tables/foo`).
		url: String,
	},

	/// Remove an installed collection by URL.
	Rm {
		/// Collection URL to remove.
		url: String,
	},

	/// Print metadata and stats for a collection URL.
	Info {
		/// Collection URL (e.g. `https://col.example.com/tables/foo`).
		url: String,
	},

	/// Download charts, bundles, and assets referenced by an installed collection.
	InstallData {
		/// Collection URL whose referenced data should be downloaded.
		url: String,
	},

	/// List installed collections.
	List,
}

impl CollectionCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		match self {
			Self::Add { url } => install_or_update(&url).await,
			Self::Rm { url } => {
				let store = store_util::open()?;
				if store.collection_rm(&url, false).is_err() {
					anyhow::bail!("collection not installed: {url}");
				}

				println!("{} Removed collection {url}", style::check_mark());
				Ok(())
			}
			Self::Info { url } => {
				let fetched = fetch_collection(&url)
					.await
					.context("failed to fetch collection")?;
				let data_filename = CollectionHeader::data_filename(fetched.header.kind);

				println!("{}", style::bold("Collection"));
				Table::new()
					.kv("URL", url)
					.kv("Kind", fetched.header.kind.as_str())
					.kv("Data", data_filename)
					.kv("Timestamp", fetched.header.timestamp.to_rfc3339())
					.blank()
					.print();

				println!("{}", style::bold("Contents"));
				match fetched.header.kind {
					CollectionKind::Table => print_table_stats(&fetched.bytes)?,
					CollectionKind::Course => print_course_stats(&fetched.bytes)?,
					CollectionKind::Pack => print_pack_stats(&fetched.bytes)?,
				}

				Ok(())
			}
			Self::InstallData { url } => {
				let store = store_util::open()?;
				let mut progress_bar = None;
				let report = store
					.collection_fetch_download_data(&url, |event: CollectionDownloadProgress| {
						let progress = progress_bar.get_or_insert_with(|| {
							style::new_count_progress(
								event.total,
								format!("Downloading data for {url}"),
							)
						});
						progress.set_length(event.total);
						progress.set_position(event.current);
						progress.set_message(event.item.to_string());
					})
					.await
					.map_err(|err| match err {
						StoreError::Parse(message) => anyhow::anyhow!("{message}"),
						other => anyhow::Error::new(other)
							.context(format!("failed to download data for collection {url}")),
					})?;
				if let Some(progress) = progress_bar.as_ref() {
					progress.finish_and_clear();
				}
				print_download_data_report(&report);
				if report.has_failures() {
					anyhow::bail!("{} download(s) failed", report.errors.len());
				}
				Ok(())
			}
			Self::List => list_installed_collections(),
		}
	}
}

async fn install_or_update(url: &str) -> anyhow::Result<()> {
	let store = store_util::open()?;
	let result = store.collection_fetch_upsert(url).await?;
	let verb = match result.status {
		CollectionUpsertStatus::Inserted => "Installed",
		CollectionUpsertStatus::Updated => "Updated",
		CollectionUpsertStatus::TimestampUnchanged => "Already current",
	};
	println!(
		"{} {verb} {} collection {url}",
		style::check_mark(),
		result.kind.as_str()
	);
	Ok(())
}

struct FetchedCollection {
	header: CollectionHeader,
	bytes: Vec<u8>,
}

async fn fetch_collection(url: &str) -> anyhow::Result<FetchedCollection> {
	let header_url = format!("{url}/header.json");
	let header_bytes = fetch_bytes(&header_url)
		.await
		.with_context(|| format!("failed to fetch {header_url}"))?;
	let header: CollectionHeader = serde_json::from_slice(&header_bytes)
		.with_context(|| format!("invalid header from {header_url}"))?;

	let data_url = header.where_to_look(url);
	let bytes = fetch_bytes(&data_url)
		.await
		.with_context(|| format!("failed to fetch {data_url}"))?;

	validate_body(&header, &bytes)?;

	Ok(FetchedCollection { header, bytes })
}

fn validate_body(header: &CollectionHeader, bytes: &[u8]) -> anyhow::Result<()> {
	let body_timestamp = match header.kind {
		CollectionKind::Table => TableCollection::from_json(bytes)?.updated,
		CollectionKind::Course => Course::from_json(bytes)?.updated,
		CollectionKind::Pack => Pack::from_json(bytes)?.updated,
	};

	if body_timestamp != header.timestamp {
		anyhow::bail!("collection timestamp changed while downloading; please try again");
	}

	Ok(())
}

async fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_secs(15))
		.build()
		.context("failed to build HTTP client")?;
	let response = client
		.get(url)
		.send()
		.await
		.with_context(|| format!("failed to fetch {url}"))?;

	if !response.status().is_success() {
		anyhow::bail!("server returned {}", response.status());
	}

	Ok(response
		.bytes()
		.await
		.context("failed to read response")?
		.to_vec())
}

fn print_table_stats(bytes: &[u8]) -> anyhow::Result<()> {
	let table = TableCollection::from_json(bytes)?;
	let chart_count = table.chart_count();
	Table::new()
		.kv("Name", table.name)
		.kv("Symbol", table.symbol)
		.kv("Gamemode", table.gamemode.to_string())
		.kv("Levels", fmt::count(table.levels.len() as u64))
		.kv("Charts", fmt::count(chart_count as u64))
		.kv("Folders", fmt::count(table.folders.len() as u64))
		.kv("Tags", fmt::count(table.tags.len() as u64))
		.print();
	Ok(())
}

fn print_course_stats(bytes: &[u8]) -> anyhow::Result<()> {
	let course = Course::from_json(bytes)?;
	Table::new()
		.kv("Name", course.name)
		.kv("Gamemode", course.gamemode.to_string())
		.kv("Charts", fmt::count(course.charts.len() as u64))
		.kv("Tags", fmt::count(course.tags.len() as u64))
		.print();
	Ok(())
}

fn print_pack_stats(bytes: &[u8]) -> anyhow::Result<()> {
	let pack = Pack::from_json(bytes)?;
	Table::new()
		.kv("Name", pack.name)
		.kv("Gamemode", pack.gamemode.to_string())
		.kv("Bundles", fmt::count(pack.bundles.len() as u64))
		.kv("Tags", fmt::count(pack.tags.len() as u64))
		.print();
	Ok(())
}

struct ListedCollection {
	kind: String,
	url: String,
	name: Option<String>,
	last_updated: Option<String>,
}

fn list_installed_collections() -> anyhow::Result<()> {
	let store = store_util::open()?;
	let mut entries = Vec::new();

	for (kind, records) in [
		("table", store.list_tables(&[])?),
		("course", store.list_courses(&[])?),
		("pack", store.list_packs(&[])?),
	] {
		for record in records {
			entries.push(ListedCollection {
				kind: kind.to_owned(),
				url: record.url,
				name: Some(record.name),
				last_updated: Some(record.updated.to_rfc3339()),
			});
		}
	}

	entries.sort_by(|left, right| {
		left.kind
			.cmp(&right.kind)
			.then_with(|| left.url.cmp(&right.url))
	});

	if entries.is_empty() {
		println!("No collections installed.");
		println!("  Add one with: bkb collection add <url>");
		return Ok(());
	}

	println!("{}", style::bold("Collections"));
	for entry in entries {
		Table::new()
			.kv("Kind", entry.kind)
			.kv("URL", entry.url)
			.kv("Name", entry.name.unwrap_or_else(|| "—".to_string()))
			.kv(
				"Updated",
				entry.last_updated.unwrap_or_else(|| "—".to_string()),
			)
			.blank()
			.print();
	}

	Ok(())
}

fn print_download_data_report(report: &CollectionDownloadDataReport) {
	let downloaded =
		report.charts_downloaded + report.bundles_downloaded + report.assets_downloaded;
	let skipped = report.charts_skipped + report.bundles_skipped + report.assets_skipped;
	let failed = report.charts_failed + report.bundles_failed + report.assets_failed;

	if downloaded == 0 && skipped == 0 && failed == 0 {
		println!("{} Nothing to download", style::check_mark());
		return;
	}

	let headline = if report.has_failures() {
		format!(
			"{} Downloaded collection data with failures",
			style::cross_mark()
		)
	} else {
		format!("{} Downloaded collection data", style::check_mark())
	};
	println!("{headline}");
	Table::new()
		.kv(
			"Charts",
			format_download_counts(
				report.charts_downloaded,
				report.charts_skipped,
				report.charts_failed,
			),
		)
		.kv(
			"Bundles",
			format_download_counts(
				report.bundles_downloaded,
				report.bundles_skipped,
				report.bundles_failed,
			),
		)
		.kv(
			"Assets",
			format_download_counts(
				report.assets_downloaded,
				report.assets_skipped,
				report.assets_failed,
			),
		)
		.print();

	if report.has_failures() {
		eprintln!();
		eprintln!("{}", style::bold("Failures"));
		for failure in &report.errors {
			eprintln!("  {} {}", style::error_label(), failure.item);
			eprintln!("    {}", failure.message);
		}
	}
}

fn format_download_counts(downloaded: u64, skipped: u64, failed: u64) -> String {
	if downloaded == 0 && skipped == 0 && failed == 0 {
		"—".to_string()
	} else {
		let mut parts = Vec::new();
		if downloaded > 0 {
			parts.push(format!("{} downloaded", fmt::count(downloaded)));
		}
		if skipped > 0 {
			parts.push(format!("{} skipped", fmt::count(skipped)));
		}
		if failed > 0 {
			parts.push(format!("{} failed", fmt::count(failed)));
		}
		parts.join(", ")
	}
}
