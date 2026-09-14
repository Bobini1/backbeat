//! Test fixtures live in the repo-root `fixtures/` directory.
//!
//! Use [`fixture_bytes!`] to embed fixture bytes at compile time.

/// Load a file from the repo-root `fixtures/` directory at compile time.
macro_rules! fixture_bytes {
	($path:literal) => {{
		include_bytes!(concat!(
			::core::env!("CARGO_MANIFEST_DIR"),
			"/../../fixtures/",
			$path
		))
	}};
}
