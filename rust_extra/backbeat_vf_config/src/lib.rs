#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![forbid(unsafe_code)]

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FOLDER_NAME: &str = "backbeat-vf";
pub const CONFIG_FILENAME: &str = "backbeat-vf.toml";

pub fn default_config_dir() -> PathBuf {
	let dirs = directories::BaseDirs::new().unwrap_or_else(|| {
		panic!(
			"Unknown system! We have no idea where your config dir should go. This should never happen; we support Linux+MacOS+Windows."
		)
	});

	#[cfg(target_os = "macos")]
	{
		dirs.home_dir().join(".config").join(CONFIG_FOLDER_NAME)
	}

	#[cfg(not(target_os = "macos"))]
	{
		dirs.config_dir().join(CONFIG_FOLDER_NAME)
	}
}

pub fn default_config_path() -> PathBuf {
	default_config_dir().join(CONFIG_FILENAME)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct VfConfig {
	#[serde(default, skip_serializing_if = "MountsConfig::is_empty")]
	pub mounts: MountsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MountsConfig {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bms_path: Option<PathBuf>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub kshoot_path: Option<PathBuf>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub stepmania_path: Option<PathBuf>,

	#[serde(default)]
	pub bms_enabled: bool,

	#[serde(default)]
	pub kshoot_enabled: bool,

	#[serde(default)]
	pub stepmania_enabled: bool,
}

impl Default for MountsConfig {
	fn default() -> Self {
		let dirs = directories::BaseDirs::new().unwrap_or_else(|| {
			panic!(
				"Unknown system! We have no idea where your home directory should go. This should never happen; we support Linux+MacOS+Windows."
			)
		});
		let root = dirs.home_dir().join("backbeat-vf");
		Self {
			bms_path: Some(root.join("bms")),
			kshoot_path: Some(root.join("kshoot")),
			stepmania_path: Some(root.join("stepmania")),
			bms_enabled: false,
			kshoot_enabled: false,
			stepmania_enabled: false,
		}
	}
}

impl MountsConfig {
	fn is_empty(&self) -> bool {
		self.bms_path.is_none()
			&& self.kshoot_path.is_none()
			&& self.stepmania_path.is_none()
			&& !self.bms_enabled
			&& !self.kshoot_enabled
			&& !self.stepmania_enabled
	}
}

impl VfConfig {
	pub fn load() -> Result<Self, ConfigError> {
		Self::load_with_overridden_dir(&default_config_dir())
	}

	pub fn load_with_overridden_dir(config_dir: &Path) -> Result<Self, ConfigError> {
		let path = config_dir.join(CONFIG_FILENAME);
		if !path.is_file() {
			let default = Self::default();
			default.write_to_dir(config_dir)?;
			return Ok(default);
		}

		let contents = fs_err::read_to_string(path)?;
		Ok(toml::from_str(&contents)?)
	}

	pub fn write_to_dir(&self, dir: &Path) -> Result<PathBuf, ConfigError> {
		if dir.ends_with(CONFIG_FILENAME) {
			panic!(
				"Tried to write_to_dir with a path name, not a filename. Refusing to create a **FOLDER** called backbeat-vf.toml"
			)
		}

		fs_err::create_dir_all(dir)?;
		let path = dir.join(CONFIG_FILENAME);
		fs_err::write(&path, self.to_toml()?)?;
		Ok(path)
	}

	pub fn to_toml(&self) -> Result<String, ConfigError> {
		Ok(toml::to_string_pretty(self)?)
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("failed to read config file: {0}")]
	Io(#[from] io::Error),

	#[error("failed to parse config file: {0}")]
	Toml(#[from] toml::de::Error),

	#[error("failed to serialize config: {0}")]
	TomlSer(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_config_path_is_under_the_vf_directory() {
		assert_eq!(
			default_config_path(),
			default_config_dir().join(CONFIG_FILENAME)
		);
		assert!(default_config_dir().ends_with(CONFIG_FOLDER_NAME));
	}

	#[test]
	fn load_creates_the_default_config_file() {
		let temp = tempfile::TempDir::new().unwrap();
		let config_dir = temp.path().join(CONFIG_FOLDER_NAME);

		assert_eq!(
			VfConfig::load_with_overridden_dir(&config_dir).unwrap(),
			VfConfig::default()
		);
		assert!(config_dir.join(CONFIG_FILENAME).is_file());
	}

	#[test]
	fn default_mount_paths_are_under_the_user_home_directory() {
		let dirs = directories::BaseDirs::new().unwrap();
		let root = dirs.home_dir().join("backbeat-vf");
		let mounts = MountsConfig::default();

		assert_eq!(mounts.bms_path, Some(root.join("bms")));
		assert_eq!(mounts.kshoot_path, Some(root.join("kshoot")));
		assert_eq!(mounts.stepmania_path, Some(root.join("stepmania")));
		assert!(!mounts.bms_enabled);
		assert!(!mounts.kshoot_enabled);
		assert!(!mounts.stepmania_enabled);
	}

	#[test]
	fn mount_paths_roundtrip() {
		let config = VfConfig {
			mounts: MountsConfig {
				bms_path: Some(PathBuf::from("/games/bms")),
				bms_enabled: true,
				..MountsConfig::default()
			},
		};

		let toml = config.to_toml().unwrap();
		assert!(toml.contains("[mounts]"));
		assert_eq!(toml::from_str::<VfConfig>(&toml).unwrap(), config);
	}
}
