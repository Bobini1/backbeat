#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use backbeat_sdk::Backbeat;
use backbeat_store_config::CONFIG_FILENAME;

mod support;
use support::new_test_store;

#[test]
fn temporary_store_uses_separate_config_and_data_directories() {
	let (temp, store) = new_test_store("bb_create");

	assert_eq!(store.config_dir(), temp.path().join("config"));
	assert_eq!(store.store_dir(), temp.path().join("data"));
	assert!(store.config_dir().join(CONFIG_FILENAME).is_file());
	assert!(store.store_dir().join(Backbeat::DB_FILENAME).is_file());
}

#[test]
fn temporary_store_can_be_reopened() {
	let (temp, store) = new_test_store("bb_open_roundtrip");
	drop(store);

	let store = Backbeat::open_with_overridden_config_dir(temp.path().join("config"))
		.expect("reopen store");
	assert_eq!(store.store_dir(), temp.path().join("data"));
}
