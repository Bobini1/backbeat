use std::{
	ffi::OsStr,
	path::{Path, PathBuf},
};

use anyhow::Context as _;
use backbeat_core::BackbeatFile;
use backbeat_sdk::Backbeat;
use clap::Args;

use crate::{fmt, store_util, style};

/// Add `.bb` and `.bbzip` files to your store.
#[derive(Debug, Args)]
pub struct ImportCommand {
	/// Path to a `.bb` or `.bbzip` file to add. Alternatively, pass a directory to add everything inside it (with -r passed).
	pub path: PathBuf,

	/// Recursively add all `.bb` files found under the given directory.
	#[arg(short, long)]
	pub recursive: bool,

	/// Delete files after adding them to your store.
	#[arg(short, long)]
	pub delete: bool,
}

impl ImportCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;

		if self.recursive {
			self.add_recursive(&store)
		} else {
			let r = add_one(&store, &self.path);
			if self.delete {
				let _ = std::fs::remove_file(&self.path);
			}

			r
		}
	}

	fn add_recursive(&self, store: &Backbeat) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.path.is_dir(),
			"--recursive requires a directory, got: {}",
			self.path.display()
		);

		let spinner = style::new_spinner(format!("Scanning {}…", self.path.display()));
		let mut total = 0usize;
		let mut errors = 0usize;

		for entry in walkdir::WalkDir::new(&self.path)
			.follow_links(true)
			.into_iter()
			.filter_map(|entry| entry.ok())
			.filter(|entry| entry.file_type().is_file())
		{
			let path = entry.path();

			if !is_relevant_extension(path) {
				continue;
			}

			match add_one(store, path) {
				Ok(()) => {
					total += 1;
					spinner.set_message(format!("Adding… {}", fmt::count(total as u64)));

					if self.delete {
						let _ = std::fs::remove_file(path);
					}
				}
				Err(err) => {
					spinner.println(format!(
						"  {} skipped {}: {err:#}",
						style::warn_label(),
						path.display()
					));
					errors += 1;
				}
			}
		}

		spinner.finish_and_clear();
		println!(
			"{} Added {} bundle{}, {} skipped",
			style::check_mark(),
			fmt::count(total as u64),
			if total == 1 { "" } else { "s" },
			fmt::count(errors as u64),
		);

		if errors > 0 && total == 0 {
			anyhow::bail!("all files failed to add");
		}

		Ok(())
	}
}

fn is_relevant_extension(path: &Path) -> bool {
	matches!(
		path.extension()
			.and_then(OsStr::to_str)
			.map(|e| e.to_ascii_lowercase())
			.as_deref(),
		Some("bb" | "bbzip"),
	)
}

fn add_one(store: &Backbeat, path: &Path) -> anyhow::Result<()> {
	match path
		.extension()
		.and_then(OsStr::to_str)
		.map(|e| e.to_ascii_lowercase())
		.as_deref()
	{
		Some("bb") => {
			let bb = BackbeatFile::from_file(path)
				.with_context(|| format!("failed to read .bb file {}", path.display()))?;
			store.import_bundle(&bb)?;
		}
		Some("bbzip") => {
			store
				.import_bbzip(path)
				.with_context(|| format!("failed to add {}", path.display()))?;
		}
		_ => {
			anyhow::bail!("{} is not a `.bb` or `.bbzip` file!", path.display());
		}
	}

	println!("{} added {}", style::check_mark(), path.display());
	Ok(())
}
