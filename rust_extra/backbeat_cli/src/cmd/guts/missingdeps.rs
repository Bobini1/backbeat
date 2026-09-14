use std::path::{Path, PathBuf};

use anyhow::Context;
use backbeat_packager::SeenCache;
use clap::Args;

/// List referenced assets that do not exist on disk.
#[derive(Debug, Args)]
pub struct MissingDepsCommand {
	/// Path to the source chart file or directory.
	pub file: PathBuf,

	/// Recursively inspect every supported chart below the given directory.
	#[arg(short, long)]
	pub recursive: bool,
}

impl MissingDepsCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let cache = SeenCache::new();

		if self.recursive {
			return missing_deps_recursive(&self.file, &cache);
		}

		print_missing_deps(&self.file, &cache)
			.with_context(|| format!("failed to package {}", self.file.display()))
	}
}

fn missing_deps_recursive(root: &Path, cache: &SeenCache) -> anyhow::Result<()> {
	anyhow::ensure!(
		root.is_dir(),
		"--recursive requires a directory, got: {}",
		root.display()
	);

	for entry in walkdir::WalkDir::new(root)
		.follow_links(false)
		.sort_by_file_name()
		.into_iter()
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().is_file())
	{
		let source = entry.path();
		if !backbeat_packager::should_be_packaged(source) {
			eprintln!("Unrecognised chart {}, can't check deps", source.display());
			continue;
		}

		if let Err(error) = print_missing_deps(source, cache) {
			eprintln!("warning: failed to package {}: {error}", source.display());
		}
	}

	Ok(())
}

fn print_missing_deps(source: &Path, cache: &SeenCache) -> anyhow::Result<()> {
	let chart_cache = cache.with_isolated_missing_assets();
	backbeat_packager::package_with_cache(source, &chart_cache)?;

	let mut missing = chart_cache.missing_asset_paths();
	missing.sort_unstable();

	for asset_path in missing {
		println!("{}\t{asset_path}", source.display());
	}

	Ok(())
}
