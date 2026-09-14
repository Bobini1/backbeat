use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let target_path = Path::new(&env::var("OUT_DIR")?).join("template.backbeat.db");

	let _ = fs::remove_file(&target_path);

	let db_url = format!("sqlite:///{}", target_path.display());

	let status = Command::new("cargo")
		.args(["sqlx", "database", "setup", "-D", &db_url])
		.status()?;

	assert!(status.success(), "failed to build template db {status:?}");

	// Compile-time `sqlx::query!` checks use this URL (no `.env` required).
	println!("cargo:rustc-env=DATABASE_URL={db_url}");

	// Tell Cargo that if the given file changes, to rerun this build script.
	println!("cargo:rerun-if-changed=migrations");

	Ok(())
}
