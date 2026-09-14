use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use vergen_git2::{Emitter, Git2Builder};

fn main() {
	let store_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../backbeat_sdk");
	let database = Path::new(&env::var("OUT_DIR").expect("OUT_DIR should be set"))
		.join("template.backbeat.db");
	let _ = fs::remove_file(&database);
	let database_url = format!("sqlite:///{}", database.display());

	let status = Command::new("cargo")
		.current_dir(&store_dir)
		.args(["sqlx", "database", "setup", "-D", &database_url])
		.status()
		.expect("run cargo sqlx database setup");
	assert!(
		status.success(),
		"failed to build backbeat_app template database"
	);

	println!("cargo:rustc-env=DATABASE_URL={database_url}");
	println!(
		"cargo:rerun-if-changed={}",
		store_dir.join("migrations").display()
	);

	let git2 = Git2Builder::default()
		.sha(true)
		.build()
		.expect("vergen git2");
	Emitter::default()
		.add_instructions(&git2)
		.expect("vergen emitter")
		.emit()
		.expect("vergen emit");

	tauri_build::build();
}
