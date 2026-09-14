//! Helpers for locating the global Backbeat store.
//!
//! There is one global store per user. The config file lives in the platform
//! config dir ([`backbeat_store_config::default_config_dir`]); the data dir is named by
//! `[store].path` inside the config. Opening the store auto-initialises both on
//! first run. There is no `--store` flag.

use std::path::PathBuf;

use anyhow::Context;
use backbeat_sdk::Backbeat;
use backbeat_store_config::{BackbeatConfig, default_config_dir};

fn override_config_dir() -> Option<PathBuf> {
	std::env::var("BKB_OVERRIDE_CONFIG_DIR")
		.ok()
		.map(PathBuf::from)
}

/// The global config directory (where `backbeat.toml` lives).
pub fn config_dir() -> PathBuf {
	if let Some(dir) = override_config_dir() {
		return dir;
	}
	default_config_dir()
}

/// Open the default global store, auto-initialising on first run.
pub fn open() -> anyhow::Result<Backbeat> {
	if let Some(dir) = override_config_dir() {
		return Backbeat::open_with_overridden_config_dir(dir.as_path())
			.context("failed to open Backbeat store");
	}

	Backbeat::open().context("failed to open Backbeat store")
}

/// Load config from the global config directory without opening the database.
///
/// Auto-creates a default `backbeat.toml` in the config dir if absent, so
/// `bkb config` works on a fresh install.
pub fn load_config() -> anyhow::Result<BackbeatConfig> {
	BackbeatConfig::load_with_overridden_dir(&config_dir()).context("failed to load config")
}
