use backbeat_sdk::Backbeat;
use backbeat_store_config::BackbeatConfig;
use tempfile::TempDir;

// Duplicated from Backbeat's test utilities because integration tests do not compile dependencies with `cfg(test)`.
pub fn new_test_store(tmpdir_name: &str) -> (TempDir, Backbeat) {
	new_test_store_with(tmpdir_name, BackbeatConfig::default())
}

pub fn new_test_store_with(tmpdir_name: &str, mut config: BackbeatConfig) -> (TempDir, Backbeat) {
	let temp = tempfile::Builder::new()
		.prefix(tmpdir_name)
		.tempdir()
		.expect("create test store temp directory");
	let config_dir = temp.path().join("config");
	config.store.path = temp.path().join("data");
	config
		.write_to_dir(&config_dir)
		.expect("write test store config");
	let store = Backbeat::open_with_overridden_config_dir(&config_dir)
		.expect("open test store from temporary config");
	(temp, store)
}
