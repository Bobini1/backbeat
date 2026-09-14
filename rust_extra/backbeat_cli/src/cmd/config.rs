use anyhow::Context as _;
use backbeat_store_config::{BackbeatConfig, ByteSize, CONFIG_FILENAME};
use clap::{Args, Subcommand};

use crate::store_util;

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
	/// Print the current value of a configuration key.
	Get(GetCommand),

	/// Update a configuration key in `backbeat.toml`.
	Set(SetCommand),

	/// Print the contents of `backbeat.toml` to stdout.
	Read,

	/// Print the path to `backbeat.toml`.
	Locate,
}

impl ConfigCommand {
	pub fn run(self) -> anyhow::Result<()> {
		match self {
			Self::Get(cmd) => cmd.run(),
			Self::Set(cmd) => cmd.run(),
			Self::Read => read_config(),
			Self::Locate => locate_config(),
		}
	}
}

fn read_config() -> anyhow::Result<()> {
	let path = store_util::config_dir().join(CONFIG_FILENAME);
	let contents = std::fs::read_to_string(&path)
		.with_context(|| format!("failed to read config file {}", path.display()))?;
	print!("{contents}");
	Ok(())
}

fn locate_config() -> anyhow::Result<()> {
	let path = store_util::config_dir().join(CONFIG_FILENAME);
	println!("{}", path.display());
	Ok(())
}

#[derive(Debug, Args)]
pub struct GetCommand {
	/// Configuration key to read.
	pub key: ConfigKey,
}

impl GetCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let config = store_util::load_config()?;

		let value = self.key.get(&config)?;
		println!("{value}");
		Ok(())
	}
}

#[derive(Debug, Args)]
pub struct SetCommand {
	/// Configuration key to update.
	pub key: ConfigKey,

	/// New value for the key.
	pub value: String,
}

impl SetCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let dir = store_util::config_dir();
		let mut config = store_util::load_config()?;

		self.key
			.set(&mut config, &self.value)
			.with_context(|| format!("invalid value for {}", self.key))?;

		config
			.write_to_dir(&dir)
			.with_context(|| format!("failed to write {}", dir.join(CONFIG_FILENAME).display()))?;

		println!("{} = {}", self.key, self.value);
		Ok(())
	}
}

/// Keys understood by `get` and `set`.
const VALID_KEYS: &str = "store.path, store.inline, downloads.concurrency, downloads.stream";

/// A validated config key, parsed from a string by clap.
#[derive(Debug, Clone)]
pub enum ConfigKey {
	StorePath,
	StoreInline,
	DownloadsConcurrency,
	DownloadsStream,
}

impl ConfigKey {
	fn get(&self, config: &BackbeatConfig) -> anyhow::Result<String> {
		match self {
			Self::StorePath => Ok(config.store.path.display().to_string()),
			Self::StoreInline => Ok(config.store.inline.to_string()),
			Self::DownloadsConcurrency => Ok(config.downloads.concurrency.to_string()),
			Self::DownloadsStream => Ok(config.downloads.stream.to_string()),
		}
	}

	fn set(&self, config: &mut BackbeatConfig, raw: &str) -> anyhow::Result<()> {
		match self {
			Self::StorePath => {
				config.store.path = std::path::PathBuf::from(raw);
			}
			Self::StoreInline => {
				config.store.inline = ByteSize::parse(raw).with_context(|| {
					format!(
						"store.inline must be a k8s-style byte quantity (e.g. \"16Ki\", \"10k\"), got {raw:?}"
					)
				})?;
			}
			Self::DownloadsConcurrency => {
				let n: u32 = raw.parse().with_context(|| {
					format!("downloads.concurrency must be a positive integer, got {raw:?}")
				})?;
				config.downloads.concurrency = n;
			}
			Self::DownloadsStream => {
				config.downloads.stream = ByteSize::parse(raw).with_context(|| {
					format!(
						"downloads.stream must be a k8s-style byte quantity (e.g. \"8Mi\", \"10k\"), got {raw:?}"
					)
				})?;
			}
		}
		Ok(())
	}
}

impl std::fmt::Display for ConfigKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::StorePath => f.write_str("store.path"),
			Self::StoreInline => f.write_str("store.inline"),
			Self::DownloadsConcurrency => f.write_str("downloads.concurrency"),
			Self::DownloadsStream => f.write_str("downloads.stream"),
		}
	}
}

impl std::str::FromStr for ConfigKey {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"store.path" => Ok(Self::StorePath),
			"store.inline" => Ok(Self::StoreInline),
			"downloads.concurrency" => Ok(Self::DownloadsConcurrency),
			"downloads.stream" => Ok(Self::DownloadsStream),
			other => anyhow::bail!("unknown config key {other:?}; valid keys: {VALID_KEYS}"),
		}
	}
}
