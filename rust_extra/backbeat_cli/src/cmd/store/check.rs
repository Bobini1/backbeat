use crate::fmt::Table;
use crate::{fmt, store_util, style};
use anyhow::Context;
use clap::Args;

/// Check the integrity of the Backbeat store.
///
/// Verifies asset contents, chart contents, and inspected chart metadata.
#[derive(Debug, Args)]
pub struct CheckCommand {}

impl CheckCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;

		let mut hdr = Table::new();
		hdr.kv("Store:", store.store_dir().display().to_string());
		hdr.blank().print();

		let spinner = style::new_spinner("Checking store integrity…");

		let report = store
			.corruption_check()
			.context("failed to run integrity check")?;

		spinner.finish_and_clear();

		// ── Assets ───────────────────────────────────────────────────────────
		let asset_ok = report.missing_assets.is_empty() && report.corrupt_assets.is_empty();
		let asset_sym = if asset_ok {
			style::check_mark()
		} else {
			style::cross_mark()
		};
		println!(
			"{asset_sym} {:<14} {:>10} checked",
			"Assets",
			fmt::count(report.large_asset_count as u64)
		);
		if !report.missing_assets.is_empty() {
			println!(
				"    {} missing on disk",
				fmt::count(report.missing_assets.len() as u64)
			);
			for asset_id in &report.missing_assets {
				println!("      {asset_id}");
			}
		}
		if !report.corrupt_assets.is_empty() {
			println!(
				"    {} wrong sha256",
				fmt::count(report.corrupt_assets.len() as u64)
			);
			for asset_id in &report.corrupt_assets {
				println!("      {asset_id}");
			}
		}

		// ── Charts ───────────────────────────────────────────────────────────
		let charts_ok = report.corrupt_charts.is_empty();
		let charts_sym = if charts_ok {
			style::check_mark()
		} else {
			style::cross_mark()
		};
		println!(
			"{charts_sym} {:<14} {:>10} checked",
			"Charts",
			fmt::count(report.chart_count as u64)
		);
		if !report.corrupt_charts.is_empty() {
			println!(
				"    {} corrupt or uninspectable",
				fmt::count(report.corrupt_charts.len() as u64)
			);
			for sha in &report.corrupt_charts {
				println!("      {sha}");
			}
		}

		// ── Chart IDs ────────────────────────────────────────────────────────
		let ids_ok = report.wrong_chart_ids.is_empty() && report.uncomputable_chart_ids.is_empty();
		let ids_sym = if ids_ok {
			style::check_mark()
		} else {
			style::cross_mark()
		};
		println!(
			"{ids_sym} {:<14} {:>10} checked",
			"Chart IDs",
			fmt::count(report.chart_id_count as u64)
		);
		if !report.wrong_chart_ids.is_empty() {
			println!(
				"    {} wrong",
				fmt::count(report.wrong_chart_ids.len() as u64)
			);
			for entry in &report.wrong_chart_ids {
				println!(
					"      {} alg={}  stored={}  computed={}",
					entry.chart_sha256, entry.alg, entry.stored_id, entry.computed_id
				);
			}
		}
		if !report.uncomputable_chart_ids.is_empty() {
			println!(
				"    {} could not be recomputed",
				fmt::count(report.uncomputable_chart_ids.len() as u64)
			);
			for entry in &report.uncomputable_chart_ids {
				println!(
					"      {} alg={}: {}",
					entry.chart_sha256, entry.alg, entry.reason
				);
			}
		}

		// ── Bundle ↔ asset links ─────────────────────────────────────────────
		if !report.dangling_asset_refs.is_empty() {
			println!(
				"{} {:<14} {:>10} dangling",
				style::cross_mark(),
				"Asset links",
				fmt::count(report.dangling_asset_refs.len() as u64)
			);
			for entry in &report.dangling_asset_refs {
				println!(
					"      bundle={} path={:?} asset={} not in downloaded_asset",
					entry.bundle_id, entry.path, entry.asset_id
				);
			}
		}

		println!();

		if report.is_ok() {
			println!(
				"{}  ({} charts, {} assets, {} chart IDs)",
				style::green("OK"),
				fmt::count(report.chart_count as u64),
				fmt::count(report.large_asset_count as u64),
				fmt::count(report.chart_id_count as u64),
			);
			Ok(())
		} else {
			println!(
				"{}  ({} issues found)",
				style::red("FAIL"),
				fmt::count(report.issue_count() as u64)
			);
			anyhow::bail!("store integrity check failed");
		}
	}
}
