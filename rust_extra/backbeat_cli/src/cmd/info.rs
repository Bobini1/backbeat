use crate::fmt::Table;
use crate::store_util;
use crate::style;
use anyhow::Context;
use backbeat_sdk::Backbeat;
use backbeat_store_config::CONFIG_FILENAME;

use clap::Args;

/// Show store information and statistics.
#[derive(Debug, Args)]
pub struct InfoCommand {}

impl InfoCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let store = store_util::open().context("failed to open Backbeat store")?;
		let stats = store
			.stats()
			.context("failed to collect store statistics")?;

		if let Ok(dir) = std::env::var("BKB_OVERRIDE_CONFIG_DIR") {
			println!("{}", style::bold("Environment"));
			Table::new()
				.kv_sub("BKB_OVERRIDE_CONFIG_DIR", dir)
				.blank()
				.print();
		}

		let config_dir = store_util::config_dir();
		let config_path = config_dir.join(CONFIG_FILENAME);

		println!("{}", style::bold("Config"));
		Table::new()
			.kv_sub("directory", config_dir.display().to_string())
			.kv_sub(
				CONFIG_FILENAME,
				if config_path.is_file() {
					style::green("present")
				} else {
					style::red("missing")
				},
			)
			.blank()
			.print();

		let data_dir = store.store_dir();
		println!("{}", style::bold("Data"));
		Table::new()
			.kv_sub("directory", data_dir.display().to_string())
			.kv_sub(
				"database",
				data_dir.join(Backbeat::DB_FILENAME).display().to_string(),
			)
			.kv_sub(
				"assets",
				data_dir.join(Backbeat::ASSETS_DIR).display().to_string(),
			)
			.blank()
			.print();

		if config_dir == data_dir {
			println!("Note: config and data share the same directory.");
			println!();
		}

		print_store_stats(&stats);

		let cfg = store.config();
		println!("{}", style::bold("Settings"));
		Table::new()
			.kv_sub("store.inline", cfg.store.inline.to_string())
			.kv_sub(
				"downloads.concurrency",
				cfg.downloads.concurrency.to_string(),
			)
			.blank()
			.print();

		let server_count = cfg.servers.len();
		println!("{} ({server_count} configured)", style::bold("Servers"));
		for server in &cfg.servers {
			println!("  {}", server.url);
		}
		println!();

		if let Some(info) = &cfg.info {
			println!("{}", style::bold("Server identity"));
			let mut t = Table::new();
			t.kv_sub("name", info.name.clone());
			if let Some(contact) = &info.contact {
				t.kv_sub("contact", contact.clone());
			}
			t.blank().print();
		}

		Ok(())
	}
}

fn print_store_stats(stats: &backbeat_sdk::store::stats::StoreStats) {
	println!("{}", style::bold("Store"));
	let mut t = crate::fmt::Table::new();
	t.row("Charts:", crate::fmt::count(stats.charts))
		.blank()
		.section("Collections:")
		.sub("Tables:", crate::fmt::count(stats.tables))
		.sub("Courses:", crate::fmt::count(stats.courses))
		.sub("Packs:", crate::fmt::count(stats.packs))
		.blank()
		.row("Assets:", crate::fmt::count(stats.asset_count))
		.blank()
		.section("Storage:")
		.sub("Assets:", crate::fmt::bytes(stats.asset_bytes))
		.sub("Database:", crate::fmt::bytes(stats.db_bytes))
		.sub("Total:", crate::fmt::bytes(stats.total_bytes()));

	t.blank().print();
}
