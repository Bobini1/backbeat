#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unreachable_pub)]
#![doc = include_str!("../../README.md")]

fn main() {
	backbeat_app::run();
}
