use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let store_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rust/backbeat_sdk");
	let database = Path::new(&env::var("OUT_DIR")?).join("template.backbeat.db");
	let _ = fs::remove_file(&database);
	let database_url = format!("sqlite:///{}", database.display());

	let status = Command::new("cargo")
		.current_dir(&store_dir)
		.args(["sqlx", "database", "setup", "-D", &database_url])
		.status()?;
	if !status.success() {
		panic!("failed to build backbeat_vf_fs template database {status:?}");
	}

	println!("cargo:rustc-env=DATABASE_URL={database_url}");
	println!(
		"cargo:rerun-if-changed={}",
		store_dir.join("migrations").display()
	);

	Ok(())
}
