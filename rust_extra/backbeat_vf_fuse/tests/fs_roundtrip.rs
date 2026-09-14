#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! End-to-end FUSE smoke test via [`scripts/fuse-test.sh`].
//!
//! Skips when FUSE is unavailable. For manual debugging, run:
//!   ./scripts/fuse-test.sh -- sh -c 'ls -la "$BACKBEAT_FUSE_MOUNT"'

use std::path::PathBuf;
use std::process::Command;

use backbeat_vf_fuse::fuse_available;

fn repo_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
#[ignore = "FUSE mount integration test is broken in CI/sandbox environments"]
fn fuse_mount_serves_files() {
	if !fuse_available() {
		eprintln!("SKIP fuse_mount_serves_files: FUSE not available on this system");
		return;
	}

	let repo = repo_root();
	let script = repo.join("scripts/fuse-test.sh");
	assert!(
		script.is_file(),
		"fuse-test.sh not found at {}",
		script.display()
	);

	let output = Command::new("bash")
		.arg(&script)
		.arg("--")
		.arg("sh")
		.arg("-c")
		.arg(
			r#"set -euo pipefail
ls -la "$BACKBEAT_FUSE_MOUNT"
file=$(find "$BACKBEAT_FUSE_MOUNT" -type f | head -1)
test -n "$file"
test -s "$file"
echo "read $(wc -c < "$file") bytes from $file""#,
		)
		.current_dir(&repo)
		.output()
		.unwrap_or_else(|err| panic!("run {}: {err}", script.display()));

	if !output.status.success() {
		panic!(
			"fuse-test.sh failed (exit {})\nstdout:\n{}\nstderr:\n{}",
			output.status.code().unwrap_or(-1),
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
		);
	}
}
