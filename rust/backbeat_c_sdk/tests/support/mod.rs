use std::ffi::OsString;

use backbeat_sdk::Backbeat;
use backbeat_store_config::{BackbeatConfig, ByteSize, default_config_dir};
use tempfile::TempDir;

pub struct Environment {
	values: Vec<(&'static str, Option<OsString>)>,
}

impl Environment {
	fn set(temp: &TempDir) -> Self {
		let values = ["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME"]
			.into_iter()
			.map(|name| (name, std::env::var_os(name)))
			.collect();
		unsafe {
			std::env::set_var("HOME", temp.path().join("home"));
			std::env::set_var("XDG_CONFIG_HOME", temp.path().join("config"));
			std::env::set_var("XDG_DATA_HOME", temp.path().join("data-home"));
		}
		Self { values }
	}
}

impl Drop for Environment {
	fn drop(&mut self) {
		for (name, value) in self.values.drain(..) {
			unsafe {
				match value {
					Some(value) => std::env::set_var(name, value),
					None => std::env::remove_var(name),
				}
			}
		}
	}
}

pub fn new_test_store(tmpdir_name: &str, inline: u64) -> (TempDir, Environment, Backbeat) {
	let temp = tempfile::Builder::new()
		.prefix(tmpdir_name)
		.tempdir()
		.expect("create test store temp directory");
	let environment = Environment::set(&temp);
	let mut config = BackbeatConfig::default();
	config.store.path = temp.path().join("data");
	config.store.inline = ByteSize(inline);
	config
		.write_to_dir(&default_config_dir())
		.expect("write test store config");
	let store = Backbeat::open().expect("open test store from temporary config");
	(temp, environment, store)
}
