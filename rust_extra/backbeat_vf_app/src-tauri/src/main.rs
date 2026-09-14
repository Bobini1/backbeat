#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unreachable_pub)]

fn main() {
	if std::env::args().nth(1).as_deref() == Some("--mount-daemon") {
		std::process::exit(backbeat_vf_mountd::run_process());
	}
	backbeat_vf_app::run();
}
