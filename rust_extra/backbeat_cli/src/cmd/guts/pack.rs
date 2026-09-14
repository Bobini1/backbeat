use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use backbeat_packager::SeenCache;
use clap::{Args, ValueEnum};

use super::output::resolve_output_dir;

/// Package a source chart file into one or more .bb files.
#[derive(Debug, Args)]
pub struct PackCommand {
	/// Path to the source chart file or directory.
	pub file: PathBuf,

	/// Recursively package every supported chart below the given directory.
	///
	/// Recursive packaging requires `--format jsonl` so callers receive the
	/// source and asset directory for every emitted bundle.
	#[arg(short, long)]
	pub recursive: bool,

	/// Output format. `jsonl` writes one machine-readable bundle record per line.
	#[arg(long, value_enum)]
	pub format: Option<PackFormat>,

	/// Directory to write `.bb` files into. Defaults to the source file's directory.
	///
	/// Ignored when `--format jsonl` is set (records go to stdout).
	#[arg(short = 'o', long = "output-dir")]
	pub output_dir: Option<PathBuf>,

	/// Fail if any referenced dependency is missing from the filesystem.
	#[arg(long)]
	pub strict: bool,

	/// Write one `.bbzip` archive containing the packaged charts and their assets.
	#[arg(short = 'z', long)]
	pub bbzip: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum PackFormat {
	Jsonl,
}

impl PackCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let cache = SeenCache::new();
		anyhow::ensure!(
			!(self.bbzip && self.recursive),
			"--bbzip cannot be used with --recursive"
		);
		if self.recursive {
			anyhow::ensure!(
				self.format == Some(PackFormat::Jsonl),
				"--recursive requires --format jsonl"
			);
			anyhow::ensure!(
				self.output_dir.is_none(),
				"--output-dir cannot be used with --recursive --format jsonl"
			);
			return package_recursive_jsonl(&self.file, &cache, self.strict);
		}

		let bbs = backbeat_packager::package_with_cache(&self.file, &cache)
			.with_context(|| format!("failed to package {:?}", self.file))?;
		if self.strict {
			super::require_no_missing_dependencies(cache.missing_asset_paths())?;
		}
		if self.format == Some(PackFormat::Jsonl) {
			anyhow::ensure!(!self.bbzip, "--bbzip cannot be used with --format jsonl");
			anyhow::ensure!(
				self.output_dir.is_none(),
				"--output-dir cannot be used with --format jsonl"
			);
			return write_jsonl_records(&self.file, &bbs);
		}

		let out_dir = resolve_output_dir(&self.file, self.output_dir.as_deref())?;
		let stem = self
			.file
			.file_stem()
			.map(|stem| stem.to_string_lossy())
			.unwrap_or_default();

		if self.bbzip {
			let path = out_dir.join(format!("{stem}.bbzip"));
			let output = fs_err::File::create(&path)
				.with_context(|| format!("failed to create {}", path.display()))?;
			backbeat_packager::write_bbzip(&self.file, &bbs, output)
				.with_context(|| format!("failed to write {}", path.display()))?;
			println!("wrote {}", path.display());
			return Ok(());
		}

		if bbs.len() == 1 {
			write_bb(out_dir.join(format!("{stem}.bb")), &bbs[0])?;
		} else {
			let width = bbs.len().to_string().len();
			for (index, bb) in bbs.iter().enumerate() {
				let filename = format!("{stem}.{:0width$}.bb", index + 1, width = width);
				write_bb(out_dir.join(filename), bb)?;
			}
		}

		Ok(())
	}
}

fn package_recursive_jsonl(root: &Path, cache: &SeenCache, strict: bool) -> anyhow::Result<()> {
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
			continue;
		}

		let bbs = backbeat_packager::package_with_cache(source, cache)
			.with_context(|| format!("failed to package {}", source.display()))?;
		if strict {
			super::require_no_missing_dependencies(cache.missing_asset_paths())
				.with_context(|| format!("failed to package {}", source.display()))?;
		}
		write_jsonl_records(source, &bbs)?;
	}

	Ok(())
}

fn write_jsonl_records(path: &Path, bbs: &[backbeat_core::BackbeatFile]) -> anyhow::Result<()> {
	let stdout = std::io::stdout();
	let mut out = stdout.lock();

	for bb in bbs {
		serde_json::to_writer(&mut out, &jsonl_record(path, bb)?)
			.context("failed to serialize package record")?;
		writeln!(out).context("failed to write package record")?;
	}

	Ok(())
}

fn jsonl_record(
	path: &Path,
	bb: &backbeat_core::BackbeatFile,
) -> anyhow::Result<serde_json::Value> {
	let source = path
		.canonicalize()
		.with_context(|| format!("failed to canonicalize {}", path.display()))?;
	let assets_dir = source
		.parent()
		.context("chart path has no parent directory")?
		.to_str()
		.context("chart asset directory is not UTF-8")?;
	let source = source.to_str().context("chart source path is not UTF-8")?;
	let mut ids = serde_json::Map::new();
	let chart_sha256 = bb.chart_sha256();

	ids.insert(
		"sha256".to_owned(),
		serde_json::Value::String(chart_sha256.to_string()),
	);
	for id in bb.additional_chart_ids() {
		ids.insert(id.alg.to_string(), serde_json::Value::String(id.val));
	}

	Ok(serde_json::json!({
		"source": source,
		"assetsDir": assets_dir,
		"bundle": bb,
		"ids": ids,
	}))
}

fn write_bb(path: PathBuf, bb: &backbeat_core::BackbeatFile) -> anyhow::Result<()> {
	let json = bb.to_json();

	fs_err::write(&path, json).with_context(|| format!("failed to write {path:?}"))?;

	println!("wrote {}", path.display());

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn jsonl_record_includes_absolute_source_and_asset_directory() {
		let tmp = tempfile::TempDir::new().unwrap();
		let path = tmp.path().join("song.bms");
		fs_err::write(&path, b"#TITLE Test;\n").unwrap();
		let bb = backbeat_packager::package(&path)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();

		let record = jsonl_record(&path, &bb).unwrap();
		assert_eq!(
			record["source"],
			path.canonicalize().unwrap().to_str().unwrap()
		);
		assert_eq!(
			record["assetsDir"],
			tmp.path().canonicalize().unwrap().to_str().unwrap()
		);
		assert_eq!(record["bundle"]["filename"], "song.bms");
		assert!(
			record["bundle"]["desc"]
				.as_str()
				.is_some_and(|description| !description.is_empty())
		);
		assert!(record["ids"]["sha256"].is_string());
		assert!(record["ids"]["md5"].is_string());
	}
}
