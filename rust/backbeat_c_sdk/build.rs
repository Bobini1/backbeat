use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(crate_dir: &Path, args: &[&str]) -> Option<String> {
	let output = Command::new("git")
		.args(args)
		.current_dir(crate_dir)
		.output()
		.ok()?;
	if !output.status.success() {
		return None;
	}

	String::from_utf8(output.stdout)
		.ok()
		.map(|value| value.trim().to_owned())
		.filter(|value| !value.is_empty())
}

fn track_git_revision(crate_dir: &Path) {
	for path in [
		git_output(crate_dir, &["rev-parse", "--git-path", "HEAD"]),
		git_output(crate_dir, &["rev-parse", "--git-path", "packed-refs"]),
		git_output(crate_dir, &["symbolic-ref", "--quiet", "HEAD"])
			.and_then(|reference| git_output(crate_dir, &["rev-parse", "--git-path", &reference])),
	]
	.into_iter()
	.flatten()
	{
		println!("cargo:rerun-if-changed={path}");
	}
}

fn escape_c_string(value: &str) -> String {
	value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
	let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
	let header_path = out_dir.join("backbeat.h");

	println!("cargo:rerun-if-changed=src/");
	println!("cargo:rerun-if-changed=cbindgen.toml");
	println!("cargo:rerun-if-env-changed=BKB_LIBCOMMIT_HASH");
	println!("cargo:rerun-if-env-changed=BKB_HEADER_PATH");
	track_git_revision(&crate_dir);

	let mut config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
		.expect("failed to load cbindgen.toml");
	let commit_hash = env::var("BKB_LIBCOMMIT_HASH")
		.ok()
		.or_else(|| git_output(&crate_dir, &["rev-parse", "HEAD"]))
		.unwrap_or_else(|| "unknown".to_owned());
	assert!(
		!commit_hash
			.bytes()
			.any(|byte| matches!(byte, 0 | b'\n' | b'\r')),
		"BKB_LIBCOMMIT_HASH contains a forbidden control character"
	);
	let commit_hash_for_c = escape_c_string(&commit_hash);
	config.after_includes = Some(format!(
		"/* Git commit used to build the library. */\n#define BKB_LIBCOMMIT_HASH \"{commit_hash_for_c}\""
	));
	println!("cargo:rustc-env=BKB_LIBCOMMIT_HASH={commit_hash}");

	cbindgen::Builder::new()
		.with_crate(crate_dir)
		.with_config(config)
		.generate()
		.expect("cbindgen failed")
		.write_to_file(&header_path);

	if let Some(export_path) = env::var_os("BKB_HEADER_PATH").map(PathBuf::from) {
		let parent = export_path
			.parent()
			.expect("BKB_HEADER_PATH must have a parent directory");
		std::fs::create_dir_all(parent).expect("failed to create header output directory");
		std::fs::copy(&header_path, export_path).expect("failed to export generated header");
	}
}
