use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use backbeat_core::{AssetId, AssetPath, BackbeatFile, BbZipWriter};

use super::Backbeat;
use crate::{AssetData, Result};

pub(crate) fn write(store: &Backbeat, bb: &BackbeatFile, output: &Path, bbzip: bool) -> Result<()> {
	if bbzip {
		write_bbzip(store, bb, output)
	} else {
		write_folder(store, bb, output)
	}
}

fn write_folder(store: &Backbeat, bb: &BackbeatFile, output: &Path) -> Result<()> {
	let depth = jail_depth(bb);
	let mut chart_dir = output.to_owned();
	for _ in 0..depth {
		chart_dir.push("_x");
	}
	crate::fs::create_dir_all(&chart_dir)?;

	for (path, id) in &bb.assets {
		let destination = asset_destination(output, depth, path);
		write_asset(store, *id, &destination)?;
	}

	crate::fs::write(chart_dir.join(bb.filename.as_path()), bb.chart.decompress())?;
	Ok(())
}

fn write_bbzip(store: &Backbeat, bb: &BackbeatFile, output: &Path) -> Result<()> {
	if let Some(parent) = output
		.parent()
		.filter(|parent| !parent.as_os_str().is_empty())
	{
		crate::fs::create_dir_all(parent)?;
	}
	let file = crate::fs::File::create(output)?;
	let mut archive = BbZipWriter::new(file);
	let json = bb.to_json();
	archive.add_file("manifest.bb", json.as_slice())?;

	let mut seen = HashSet::new();
	for id in bb.assets.values().copied().filter(|id| seen.insert(*id)) {
		match store.get_asset(id)? {
			AssetData::Bytes(bytes) => archive.add_file(id.to_string(), bytes.as_slice())?,
			AssetData::File(path) => {
				let mut file = crate::fs::File::open(path)?;
				archive.add_file(id.to_string(), &mut file)?;
			}
		}
	}

	archive.finish()?;
	Ok(())
}

fn write_asset(store: &Backbeat, id: AssetId, output: &Path) -> Result<()> {
	if let Some(parent) = output.parent() {
		crate::fs::create_dir_all(parent)?;
	}

	match store.get_asset(id)? {
		AssetData::Bytes(bytes) => crate::fs::write(output, bytes)?,
		AssetData::File(path) => {
			crate::fs::copy(path, output)?;
		}
	}
	Ok(())
}

fn jail_depth(bb: &BackbeatFile) -> usize {
	bb.assets
		.keys()
		.map(|path| {
			let mut depth = 0isize;
			let mut max_up = 0usize;
			for component in path.as_path().components() {
				match component {
					Component::Normal(_) => depth += 1,
					Component::ParentDir => {
						depth -= 1;
						max_up = max_up.max((-depth).max(0).cast_unsigned());
					}
					Component::CurDir => {}
					Component::RootDir | Component::Prefix(_) => unreachable!(),
				}
			}
			max_up
		})
		.max()
		.unwrap_or(0)
}

fn asset_destination(output: &Path, depth: usize, asset: &AssetPath) -> PathBuf {
	let mut relative = PathBuf::new();
	for _ in 0..depth {
		relative.push("_x");
	}
	for component in asset.as_path().components() {
		match component {
			Component::Normal(part) => relative.push(part),
			Component::ParentDir => {
				assert!(relative.pop(), "jail depth must contain asset path");
			}
			Component::CurDir => {}
			Component::RootDir | Component::Prefix(_) => unreachable!(),
		}
	}
	output.join(relative)
}
